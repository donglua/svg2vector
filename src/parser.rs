#[path = "parser_clip.rs"]
mod clip;
#[path = "parser_css.rs"]
pub(crate) mod css;
#[path = "dimensions.rs"]
mod dimensions;
#[path = "parser_properties.rs"]
mod parser_properties;
#[path = "parser_refs.rs"]
mod references;
#[path = "shapes.rs"]
mod shapes;
#[path = "styles.rs"]
mod styles;

use crate::geometry::Transform;
use crate::model::{Drawable, Node, VectorPath};
use crate::paint::{self, PaintContext};
use crate::{Error, Options, Size};
use roxmltree::{Document, Node as XmlNode, NodeId};
use std::cell::Cell;
use styles::Style;

pub(crate) fn parse(input: &str, options: &Options) -> Result<Drawable, Error> {
    let document = Document::parse(input)?;
    let root = document.root_element();
    if !root.has_tag_name("svg") {
        return Err(Error::Invalid("root element must be svg".into()));
    }
    if root
        .tag_name()
        .namespace()
        .is_some_and(|ns| ns != "http://www.w3.org/2000/svg")
    {
        return Err(Error::Invalid("root SVG namespace is invalid".into()));
    }
    let dimensions = dimensions::resolve(root, options)?;
    let parser = Parser {
        document: &document,
        css: css::Stylesheet::parse(&document)?,
        references: references::References::new(&document),
        root: root.id(),
        remaining: Cell::new(100_000),
    };
    let frame = Frame {
        style: Style::default(),
        transform: dimensions.transform,
        viewport: dimensions.source_viewport,
        active: Vec::new(),
    };
    let nodes = parser.render(root, &frame)?;
    Ok(Drawable {
        size: dimensions.size,
        viewport: dimensions.viewport,
        nodes,
    })
}

struct Parser<'a, 'input> {
    document: &'a Document<'input>,
    css: css::Stylesheet,
    references: references::References<'a, 'input>,
    root: NodeId,
    remaining: Cell<usize>,
}

#[derive(Clone)]
struct Frame {
    style: Style,
    transform: Transform,
    viewport: Size,
    active: Vec<NodeId>,
}

impl<'a, 'input> Parser<'a, 'input> {
    fn consume(&self) -> Result<(), Error> {
        let remaining =
            self.remaining.get().checked_sub(1).ok_or_else(|| {
                Error::Invalid("SVG expanded element count exceeds 100000".into())
            })?;
        self.remaining.set(remaining);
        Ok(())
    }

    fn frame(&self, node: XmlNode<'_, '_>, parent: &Frame) -> Result<Frame, Error> {
        if parent.active.len() >= 128 {
            return Err(Error::Invalid(
                "SVG nesting or reference depth exceeds 128".into(),
            ));
        }
        parser_properties::check_attributes(node)?;
        let style = parent
            .style
            .derive(&self.css.properties(node)?, parent.viewport)?;
        let transform = parent.transform.multiply(style.transform);
        if ![
            transform.a,
            transform.b,
            transform.c,
            transform.d,
            transform.e,
            transform.f,
        ]
        .into_iter()
        .all(f64::is_finite)
        {
            return Err(Error::Invalid(
                "non-finite accumulated transformation".into(),
            ));
        }
        let mut active = parent.active.clone();
        active.push(node.id());
        Ok(Frame {
            transform,
            style,
            viewport: parent.viewport,
            active,
        })
    }

    fn render(&self, node: XmlNode<'a, 'input>, parent: &Frame) -> Result<Vec<Node>, Error> {
        self.consume()?;
        let tag = node.tag_name().name();
        match tag {
            "defs" | "style" | "title" | "desc" | "metadata" | "linearGradient"
            | "radialGradient" | "clipPath" | "symbol" => return Ok(Vec::new()),
            _ => {}
        }
        let frame = self.frame(node, parent)?;
        if !frame.style.displayed || frame.style.opacity == 0.0 {
            return Ok(Vec::new());
        }
        let nodes = match tag {
            "svg" if node.id() == self.root => self.children(node, &frame)?,
            "svg" => return Err(Error::Unsupported("nested svg viewport".into())),
            "g" | "a" => self.children(node, &frame)?,
            "use" => self.use_node(node, &frame)?,
            "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" => {
                self.shape(node, &frame)?
            }
            _ => return Err(Error::Unsupported(format!("element <{tag}>"))),
        };
        self.apply_clip(nodes, &frame)
    }

    fn children(&self, node: XmlNode<'a, 'input>, frame: &Frame) -> Result<Vec<Node>, Error> {
        let mut nodes = Vec::new();
        for child in node.children().filter(XmlNode::is_element) {
            nodes.extend(self.render(child, frame)?);
        }
        Ok(nodes)
    }

    fn use_node(&self, node: XmlNode<'a, 'input>, frame: &Frame) -> Result<Vec<Node>, Error> {
        let target = self
            .references
            .resolve(references::href(node)?, &frame.active)?;
        if matches!(
            target.tag_name().name(),
            "symbol" | "svg" | "defs" | "clipPath" | "linearGradient" | "radialGradient"
        ) {
            return Err(Error::Unsupported(format!(
                "use referencing {}",
                target.tag_name().name()
            )));
        }
        let x = dimensions::length(
            node.attribute("x").unwrap_or("0"),
            Some(frame.viewport.width),
        )?;
        let y = dimensions::length(
            node.attribute("y").unwrap_or("0"),
            Some(frame.viewport.height),
        )?;
        let translated = Frame {
            transform: frame.transform.multiply(Transform::translate(x, y)),
            ..frame.clone()
        };
        self.render(target, &translated)
    }

    fn shape(&self, node: XmlNode<'_, '_>, frame: &Frame) -> Result<Vec<Node>, Error> {
        if !frame.style.visible {
            return Ok(Vec::new());
        }
        let local_data = shapes::path(node, frame.viewport)?;
        let bounds = local_data.bounds();
        let data = local_data.transformed(frame.transform);
        if data.is_empty() {
            return Ok(Vec::new());
        }
        let style = &frame.style;
        let context = PaintContext {
            document: self.document,
            viewport: frame.viewport,
            bounds,
            transform: frame.transform,
            current_color: &style.color,
        };
        let fill = paint::resolve(&style.fill, style.opacity * style.fill_opacity, &context)?;
        let stroke = paint::resolve(
            &style.stroke,
            style.opacity * style.stroke_opacity,
            &context,
        )?;
        let stroke_width = if style.non_scaling_stroke {
            style.stroke_width
        } else {
            style.stroke_width * frame.transform.determinant().abs().sqrt()
        };
        Ok(vec![Node::Path(Box::new(VectorPath {
            data,
            fill,
            stroke,
            stroke_width,
            fill_rule: style.fill_rule,
            line_cap: style.line_cap,
            line_join: style.line_join,
            miter_limit: style.miter_limit,
        }))])
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "parser_geometry_tests.rs"]
mod geometry_tests;

#[cfg(test)]
#[path = "parser_validation_tests.rs"]
mod validation_tests;
