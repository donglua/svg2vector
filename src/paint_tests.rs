#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{PaintContext, resolve};
use crate::Size;
use crate::geometry::{Bounds, Point, Transform};
use crate::model::{Gradient, GradientKind, Paint, Spread};

fn parse_paint(source: &str, value: &str, opacity: f64, transform: Transform) -> Paint {
    let document = roxmltree::Document::parse(source).unwrap();
    resolve(
        value,
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
            current_color: "rebeccapurple",
        },
    )
    .unwrap()
}

fn gradient(source: &str) -> Gradient {
    match parse_paint(source, "url(#g)", 1.0, Transform::IDENTITY) {
        Paint::Gradient(gradient) => gradient,
        paint => panic!("expected gradient, got {paint:?}"),
    }
}

fn assert_point(point: Point, x: f64, y: f64) {
    assert!((point.x - x).abs() < 1e-8, "{} != {x}", point.x);
    assert!((point.y - y).abs() < 1e-8, "{} != {y}", point.y);
}

#[test]
fn colors_preserve_alpha_and_opacity_when_resolved() {
    // Given common SVG color syntaxes and a separate paint opacity.
    let cases = [
        ("#abc", "#AABBCC"),
        ("#1234", "#44112233"),
        ("#11223380", "#80112233"),
        ("rgb(100%,0%,50%)", "#FF0080"),
        ("rgba(10,20,30,0.5)", "#800A141E"),
        ("rgba(10,20,30,50%)", "#800A141E"),
        ("currentColor", "#663399"),
        ("transparent", "#00000000"),
    ];
    for (input, expected) in cases {
        // When the color is resolved.
        let paint = parse_paint("<svg/>", input, 0.25, Transform::IDENTITY);
        // Then SVG alpha order and the independent opacity are preserved.
        match paint {
            Paint::Solid { color, opacity } => {
                assert_eq!(color, expected);
                assert_eq!(opacity, 0.25);
            }
            other => panic!("unexpected paint {other:?}"),
        }
    }
}

#[test]
fn gradient_preserves_normal_when_bounds_scale_unequally() {
    // Given local path bounds and a diagonal percentage gradient.
    let source = r##"<svg><linearGradient id="g" x1="25%" y1="50%" x2="100%" y2="0%"><stop stop-color="#123"/><stop offset="100%" stop-color="red"/></linearGradient></svg>"##;
    // When the gradient is resolved.
    let result = gradient(source);
    // Then the transformed normal preserves the SVG interpolation direction.
    match result.kind {
        GradientKind::Linear { start, end } => {
            assert_point(start, 20.0, 30.0);
            assert_point(end, 35.6, 9.2);
        }
        other => panic!("unexpected gradient {other:?}"),
    }
}

#[test]
fn stops_inherit_when_href_supplies_a_template() {
    // Given a template with styled alpha stops and a derived coordinate override.
    let source = r##"<svg><linearGradient id="base" x2="50%" spreadMethod="reflect"><stop offset="20%" style="stop-color:#ff000080;stop-opacity:0.5"/><stop offset="10%" stop-color="blue"/></linearGradient><linearGradient id="g" href="#base" x1="50%"/></svg>"##;
    // When the derived gradient is resolved.
    let result = gradient(source);
    // Then template stops, opacity, monotonic offsets, and spread are retained.
    assert_eq!(result.stops[0].color, "#40FF0000");
    assert_eq!(result.stops[0].offset, 0.2);
    assert_eq!(result.stops[1].offset, 0.2);
    assert!(matches!(result.spread, Spread::Mirror));
    match result.kind {
        GradientKind::Linear { start, end } => {
            assert_point(start, 30.0, 20.0);
            assert_point(end, 30.0, 20.0);
        }
        other => panic!("unexpected gradient {other:?}"),
    }
}

#[test]
fn invalid_paints_fail_when_references_or_values_are_invalid() {
    // Given invalid references, unsupported resources, and malformed colors.
    let cases = [
        ("<svg/>", "url(#missing)"),
        (
            r##"<svg><linearGradient id="g" href="#g"/></svg>"##,
            "url(#g)",
        ),
        (
            r##"<svg><linearGradient id="g" href="#b"/><linearGradient id="b" href="#g"/></svg>"##,
            "url(#g)",
        ),
        (r##"<svg><pattern id="g"/></svg>"##, "url(#g)"),
        ("<svg/>", "#12zz45"),
        ("<svg/>", "rgba(0,0,0,127)"),
        ("<svg/>", "url(other.svg#g)"),
    ];
    for (source, value) in cases {
        let document = roxmltree::Document::parse(source).unwrap();
        let context = PaintContext {
            document: &document,
            viewport: Size {
                width: 100.0,
                height: 80.0,
            },
            bounds: Bounds {
                width: 10.0,
                height: 10.0,
                ..Bounds::default()
            },
            transform: Transform::IDENTITY,
            current_color: "black",
        };
        // When resolving an invalid paint.
        let result = resolve(value, 1.0, &context);
        // Then conversion fails rather than silently replacing it.
        assert!(result.is_err(), "accepted {value} from {source}");
    }
}
