//! Android XML emission for the normalized drawable model.
use crate::Error;
use crate::geometry::number;
use crate::model::{
    Drawable, FillRule, Gradient, GradientKind, LineCap, LineJoin, Node, Paint, Spread, VectorPath,
};

pub(crate) fn write(drawable: &Drawable) -> Result<String, Error> {
    for dimension in [
        drawable.size.width,
        drawable.size.height,
        drawable.viewport.width,
        drawable.viewport.height,
    ] {
        if number(dimension) == "0" {
            return Err(Error::Invalid(
                "dimensions are too small for six-decimal Android XML output".into(),
            ));
        }
    }
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<vector xmlns:android=\"http://schemas.android.com/apk/res/android\"\n",
    );
    if has_gradients(&drawable.nodes) {
        xml.push_str("    xmlns:aapt=\"http://schemas.android.com/aapt\"\n");
    }
    xml.push_str(&format!("    android:width=\"{}dp\"\n    android:height=\"{}dp\"\n    android:viewportWidth=\"{}\"\n    android:viewportHeight=\"{}\">\n",number(drawable.size.width),number(drawable.size.height),number(drawable.viewport.width),number(drawable.viewport.height)));
    write_nodes(&mut xml, &drawable.nodes, 1)?;
    xml.push_str("</vector>\n");
    Ok(xml)
}

fn has_gradients(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| match node {
        Node::Path(path) => {
            matches!(path.fill, Paint::Gradient(_)) || matches!(path.stroke, Paint::Gradient(_))
        }
        Node::ClipGroup { nodes, .. } => has_gradients(nodes),
    })
}

fn write_nodes(xml: &mut String, nodes: &[Node], depth: usize) -> Result<(), Error> {
    let indent = "  ".repeat(depth);
    for node in nodes {
        match node {
            Node::Path(path) => write_path(xml, path, &indent)?,
            Node::ClipGroup { clips, nodes } => {
                xml.push_str(&format!("{indent}<group>\n"));
                for clip in clips {
                    clip.data.validate()?;
                    xml.push_str(&format!(
                        "{indent}  <clip-path android:pathData=\"{}\" android:fillType=\"{}\"/>\n",
                        clip.data.to_android(),
                        fill_rule(clip.fill_rule)
                    ));
                }
                write_nodes(xml, nodes, depth + 1)?;
                xml.push_str(&format!("{indent}</group>\n"));
            }
        }
    }
    Ok(())
}

fn write_path(xml: &mut String, path: &VectorPath, indent: &str) -> Result<(), Error> {
    path.data.validate()?;
    xml.push_str(&format!(
        "{indent}<path\n{indent}    android:pathData=\"{}\"\n{indent}    android:fillType=\"{}\"",
        path.data.to_android(),
        fill_rule(path.fill_rule)
    ));
    write_paint_attribute(xml, &path.fill, &format!("{indent}    android:fill"));
    if !matches!(path.stroke, Paint::None) {
        write_paint_attribute(xml, &path.stroke, &format!("{indent}    android:stroke"));
        let cap = match path.line_cap {
            LineCap::Butt => "butt",
            LineCap::Round => "round",
            LineCap::Square => "square",
        };
        let join = match path.line_join {
            LineJoin::Miter => "miter",
            LineJoin::Round => "round",
            LineJoin::Bevel => "bevel",
        };
        if !path.stroke_width.is_finite() || !path.miter_limit.is_finite() {
            return Err(Error::Invalid("non-finite stroke dimensions".into()));
        }
        xml.push_str(&format!("\n{indent}    android:strokeWidth=\"{}\"\n{indent}    android:strokeLineCap=\"{cap}\"\n{indent}    android:strokeLineJoin=\"{join}\"\n{indent}    android:strokeMiterLimit=\"{}\"",number(path.stroke_width),number(path.miter_limit)));
    }
    if matches!(path.fill, Paint::Gradient(_)) || matches!(path.stroke, Paint::Gradient(_)) {
        xml.push_str(">\n");
        if let Paint::Gradient(gradient) = &path.fill {
            write_gradient(xml, gradient, &format!("{indent}  "), "fillColor")?;
        }
        if let Paint::Gradient(gradient) = &path.stroke {
            write_gradient(xml, gradient, &format!("{indent}  "), "strokeColor")?;
        }
        xml.push_str(&format!("{indent}</path>\n"));
    } else {
        xml.push_str("/>\n");
    }
    Ok(())
}

fn write_paint_attribute(xml: &mut String, paint: &Paint, prefix: &str) {
    match paint {
        Paint::None => xml.push_str(&format!("\n{prefix}Color=\"#00000000\"")),
        Paint::Solid { color, opacity } => {
            xml.push_str(&format!("\n{prefix}Color=\"{color}\""));
            if *opacity != 1.0 {
                xml.push_str(&format!("\n{prefix}Alpha=\"{}\"", number(*opacity)));
            }
        }
        Paint::Gradient(_) => {}
    }
}

fn write_gradient(
    xml: &mut String,
    gradient: &Gradient,
    indent: &str,
    usage: &str,
) -> Result<(), Error> {
    xml.push_str(&format!(
        "{indent}<aapt:attr name=\"android:{usage}\">\n{indent}  <gradient"
    ));
    let coordinates = match gradient.kind {
        GradientKind::Linear { start, end } => {
            xml.push_str(" android:type=\"linear\"");
            vec![
                ("startX", start.x),
                ("startY", start.y),
                ("endX", end.x),
                ("endY", end.y),
            ]
        }
        GradientKind::Radial { center, radius } => {
            xml.push_str(" android:type=\"radial\"");
            vec![
                ("centerX", center.x),
                ("centerY", center.y),
                ("gradientRadius", radius),
            ]
        }
    };
    for (name, value) in coordinates {
        if !value.is_finite() {
            return Err(Error::Invalid("non-finite gradient coordinates".into()));
        }
        xml.push_str(&format!(
            "\n{indent}      android:{name}=\"{}\"",
            number(value)
        ));
    }
    let spread = match gradient.spread {
        Spread::Clamp => "clamp",
        Spread::Repeat => "repeat",
        Spread::Mirror => "mirror",
    };
    xml.push_str(&format!("\n{indent}      android:tileMode=\"{spread}\">\n"));
    for stop in &gradient.stops {
        xml.push_str(&format!(
            "{indent}    <item android:offset=\"{}\" android:color=\"{}\"/>\n",
            number(stop.offset),
            stop.color
        ));
    }
    xml.push_str(&format!("{indent}  </gradient>\n{indent}</aapt:attr>\n"));
    Ok(())
}

const fn fill_rule(rule: FillRule) -> &'static str {
    match rule {
        FillRule::NonZero => "nonZero",
        FillRule::EvenOdd => "evenOdd",
    }
}
