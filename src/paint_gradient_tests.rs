#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{PaintContext, resolve};
use crate::geometry::{Bounds, Point, Transform};
use crate::model::{GradientKind, Paint};
use crate::{Error, Size};

fn paint(definition: &str, opacity: f64, transform: Transform) -> Result<Paint, Error> {
    let document = roxmltree::Document::parse(definition)?;
    resolve(
        "url(#g)",
        opacity,
        &PaintContext {
            document: &document,
            viewport: Size {
                width: 100.0,
                height: 80.0,
            },
            bounds: Bounds {
                x: 10.0,
                y: 20.0,
                width: 40.0,
                height: 20.0,
            },
            transform,
            current_color: "blue",
        },
    )
}

fn assert_point(point: Point, x: f64, y: f64) {
    assert!((point.x - x).abs() < 1e-8, "{} != {x}", point.x);
    assert!((point.y - y).abs() < 1e-8, "{} != {y}", point.y);
}

#[test]
fn object_transform_composes_when_bounds_are_local() {
    // Given a normalized translation and local path bounds.
    let source = r##"<svg><linearGradient id="g" gradientTransform="translate(.25 .5)"><stop stop-color="red"/></linearGradient></svg>"##;
    // When the paint resolves with a shape translation.
    let result = paint(source, 1.0, Transform::translate(100.0, 100.0)).unwrap();
    // Then the normalized translation and shape translation both apply once.
    match result {
        Paint::Gradient(gradient) => match gradient.kind {
            GradientKind::Linear { start, end } => {
                assert_point(start, 120.0, 130.0);
                assert_point(end, 160.0, 130.0);
            }
            other => panic!("unexpected gradient {other:?}"),
        },
        other => panic!("unexpected paint {other:?}"),
    }
}

#[test]
fn user_space_applies_both_transforms_when_coordinates_are_percentages() {
    // Given user-space percentages and a gradient scale.
    let source = r##"<svg><linearGradient id="g" gradientUnits="userSpaceOnUse" gradientTransform="scale(2)" x1="10%" y1="50%" x2="100%" y2="0%"><stop stop-color="red"/></linearGradient></svg>"##;
    // When a shape translation is composed with the gradient transform.
    let result = paint(source, 1.0, Transform::translate(10.0, 20.0)).unwrap();
    // Then percentages use the viewport and gradient transform precedes geometry transform.
    match result {
        Paint::Gradient(gradient) => match gradient.kind {
            GradientKind::Linear { start, end } => {
                assert_point(start, 30.0, 100.0);
                assert_point(end, 210.0, 20.0);
            }
            other => panic!("unexpected gradient {other:?}"),
        },
        other => panic!("unexpected paint {other:?}"),
    }
}

#[test]
fn radial_radius_scales_when_rotation_and_uniform_scale_are_representable() {
    // Given a circular gradient with a uniform scale and quarter turn.
    let source = r##"<svg><radialGradient id="g" gradientUnits="userSpaceOnUse" cx="10" cy="20" r="5" fx="10" fy="20" gradientTransform="matrix(0 2 -2 0 0 0)"><stop stop-color="red"/></radialGradient></svg>"##;
    // When the shape translation is applied.
    let result = paint(source, 1.0, Transform::translate(4.0, 6.0)).unwrap();
    // Then center and radius remain a representable Android circle.
    match result {
        Paint::Gradient(gradient) => match gradient.kind {
            GradientKind::Radial { center, radius } => {
                assert_point(center, -36.0, 26.0);
                assert_eq!(radius, 10.0);
            }
            other => panic!("unexpected gradient {other:?}"),
        },
        other => panic!("unexpected paint {other:?}"),
    }
}

