use super::{Frame, Parser, dimensions, references, shapes};
use crate::geometry::{Bounds, PathData, Transform};
use crate::model::{ClipPath, Node};
use crate::{Error, Size};
use roxmltree::Node as XmlNode;

impl<'a, 'input> Parser<'a, 'input> {
    pub(super) fn apply_clip(&self, nodes: Vec<Node>, frame: &Frame) -> Result<Vec<Node>, Error> {
        let Some(reference) = &frame.style.clip else {
            return Ok(nodes);
        };
        if nodes.is_empty() {
            return Ok(nodes);
        }
        let target = self
            .references
            .resolve(references::url(reference)?, &frame.active)?;
        if !target.has_tag_name("clipPath") {
            return Err(Error::Invalid("clip-path target is not a clipPath".into()));
        }
        let mut clip_frame = Frame {
            style: super::styles::Style::default(),
            ..frame.clone()
        };
        match target
            .attribute("clipPathUnits")
            .unwrap_or("userSpaceOnUse")
        {
            "userSpaceOnUse" => {}
            "objectBoundingBox" => {
                let bounds = local_bounds(&nodes, inverse(frame.transform)?)?;
                if bounds.width == 0.0 || bounds.height == 0.0 {
                    return Ok(Vec::new());
                }
                clip_frame.transform = frame
                    .transform
                    .multiply(Transform::translate(bounds.x, bounds.y))
                    .multiply(Transform::scale(bounds.width, bounds.height));
                clip_frame.viewport = Size {
                    width: 1.0,
                    height: 1.0,
                };
            }
            units => return Err(Error::Unsupported(format!("clipPathUnits={units}"))),
        }
        let clips = self.clip_shapes(target, &clip_frame)?;
        if clips.is_empty() {
            return Ok(Vec::new());
        }
        // SVG unions sibling clip shapes; concatenating their contours can cancel overlaps.
        if clips.len() > 1 {
            return Err(Error::Unsupported(
                "clipPath with multiple shapes requires unsupported geometric union".into(),
            ));
        }
        Ok(vec![Node::ClipGroup { clips, nodes }])
    }

    fn clip_shapes(
        &self,
        node: XmlNode<'a, 'input>,
        parent: &Frame,
    ) -> Result<Vec<ClipPath>, Error> {
        self.consume()?;
        if matches!(
            node.tag_name().name(),
            "title" | "desc" | "metadata" | "defs" | "style"
        ) {
            return Ok(Vec::new());
        }
        let frame = self.frame(node, parent)?;
        if !frame.style.displayed {
            return Ok(Vec::new());
        }
        if frame.style.clip.is_some() {
            return Err(Error::Unsupported(
                "nested clip-path inside clipPath".into(),
            ));
        }
        match node.tag_name().name() {
            "clipPath" | "g" => {
                let mut result = Vec::new();
                for child in node.children().filter(XmlNode::is_element) {
                    result.extend(self.clip_shapes(child, &frame)?);
                }
                Ok(result)
            }
            "use" => {
                let target = self
                    .references
                    .resolve(references::href(node)?, &frame.active)?;
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
                    ..frame
                };
                self.clip_shapes(target, &translated)
            }
            "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" => {
                if !frame.style.visible {
                    return Ok(Vec::new());
                }
                let data = shapes::path(node, frame.viewport)?.transformed(frame.transform);
                if data.is_empty() {
                    Ok(Vec::new())
                } else {
                    Ok(vec![ClipPath {
                        data,
                        fill_rule: frame.style.clip_rule,
                    }])
                }
            }
            name => Err(Error::Unsupported(format!("clipPath child <{name}>"))),
        }
    }
}

fn inverse(transform: Transform) -> Result<Transform, Error> {
    let det = transform.determinant();
    if det == 0.0 {
        return Err(Error::Invalid(
            "singular transformation with objectBoundingBox clip".into(),
        ));
    }
    Ok(Transform {
        a: transform.d / det,
        b: -transform.b / det,
        c: -transform.c / det,
        d: transform.a / det,
        e: (transform.c * transform.f - transform.d * transform.e) / det,
        f: (transform.b * transform.e - transform.a * transform.f) / det,
    })
}

fn local_bounds(nodes: &[Node], transform: Transform) -> Result<Bounds, Error> {
    let mut paths = Vec::new();
    collect_paths(nodes, &mut paths);
    let mut result: Option<Bounds> = None;
    for data in paths {
        let bounds = data.transformed(transform).bounds();
        result = Some(match result {
            None => bounds,
            Some(previous) => {
                let x = previous.x.min(bounds.x);
                let y = previous.y.min(bounds.y);
                Bounds {
                    x,
                    y,
                    width: (previous.x + previous.width).max(bounds.x + bounds.width) - x,
                    height: (previous.y + previous.height).max(bounds.y + bounds.height) - y,
                }
            }
        });
    }
    result.ok_or_else(|| Error::Invalid("clip target has no paths".into()))
}

fn collect_paths<'a>(nodes: &'a [Node], paths: &mut Vec<&'a PathData>) {
    for node in nodes {
        match node {
            Node::Path(path) => paths.push(&path.data),
            Node::ClipGroup { nodes, .. } => collect_paths(nodes, paths),
        }
    }
}
