// Gradient inheritance and stop normalization follow Android SvgGradientNode and Svg2Vector.
// Copyright (C) 2017 The Android Open Source Project
// Licensed under the Apache License, Version 2.0: https://www.apache.org/licenses/LICENSE-2.0

use super::{PaintContext, color, coords, validate};
use crate::Error;
use crate::model::{Gradient, GradientKind, Paint, Spread, Stop};
use crate::parser::css::Stylesheet;
use roxmltree::Node;

pub(super) struct Definition<'a, 'input> {
    nodes: Vec<Node<'a, 'input>>,
    pub radial: bool,
}

impl<'a, 'input> Definition<'a, 'input> {
    fn collect(id: &str, document: &'a roxmltree::Document<'input>) -> Result<Self, Error> {
        let mut nodes = Vec::new();
        let mut reference = id;
        loop {
            let node = document
                .descendants()
                .rfind(|node| node.attribute("id") == Some(reference))
                .ok_or_else(|| {
                    Error::Invalid(format!("gradient reference not found: #{reference}"))
                })?;
            if !matches!(node.tag_name().name(), "linearGradient" | "radialGradient") {
                return Err(Error::Unsupported(format!(
                    "paint reference #{reference} targets {}",
                    node.tag_name().name()
                )));
            }
            if nodes.contains(&node) {
                return Err(Error::Invalid(format!(
                    "cyclic gradient reference: #{reference}"
                )));
            }
            validate::server(node)?;
            nodes.push(node);
            let href = node
                .attribute("href")
                .or_else(|| node.attribute(("http://www.w3.org/1999/xlink", "href")));
            match href {
                None => break,
                Some(href) => {
                    reference = href
                        .trim()
                        .strip_prefix('#')
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| {
                            Error::Unsupported(format!("external or empty gradient href: {href}"))
                        })?;
                }
            }
        }
        let radial = nodes
            .first()
            .is_some_and(|node| node.has_tag_name("radialGradient"));
        Ok(Self { nodes, radial })
    }

    pub fn attribute(&self, name: &str) -> Option<&'a str> {
        self.nodes.iter().find_map(|node| node.attribute(name))
    }

    fn stops(&self, opacity: f64, context: &PaintContext<'_, '_>) -> Result<Vec<Stop>, Error> {
        let source = self
            .nodes
            .iter()
            .find(|node| node.children().any(|child| child.has_tag_name("stop")))
            .ok_or_else(|| Error::Invalid("gradient has no stops".into()))?;
        let mut result = Vec::new();
        let mut greatest_offset: f64 = 0.0;
        let stylesheet = Stylesheet::parse(context.document)?;
        for node in source.children().filter(Node::is_element) {
            match node.tag_name().name() {
                "stop" => {}
                "title" | "desc" | "metadata" => continue,
                name => return Err(Error::Unsupported(format!("gradient child {name}"))),
            }
            validate::stop(node)?;
            let offset = node
                .attribute("offset")
                .map(color::number_or_percent)
                .transpose()?
                .unwrap_or(greatest_offset);
            greatest_offset = greatest_offset.max(offset.clamp(0.0, 1.0));
            let stop_opacity = inherited(&stylesheet, node, "stop-opacity")?
                .as_deref()
                .map(color::number_or_percent)
                .transpose()?
                .unwrap_or(1.0);
            let current_color = inherited(&stylesheet, node, "color")?;
            let value = inherited(&stylesheet, node, "stop-color")?;
            let raw_color = value.as_deref().unwrap_or("black");
            let color = match color::parse(raw_color, current_color.as_deref().unwrap_or("black")) {
                Ok(color) => color::argb(color, opacity * stop_opacity.clamp(0.0, 1.0)),
                Err(_) => raw_color.to_owned(),
            };
            result.push(Stop {
                offset: greatest_offset,
                color,
            });
        }
        if result.len() == 1 {
            if let Some(stop) = result.first() {
                result.push(Stop {
                    offset: 1.0,
                    color: stop.color.clone(),
                });
            }
        }
        Ok(result)
    }
}

pub(super) fn resolve(
    id: &str,
    opacity: f64,
    context: &PaintContext<'_, '_>,
) -> Result<Paint, Error> {
    let definition = Definition::collect(id, context.document)?;
    let stops = definition.stops(opacity, context)?;
    let spread = match definition.attribute("spreadMethod").unwrap_or("pad").trim() {
        "pad" => Spread::Clamp,
        "repeat" => Spread::Repeat,
        "reflect" => Spread::Mirror,
        method => {
            return Err(Error::Unsupported(format!(
                "gradient spreadMethod: {method}"
            )));
        }
    };
    let Some(kind) = coords::resolve(&definition, context)? else {
        return Ok(Paint::None);
    };
    if matches!(kind, GradientKind::Radial { radius: 0.0, .. }) {
        return stops
            .last()
            .map(|stop| Paint::Solid {
                color: stop.color.clone(),
                opacity: 1.0,
            })
            .ok_or_else(|| Error::Invalid("gradient has no stops".into()));
    }
    Ok(Paint::Gradient(Gradient {
        kind,
        stops,
        spread,
    }))
}

fn inherited(
    stylesheet: &Stylesheet,
    node: Node<'_, '_>,
    name: &str,
) -> Result<Option<String>, Error> {
    for ancestor in node.ancestors().filter(Node::is_element) {
        if let Some(value) = stylesheet.properties(ancestor)?.remove(name) {
            if value != "inherit"
                && !(name == "color" && value.eq_ignore_ascii_case("currentColor"))
            {
                return Ok(Some(value));
            }
        }
    }
    Ok(None)
}