#[test]
fn stop_alpha_composes_when_styles_and_paint_opacity_apply() {
    // Given a styled stop that overrides its presentation attributes.
    let source = r##"<svg color="green"><linearGradient id="g"><stop stop-color="red" stop-opacity="1" style="stop-color:currentColor;stop-opacity:50%"/></linearGradient></svg>"##;
    // When paint opacity is also one half.
    let result = paint(source, 0.5, Transform::IDENTITY).unwrap();
    // Then alpha is multiplied and a single stop becomes a constant two-stop gradient.
    match result {
        Paint::Gradient(gradient) => {
            assert_eq!(gradient.stops.len(), 2);
            assert_eq!(gradient.stops[0].color, "#3F008000");
            assert_eq!(gradient.stops[1].offset, 1.0);
            assert_eq!(gradient.stops[1].color, "#3F008000");
        }
        other => panic!("unexpected paint {other:?}"),
    }
}

#[test]
fn radial_rejects_when_focal_point_or_ellipse_cannot_be_represented() {
    // Given unsupported radial geometry and malformed coordinate values.
    let attributes = [
        "",
        "gradientUnits='userSpaceOnUse' fx='1'",
        "gradientUnits='userSpaceOnUse' gradientTransform='scale(2 1)'",
        "gradientUnits='userSpaceOnUse' fr='.1'",
        "gradientUnits='userSpaceOnUse' r='-1'",
        "gradientUnits='userSpaceOnUse' r='NaN'",
        "gradientUnits='userSpaceOnUse' gradientTransform='scale(0)'",
        "gradientUnits='invalid'",
    ];
    for attributes in attributes {
        let source = format!(
            "<svg><radialGradient id='g' {attributes}><stop stop-color='red'/></radialGradient></svg>"
        );
        // When conversion encounters the unsupported geometry.
        let result = paint(&source, 1.0, Transform::IDENTITY);
        // Then it reports a conversion error.
        assert!(result.is_err(), "accepted {attributes}");
    }
}

#[test]
fn stylesheet_cascades_when_stops_have_selector_and_inline_rules() {
    // Given competing presentation, stylesheet, and inline stop declarations.
    let source = r##"<svg><style>.faded { stop-color: blue !important; stop-opacity: .25 } #first { stop-opacity: .5 }</style><linearGradient id="g"><stop id="first" class="faded" stop-color="red" style="stop-color:green"/></linearGradient></svg>"##;
    // When the referenced gradient is resolved through the shared CSS cascade.
    let result = paint(source, 1.0, Transform::IDENTITY).unwrap();
    // Then important color and the more specific opacity both take effect.
    match result {
        Paint::Gradient(gradient) => assert_eq!(gradient.stops[0].color, "#7F0000FF"),
        other => panic!("unexpected paint {other:?}"),
    }
}

#[test]
fn gradient_features_reject_when_android_cannot_represent_them() {
    // Given interpolation and animation that Android gradient XML cannot encode.
    let definitions = [
        r##"<linearGradient id="g" color-interpolation="linearRGB"><stop/></linearGradient>"##,
        r##"<linearGradient id="g"><stop><animate attributeName="stop-color"/></stop></linearGradient>"##,
        r##"<linearGradient id="g" href="#base"><animate attributeName="x1"/></linearGradient><linearGradient id="base"><stop/></linearGradient>"##,
    ];
    for definition in definitions {
        // When the referenced paint server is resolved.
        let result = paint(
            &format!("<svg>{definition}</svg>"),
            1.0,
            Transform::IDENTITY,
        );
        // Then it returns an explicit error instead of a static approximation.
        assert!(matches!(result, Err(Error::Unsupported(_))));
    }
}

#[test]
fn duplicate_gradient_ids_select_last_when_matching_aosp_lookup() {
    // Given exported definitions that reuse an ID.
    let source = r##"<svg><linearGradient id="g"><stop stop-color="red"/></linearGradient><linearGradient id="g"><stop stop-color="blue"/></linearGradient></svg>"##;
    // When the paint reference is resolved.
    let result = paint(source, 1.0, Transform::IDENTITY).unwrap();
    // Then the last definition wins, matching SvgTree.addIdToMap.
    match result {
        Paint::Gradient(gradient) => assert_eq!(gradient.stops[0].color, "#FF0000FF"),
        other => panic!("unexpected paint {other:?}"),
    }
}
