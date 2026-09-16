use crate::Error;
use roxmltree::{Document, Node, NodeId};
use std::collections::HashMap;

pub(super) struct References<'a, 'input> {
    ids: HashMap<&'a str, Node<'a, 'input>>,
}

impl<'a, 'input> References<'a, 'input> {
    pub fn new(document: &'a Document<'input>) -> Self {
        let mut ids = HashMap::new();
        for node in document.descendants().filter(Node::is_element) {
            if let Some(id) = node.attribute("id") {
                ids.insert(id, node);
            }
        }
        Self { ids }
    }

    pub fn resolve(&self, value: &str, active: &[NodeId]) -> Result<Node<'a, 'input>, Error> {
        let id = value
            .trim()
            .strip_prefix('#')
            .ok_or_else(|| Error::Unsupported(format!("external reference: {value}")))?;
        if id.is_empty() {
            return Err(Error::Invalid("empty local reference".into()));
        }
        let node = self
            .ids
            .get(id)
            .copied()
            .ok_or_else(|| Error::Invalid(format!("unresolved reference: {id}")))?;
        if active.contains(&node.id()) {
            return Err(Error::Invalid(format!("reference cycle at #{id}")));
        }
        Ok(node)
    }
}

pub(super) fn href<'a, 'input>(node: Node<'a, 'input>) -> Result<&'a str, Error> {
    node.attribute("href")
        .or_else(|| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .ok_or_else(|| Error::Invalid("use element has no href".into()))
}

pub(super) fn url(value: &str) -> Result<&str, Error> {
    let reference = value
        .trim()
        .strip_prefix("url(")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| Error::Invalid(format!("invalid clip-path URL: {value}")))?
        .trim();
    Ok(reference.trim_matches(['\'', '"']))
}
