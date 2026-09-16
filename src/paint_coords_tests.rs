#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{PaintContext, resolve};
use crate::Size;
use crate::geometry::{Bounds, Point, Transform};
use crate::model::{GradientKind, Paint};

fn kind(
    attributes: &str,
    radial: bool,
    bounds: Bounds,
    viewport: Size,
    transform: Transform,
) -> GradientKind {
    let tag = if radial {
        "radialGradient"
    } else {
        "linearGradient"
    };
    let source = format!(
        "<svg><{tag} id='g' {attributes}><stop stop-color='red'/><stop offset='1' stop-color='blue'/></{tag}></svg>"
    );
    let document = roxmltree::Document::parse(&source).unwrap();
    match resolve(
        "url(#g)",
        1.0,
        &PaintContext {
            document: &document,
            viewport,
            bounds,
            transform,
            current_color: "black",
        },
    )
    .unwrap()
    {
        Paint::Gradient(gradient) => gradient.kind,
        other => panic!("unexpected paint {other:?}"),
    }
}

fn bounds() -> Bounds {
    Bounds {
        x: 10.0,
        y: 20.0,
        width: 40.0,
        height: 20.0,
    }
}
fn viewport() -> Size {
    Size {
        width: 100.0,
        height: 100.0,
    }
}
fn assert_point(point: Point, x: f64, y: f64) {
    assert!((point.x - x).abs() < 1e-8, "{} != {x}", point.x);
    assert!((point.y - y).abs() < 1e-8, "{} != {y}", point.y);
}

#[test]
fn linear_defaults_use_percentages_when_in_user_space() {
    // Given omitted user-space endpoint attributes and a 100-unit viewport.
    let attributes = "gradientUnits='userSpaceOnUse'";
    // When the gradient coordinates are resolved.
    let result = kind(attributes, false, bounds(), viewport(), Transform::IDENTITY);
    // Then the default endpoint is 100 percent of the viewport width.
    match result {
        GradientKind::Linear { start, end } => {
            assert_point(start, 0.0, 0.0);
            assert_point(end, 100.0, 0.0);
        }
        other => panic!("unexpected gradient {other:?}"),
    }
}

#[test]
fn radial_defaults_use_percentages_when_in_user_space() {
    // Given omitted radial attributes and a square 100-unit viewport.
    let attributes = "gradientUnits='userSpaceOnUse'";
    // When the radial coordinates are resolved.
    let result = kind(attributes, true, bounds(), viewport(), Transform::IDENTITY);
    // Then both center coordinates and radius default to 50 percent.
    match result {
        GradientKind::Radial { center, radius } => {
            assert_point(center, 50.0, 50.0);
            assert!((radius - 50.0).abs() < 1e-8);
        }
        other => panic!("unexpected gradient {other:?}"),
    }
}

#[test]
fn radial_percentage_radius_uses_diagonal_when_viewport_is_not_square() {
    // Given a percentage radius and a viewport with unequal dimensions.
    let attributes = "gradientUnits='userSpaceOnUse' r='50%'";
    let viewport = Size {
        width: 100.0,
        height: 80.0,
    };
    // When the radial coordinates are resolved.
    let result = kind(attributes, true, bounds(), viewport, Transform::IDENTITY);
    // Then the radius uses the normalized diagonal.
    match result {
        GradientKind::Radial { radius, .. } => {
            assert!((radius - 8200.0_f64.sqrt() / 2.0).abs() < 1e-8)
        }
        other => panic!("unexpected gradient {other:?}"),
    }
}

#[test]
fn bounding_box_gradient_rotates_when_the_shape_rotates() {
    // Given a rectangle rotated 90 degrees around its center at (30, 30).
    let transform = Transform {
        a: 0.0,
        b: 1.0,
        c: -1.0,
        d: 0.0,
        e: 60.0,
        f: 0.0,
    };
    // When the default bounding-box gradient is resolved.
    let result = kind("", false, bounds(), viewport(), transform);
    // Then the gradient follows the shape's original horizontal axis.
    match result {
        GradientKind::Linear { start, end } => {
            assert_point(start, 40.0, 10.0);
            assert_point(end, 40.0, 50.0);
        }
        other => panic!("unexpected gradient {other:?}"),
    }
}

#[test]
fn bounding_box_origin_is_preserved_when_gradient_is_scaled() {
    // Given a rectangle at a nonzero origin and a half-size gradient transform.
    let bounds = Bounds {
        x: 20.0,
        y: 30.0,
        width: 40.0,
        height: 20.0,
    };
    // When the gradient is resolved in normalized bounding-box coordinates.
    let result = kind(
        "gradientTransform='scale(.5)'",
        false,
        bounds,
        viewport(),
        Transform::IDENTITY,
    );
    // Then scaling affects the gradient span without moving the box origin.
    match result {
        GradientKind::Linear { start, end } => {
            assert_point(start, 20.0, 30.0);
            assert_point(end, 40.0, 30.0);
        }
        other => panic!("unexpected gradient {other:?}"),
    }
}

#[test]
fn linear_interpolation_is_preserved_when_the_transform_shears() {
    // Given a diagonal normalized gradient and a sheared rectangular shape.
    let transform = Transform {
        a: 1.0,
        b: 0.25,
        c: 0.75,
        d: 1.0,
        e: 3.0,
        f: -2.0,
    };
    // When Android endpoints are calculated for the complete affine transform.
    let result = kind(
        "x1='.2' y1='.3' x2='.8' y2='.9'",
        false,
        bounds(),
        viewport(),
        transform,
    );
    // Then multiple independent sample points retain their original interpolation factors.
    match result {
        GradientKind::Linear { start, end } => {
            let (dx, dy) = (end.x - start.x, end.y - start.y);
            for (u, v) in [(0.2, 0.3), (0.6, 0.5), (0.8, 0.9), (0.1, 0.9)] {
                let (x, y) = (10.0 + 40.0 * u, 20.0 + 20.0 * v);
                let target = Point {
                    x: x + 0.75 * y + 3.0,
                    y: 0.25 * x + y - 2.0,
                };
                let actual =
                    ((target.x - start.x) * dx + (target.y - start.y) * dy) / (dx * dx + dy * dy);
                let expected = (u + v - 0.5) / 1.2;
                assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
            }
        }
        other => panic!("unexpected gradient {other:?}"),
    }
}
