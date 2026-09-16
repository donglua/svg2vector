use crate::Error;
use roxmltree::Node;
use std::collections::BTreeMap;

pub(crate) type Properties = BTreeMap<String, String>;

pub(super) fn declarations(input: &str) -> Result<Vec<(String, String, bool)>, Error> {
    input
        .split(';')
        .filter(|entry| !entry.trim().is_empty())
        .map(|entry| {
            let (name, value) = entry
                .split_once(':')
                .ok_or_else(|| Error::Invalid(format!("invalid style declaration: {entry}")))?;
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim();
            let important = value.ends_with("!important");
            let value = value.strip_suffix("!important").unwrap_or(value).trim();
            if value.is_empty() {
                return Err(Error::Invalid(format!("empty style value: {name}")));
            }
            check_property(&name, value)?;
            Ok((name, value.to_owned(), important))
        })
        .collect()
}

pub(super) fn is_property(name: &str) -> bool {
    matches!(
        name,
        "stop-color"
            | "stop-opacity"
            | "fill"
            | "fill-rule"
            | "fill-opacity"
            | "stroke"
            | "stroke-width"
            | "stroke-opacity"
            | "stroke-linecap"
            | "stroke-linejoin"
            | "stroke-miterlimit"
            | "opacity"
            | "color"
            | "display"
            | "visibility"
            | "clip-path"
            | "clip-rule"
            | "vector-effect"
            | "transform"
    )
}

pub(super) fn check_property(name: &str, value: &str) -> Result<(), Error> {
    if is_property(name) || matches!(name, "stop-color" | "stop-opacity") {
        return Ok(());
    }
    match (name, value) {
        (
            "filter" | "mask" | "marker" | "marker-start" | "marker-mid" | "marker-end"
            | "stroke-dasharray",
            "none",
        )
        | ("stroke-dashoffset", "0")
        | ("paint-order", "normal")
        | ("isolation", "auto")
        | ("mix-blend-mode", "normal")
        | ("overflow", "hidden") => Ok(()),
        ("shape-rendering", "auto" | "geometricPrecision") | ("color-interpolation", "sRGB") => {
            Ok(())
        }
        _ => Err(Error::Unsupported(format!("style property {name}={value}"))),
    }
}

pub(super) fn check_attributes(node: Node<'_, '_>) -> Result<(), Error> {
    for attribute in node.attributes() {
        if attribute.namespace().is_some() {
            continue;
        }
        let name = attribute.name();
        if name.starts_with("data-") || name.starts_with("aria-") {
            continue;
        }
        if is_property(name) {
            continue;
        }
        match name {
            "id"
            | "class"
            | "style"
            | "version"
            | "baseProfile"
            | "role"
            | "focusable"
            | "x"
            | "y"
            | "width"
            | "height"
            | "viewBox"
            | "preserveAspectRatio"
            | "d"
            | "points"
            | "cx"
            | "cy"
            | "r"
            | "rx"
            | "ry"
            | "x1"
            | "x2"
            | "y1"
            | "y2"
            | "href"
            | "clipPathUnits" => {}
            _ => check_property(name, attribute.value())?,
        }
    }
    Ok(())
}
