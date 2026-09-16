use super::parse;
use crate::model::Node;
use crate::{Options, Size};

#[test]
fn centers_original_artwork_when_canvas_is_larger() {
    // Given
    let svg = r#"<svg viewBox="0 0 10 20"><rect width="10" height="20"/></svg>"#;
    let options = Options {
        canvas: Some(Size {
            width: 30.0,
            height: 40.0,
        }),
        ..Options::default()
    };
    // When
    let parsed = parse(svg, &options).expect("valid SVG");
    // Then
    let Node::Path(path) = &parsed.nodes[0] else {
        panic!("path expected")
    };
    let bounds = path.data.bounds();
    assert_eq!(
        (bounds.x, bounds.y, bounds.width, bounds.height),
        (10.0, 10.0, 10.0, 20.0)
    );
    assert_eq!(
        parsed.size,
        Size {
            width: 30.0,
            height: 40.0
        }
    );
}

#[test]
fn fits_without_stretching_when_intrinsic_aspect_differs() {
    // Given
    let svg =
        r#"<svg width="30" height="10" viewBox="0 0 10 10"><rect width="10" height="10"/></svg>"#;
    // When
    let parsed = parse(svg, &Options::default()).expect("valid SVG");
    // Then
    let Node::Path(path) = &parsed.nodes[0] else {
        panic!("path expected")
    };
    let bounds = path.data.bounds();
    assert_eq!(
        (bounds.x, bounds.y, bounds.width, bounds.height),
        (10.0, 0.0, 10.0, 10.0)
    );
    assert_eq!(
        parsed.viewport,
        Size {
            width: 30.0,
            height: 10.0
        }
    );
}

#[test]
fn retains_intrinsic_ratio_when_only_width_is_overridden() {
    // Given
    let svg = r#"<svg width="20" height="40"><rect width="10" height="20"/></svg>"#;
    let options = Options {
        width: Some(30.0),
        ..Options::default()
    };
    // When
    let parsed = parse(svg, &options).expect("valid SVG");
    // Then
    assert_eq!(
        parsed.size,
        Size {
            width: 30.0,
            height: 60.0
        }
    );
    assert_eq!(
        parsed.viewport,
        Size {
            width: 20.0,
            height: 40.0
        }
    );
}

#[test]
fn composes_group_use_and_target_transforms_when_referencing_shape() {
    // Given
    let svg = r##"<svg viewBox="0 0 100 100"><defs><rect id="r" width="2" height="3" transform="translate(1 2)"/></defs><g transform="scale(2)"><use href="#r" x="3" y="4"/></g></svg>"##;
    // When
    let parsed = parse(svg, &Options::default()).expect("valid SVG");
    // Then
    let Node::Path(path) = &parsed.nodes[0] else {
        panic!("path expected")
    };
    let bounds = path.data.bounds();
    assert_eq!(
        (bounds.x, bounds.y, bounds.width, bounds.height),
        (8.0, 12.0, 4.0, 6.0)
    );
}

#[test]
fn uses_local_bounds_when_object_bounding_box_clip_is_transformed() {
    // Given
    let svg = r##"<svg viewBox="0 0 100 100"><defs><clipPath id="c" clipPathUnits="objectBoundingBox"><rect width="0.5" height="1"/></clipPath></defs><rect x="10" y="20" width="30" height="40" transform="translate(5 7) scale(2)" clip-path="url(#c)"/></svg>"##;
    // When
    let parsed = parse(svg, &Options::default()).expect("valid SVG");
    // Then
    let Node::ClipGroup { clips, .. } = &parsed.nodes[0] else {
        panic!("clip expected")
    };
    let bounds = clips[0].data.bounds();
    assert_eq!(
        (bounds.x, bounds.y, bounds.width, bounds.height),
        (25.0, 47.0, 30.0, 80.0)
    );
}

#[test]
fn converts_every_basic_shape_when_dimensions_are_valid() {
    // Given
    let svg = r#"<svg viewBox="0 0 100 100"><rect width="10" height="20" rx="2"/><circle cx="30" cy="30" r="10"/><ellipse cx="60" cy="30" rx="20" ry="10"/><line x1="10" y1="50" x2="20" y2="50"/><polyline points="30,50 40,60 50,50"/><polygon points="60,50 70,60 80,50"/></svg>"#;
    // When
    let parsed = parse(svg, &Options::default()).expect("valid SVG");
    // Then
    let extents: Vec<_> = parsed
        .nodes
        .iter()
        .map(|node| {
            let Node::Path(path) = node else {
                panic!("path expected")
            };
            let bounds = path.data.bounds();
            (
                bounds.x.round(),
                bounds.y.round(),
                bounds.width.round(),
                bounds.height.round(),
            )
        })
        .collect();
    assert_eq!(
        extents,
        [
            (0.0, 0.0, 10.0, 20.0),
            (20.0, 20.0, 20.0, 20.0),
            (40.0, 20.0, 40.0, 20.0),
            (10.0, 50.0, 10.0, 0.0),
            (30.0, 50.0, 20.0, 10.0),
            (60.0, 50.0, 20.0, 10.0)
        ]
    );
}
