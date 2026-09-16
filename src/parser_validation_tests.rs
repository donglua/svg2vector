use super::parse;
use crate::{Error, Options};

#[test]
fn rejects_unsupported_dash_patterns_when_they_affect_strokes() {
    // Given
    let svg = r#"<svg viewBox="0 0 10 10"><path d="M0 0L10 10" stroke="red" stroke-dasharray="2 3"/></svg>"#;
    // When
    let result = parse(svg, &Options::default());
    // Then
    assert!(matches!(result, Err(Error::Unsupported(_))));
}

#[test]
fn rejects_external_resources_when_use_href_is_remote() {
    // Given
    let svg =
        r#"<svg viewBox="0 0 10 10"><use href="https://example.org/icons.svg#remote"/></svg>"#;
    // When
    let result = parse(svg, &Options::default());
    // Then
    assert!(matches!(result, Err(Error::Unsupported(_))));
}

#[test]
fn rejects_nonfinite_opacity_when_it_is_not_a_number() {
    // Given
    let svg = r#"<svg viewBox="0 0 10 10"><path d="M0 0L10 10" opacity="NaN"/></svg>"#;
    // When
    let result = parse(svg, &Options::default());
    // Then
    assert!(matches!(result, Err(Error::Invalid(_))));
}

#[test]
fn rejects_overflow_when_ancestor_transforms_multiply() {
    // Given
    let svg = r#"<svg viewBox="0 0 10 10"><g transform="scale(1e200)"><path d="M1 1L2 2" transform="scale(1e200)"/></g></svg>"#;
    // When
    let result = parse(svg, &Options::default());
    // Then
    assert!(matches!(result, Err(Error::Invalid(_))));
}

#[test]
fn enforces_one_expansion_budget_when_multiple_clips_expand_empty_references() {
    // Given
    let mut svg =
        String::from(r#"<svg viewBox="0 0 10 10"><defs><rect id="n0" width="0" height="1"/>"#);
    for index in 1..=14 {
        svg.push_str(&format!(
            r##"<g id="n{index}"><use href="#n{}"/><use href="#n{}"/></g>"##,
            index - 1,
            index - 1
        ));
    }
    svg.push_str(r##"<clipPath id="c"><use href="#n14"/></clipPath></defs><rect width="1" height="1" clip-path="url(#c)"/><rect width="1" height="1" clip-path="url(#c)"/></svg>"##);
    // When
    let result = parse(&svg, &Options::default());
    // Then
    assert!(matches!(result, Err(Error::Invalid(_))));
}
