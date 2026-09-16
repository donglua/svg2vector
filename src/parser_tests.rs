use super::parse;
use crate::model::{FillRule, Node, Paint};
use crate::{Options, Size};

#[test]
fn translates_nonzero_viewbox_when_origin_is_present() {
    // Given
    let svg = r#"<svg viewBox="10 20 30 40"><path d="M10 20L40 60Z"/></svg>"#;
    // When
    let parsed = parse(svg, &Options::default()).expect("valid SVG");
    // Then
    let Node::Path(path) = &parsed.nodes[0] else {
        panic!("path expected")
    };
    assert_eq!(path.data.to_android(), "M0,0L30,40Z");
    assert_eq!(
        parsed.viewport,
        Size {
            width: 30.0,
            height: 40.0
        }
    );
}

#[test]
fn multiplies_group_opacity_when_styles_are_inherited() {
    // Given
    let svg = r#"<svg viewBox="0 0 10 10"><g fill="red" opacity="0.5"><path d="M0 0L1 1" opacity="0.4" fill-opacity="0.5"/></g></svg>"#;
    // When
    let parsed = parse(svg, &Options::default()).expect("valid SVG");
    // Then
    let Node::Path(path) = &parsed.nodes[0] else {
        panic!("path expected")
    };
    let Paint::Solid { opacity, .. } = path.fill else {
        panic!("solid paint expected")
    };
    assert!((opacity - 0.1).abs() < 1e-10);
}

#[test]
fn resolves_css_by_specificity_when_selectors_compete() {
    // Given
    let svg = r#"<svg viewBox="0 0 10 10"><style>#mark{fill:blue}.hot{fill:green}path{fill:red}</style><path id="mark" class="hot" d="M0 0L1 1" fill="yellow"/></svg>"#;
    // When
    let parsed = parse(svg, &Options::default()).expect("valid SVG");
    // Then
    let Node::Path(path) = &parsed.nodes[0] else {
        panic!("path expected")
    };
    let Paint::Solid { color, .. } = &path.fill else {
        panic!("solid paint expected")
    };
    assert_eq!(color, "#0000FF");
}

#[test]
fn rejects_cycles_when_use_references_itself() {
    // Given
    let svg = r##"<svg viewBox="0 0 10 10"><defs><g id="a"><use href="#a"/></g></defs><use href="#a"/></svg>"##;
    // When
    let result = parse(svg, &Options::default());
    // Then
    assert!(matches!(result, Err(crate::Error::Invalid(_))));
}

#[test]
fn preserves_evenodd_when_clip_path_has_holes() {
    // Given
    let svg = r##"<svg viewBox="0 0 10 10"><defs><clipPath id="cut"><path clip-rule="evenodd" d="M0 0H10V10H0ZM2 2H8V8H2Z"/></clipPath></defs><rect width="10" height="10" clip-path="url(#cut)"/></svg>"##;
    // When
    let parsed = parse(svg, &Options::default()).expect("valid SVG");
    // Then
    let Node::ClipGroup { clips, .. } = &parsed.nodes[0] else {
        panic!("clip expected")
    };
    assert_eq!(clips[0].fill_rule, FillRule::EvenOdd);
}

#[test]
fn rejects_unsupported_visuals_when_text_would_be_lost() {
    // Given
    let svg = r#"<svg viewBox="0 0 10 10"><text x="0" y="2">A</text></svg>"#;
    // When
    let result = parse(svg, &Options::default());
    // Then
    assert!(matches!(result, Err(crate::Error::Unsupported(_))));
}

#[test]
fn resolves_last_definition_when_ids_are_duplicated() {
    // Given
    let svg = r##"<svg viewBox="0 0 10 10"><defs><rect id="shape" width="1" height="1"/><rect id="shape" width="3" height="4"/></defs><use href="#shape"/></svg>"##;
    // When
    let parsed = parse(svg, &Options::default()).expect("AOSP permits duplicate IDs");
    // Then
    let Node::Path(path) = &parsed.nodes[0] else {
        panic!("path expected")
    };
    let bounds = path.data.bounds();
    assert_eq!((bounds.width, bounds.height), (3.0, 4.0));
}

#[test]
fn rejects_multiple_evenodd_clip_shapes_when_regions_overlap() {
    // Given
    let svg = r##"<svg viewBox="0 0 10 10"><defs><clipPath id="cut" clip-rule="evenodd"><rect width="8" height="10"/><rect x="2" width="8" height="10"/></clipPath></defs><rect width="10" height="10" clip-path="url(#cut)"/></svg>"##;
    // When
    let result = parse(svg, &Options::default());
    // Then
    assert!(matches!(result, Err(crate::Error::Unsupported(_))));
}

#[test]
fn rejects_multiple_nonzero_clip_shapes_when_windings_are_opposite() {
    // Given
    let svg = r##"<svg viewBox="0 0 10 10"><defs><clipPath id="cut"><path d="M0 0H10V10H0Z"/><path d="M0 0V10H10V0Z"/></clipPath></defs><rect width="10" height="10" clip-path="url(#cut)"/></svg>"##;
    // When
    let result = parse(svg, &Options::default());
    // Then
    assert!(matches!(result, Err(crate::Error::Unsupported(_))));
}

#[test]
fn rejects_multiple_clip_shapes_when_group_and_use_resolve_them() {
    // Given
    let svg = r##"<svg viewBox="0 0 10 10"><defs><rect id="shape" width="8" height="10"/><clipPath id="cut"><g><use href="#shape"/><use href="#shape" x="2"/></g></clipPath></defs><rect width="10" height="10" clip-path="url(#cut)"/></svg>"##;
    // When
    let result = parse(svg, &Options::default());
    // Then
    assert!(matches!(result, Err(crate::Error::Unsupported(_))));
}

#[test]
fn retains_single_compound_clip_when_other_shapes_are_empty() {
    // Given
    let svg = r##"<svg viewBox="0 0 10 10"><defs><clipPath id="cut"><rect width="0" height="10"/><path d="M0 0H10V10H0ZM2 2V8H8V2Z"/></clipPath></defs><rect width="10" height="10" clip-path="url(#cut)"/></svg>"##;
    // When
    let parsed = parse(svg, &Options::default()).expect("single compound clip is supported");
    // Then
    let Node::ClipGroup { clips, .. } = &parsed.nodes[0] else {
        panic!("clip expected")
    };
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].fill_rule, FillRule::NonZero);
    assert_eq!(
        clips[0].data.to_android(),
        "M0,0L10,0L10,10L0,10ZM2,2L2,8L8,8L8,2Z"
    );
}
