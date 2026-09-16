use crate::Error;
use roxmltree::Node;

pub(super) fn server(node: Node<'_, '_>) -> Result<(), Error> {
    attributes(node, true)?;
    for child in node.children().filter(Node::is_element) {
        if !matches!(
            child.tag_name().name(),
            "stop" | "title" | "desc" | "metadata"
        ) {
            return Err(Error::Unsupported(format!(
                "gradient child {}",
                child.tag_name().name()
            )));
        }
    }
    Ok(())
}

pub(super) fn stop(node: Node<'_, '_>) -> Result<(), Error> {
    attributes(node, false)?;
    for child in node.children().filter(Node::is_element) {
        if !matches!(child.tag_name().name(), "title" | "desc" | "metadata") {
            return Err(Error::Unsupported(format!(
                "gradient stop child {}",
                child.tag_name().name()
            )));
        }
    }
    Ok(())
}

fn attributes(node: Node<'_, '_>, server: bool) -> Result<(), Error> {
    if node
        .tag_name()
        .namespace()
        .is_some_and(|namespace| namespace != "http://www.w3.org/2000/svg")
    {
        return Err(Error::Unsupported("non-SVG gradient namespace".into()));
    }
    for attribute in node
        .attributes()
        .filter(|attribute| attribute.namespace().is_none())
    {
        let name = attribute.name();
        if name.starts_with("data-") || name.starts_with("aria-") {
            continue;
        }
        match name {
            "id" | "class" | "style" | "color" | "stop-color" | "stop-opacity" => {}
            "color-interpolation" if matches!(attribute.value(), "sRGB" | "auto" | "inherit") => {}
            "gradientUnits" | "gradientTransform" | "spreadMethod" | "href" | "x1" | "y1"
            | "x2" | "y2" | "cx" | "cy" | "r" | "fx" | "fy" | "fr"
                if server => {}
            "offset" if !server => {}
            _ => {
                return Err(Error::Unsupported(format!(
                    "{} attribute {name}={}",
                    node.tag_name().name(),
                    attribute.value()
                )));
            }
        }
    }
    Ok(())
}
