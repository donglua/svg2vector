use super::parser_properties::{Properties, declarations, is_property};
use crate::Error;
use roxmltree::{Document, Node};
use std::collections::BTreeMap;

type Priority = (bool, usize, usize, usize, usize);
type Cascaded = BTreeMap<String, (Priority, String)>;

pub(crate) struct Stylesheet {
    rules: Vec<Rule>,
}
struct Rule {
    selector: Selector,
    declarations: Vec<(String, String, bool)>,
}
struct Selector {
    tag: Option<String>,
    ids: Vec<String>,
    classes: Vec<String>,
}

impl Stylesheet {
    pub fn parse(document: &Document<'_>) -> Result<Self, Error> {
        let mut rules = Vec::new();
        for node in document.descendants().filter(|n| n.has_tag_name("style")) {
            if node.attribute("type").is_some_and(|v| v != "text/css")
                || node.attribute("media").is_some_and(|v| v != "all")
            {
                return Err(Error::Unsupported(
                    "conditional or non-CSS stylesheet".into(),
                ));
            }
            let text: String = node.children().filter_map(|child| child.text()).collect();
            let stripped = strip_comments(&text)?;
            let mut rest = stripped.trim();
            while !rest.is_empty() {
                let (selectors, body) = rest
                    .split_once('{')
                    .ok_or_else(|| Error::Invalid("missing CSS declaration block".into()))?;
                let (body, tail) = body
                    .split_once('}')
                    .ok_or_else(|| Error::Invalid("unclosed CSS declaration block".into()))?;
                let declarations = declarations(body)?;
                for selector in selectors.split(',') {
                    rules.push(Rule {
                        selector: Selector::parse(selector.trim())?,
                        declarations: declarations.clone(),
                    });
                }
                rest = tail.trim();
            }
        }
        Ok(Self { rules })
    }

    pub fn properties(&self, node: Node<'_, '_>) -> Result<Properties, Error> {
        let mut values = Cascaded::new();
        for attr in node
            .attributes()
            .filter(|attr| attr.namespace().is_none() && is_property(attr.name()))
        {
            values.insert(
                attr.name().to_owned(),
                ((false, 0, 0, 0, 0), attr.value().to_owned()),
            );
        }
        for rule in &self.rules {
            if !rule.selector.matches(node) {
                continue;
            }
            for (name, value, important) in &rule.declarations {
                let priority = (
                    *important,
                    0,
                    rule.selector.ids.len(),
                    rule.selector.classes.len(),
                    usize::from(rule.selector.tag.is_some()),
                );
                // Iteration order breaks equal-specificity ties in favor of later rules.
                insert(&mut values, name, (priority, value));
            }
        }
        if let Some(inline) = node.attribute("style") {
            for (name, value, important) in declarations(inline)? {
                insert(&mut values, &name, ((important, 1, 0, 0, 0), &value));
            }
        }
        Ok(values
            .into_iter()
            .map(|(name, (_, value))| (name, value))
            .collect())
    }
}

fn insert(values: &mut Cascaded, name: &str, candidate: (Priority, &str)) {
    if values
        .get(name)
        .is_none_or(|(priority, _)| *priority <= candidate.0)
    {
        values.insert(name.to_owned(), (candidate.0, candidate.1.to_owned()));
    }
}

impl Selector {
    fn parse(input: &str) -> Result<Self, Error> {
        if input.is_empty()
            || input
                .chars()
                .any(|c| !(c.is_alphanumeric() || matches!(c, '-' | '_' | '#' | '.' | '*')))
        {
            return Err(Error::Unsupported(format!("CSS selector {input}")));
        }
        let mut selector = Self {
            tag: None,
            ids: Vec::new(),
            classes: Vec::new(),
        };
        let mut rest = input;
        if !rest.starts_with(['.', '#']) {
            let end = rest.find(['.', '#']).unwrap_or(rest.len());
            let tag = &rest[..end];
            if tag != "*" {
                selector.tag = Some(tag.to_owned());
            }
            rest = &rest[end..];
        }
        while !rest.is_empty() {
            let kind = rest.as_bytes()[0];
            rest = &rest[1..];
            let end = rest.find(['.', '#']).unwrap_or(rest.len());
            let name = &rest[..end];
            if name.is_empty() || name.contains('*') {
                return Err(Error::Unsupported(format!("CSS selector {input}")));
            }
            match kind {
                b'#' => selector.ids.push(name.to_owned()),
                b'.' => selector.classes.push(name.to_owned()),
                _ => return Err(Error::Unsupported(format!("CSS selector {input}"))),
            }
            rest = &rest[end..];
        }
        Ok(selector)
    }

    fn matches(&self, node: Node<'_, '_>) -> bool {
        self.tag
            .as_deref()
            .is_none_or(|tag| node.tag_name().name() == tag)
            && self
                .ids
                .iter()
                .all(|id| node.attribute("id") == Some(id.as_str()))
            && self.classes.iter().all(|class| {
                node.attribute("class")
                    .is_some_and(|all| all.split_whitespace().any(|item| item == class))
            })
    }
}

fn strip_comments(input: &str) -> Result<String, Error> {
    let mut result = String::new();
    let mut rest = input;
    while let Some((before, comment)) = rest.split_once("/*") {
        result.push_str(before);
        let (_, after) = comment
            .split_once("*/")
            .ok_or_else(|| Error::Invalid("unclosed CSS comment".into()))?;
        rest = after;
    }
    result.push_str(rest);
    Ok(result)
}
