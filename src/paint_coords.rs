// Based on Android SvgGradientNode; SVG defaults, local bounds, and affine normals
// intentionally correct the upstream flattened-bounds approximation.
// Copyright (C) 2017 The Android Open Source Project
// Licensed under the Apache License, Version 2.0: https://www.apache.org/licenses/LICENSE-2.0

use super::{PaintContext, color, gradient::Definition};
use crate::geometry::{Point, Transform};
use crate::model::GradientKind;
use crate::{Error, Size};

struct Coordinates {
    size: Size,
    user_space: bool,
    transform: Transform,
}

pub(super) fn resolve(
    definition: &Definition<'_, '_>,
    context: &PaintContext<'_, '_>,
) -> Result<Option<GradientKind>, Error> {
    let user_space = match definition
        .attribute("gradientUnits")
        .unwrap_or("objectBoundingBox")
        .trim()
    {
        "objectBoundingBox" => false,
        "userSpaceOnUse" => true,
        units => return Err(Error::Unsupported(format!("gradientUnits: {units}"))),
    };
    let bounds = context.bounds;
    if !user_space && (bounds.width == 0.0 || bounds.height == 0.0) {
        return Ok(None);
    }
    let gradient_transform = definition
        .attribute("gradientTransform")
        .map(Transform::parse)
        .transpose()?
        .unwrap_or(Transform::IDENTITY);
    let transform = if user_space {
        context.transform.multiply(gradient_transform)
    } else {
        context
            .transform
            .multiply(Transform::translate(bounds.x, bounds.y))
            .multiply(Transform::scale(bounds.width, bounds.height))
            .multiply(gradient_transform)
    };
    if !transform.determinant().is_finite() || transform.determinant() == 0.0 {
        return Err(Error::Unsupported("singular gradient transform".into()));
    }
    let coordinates = Coordinates {
        size: context.viewport,
        user_space,
        transform,
    };
    let kind = if definition.radial {
        coordinates.radial(definition)?
    } else {
        let start = coordinates.point(definition, "x1", "y1", 0.0, 0.0)?;
        let end = coordinates.point(definition, "x2", "y2", 1.0, 0.0)?;
        linear(start, end, transform)
    };
    if !finite(&kind) {
        return Err(Error::Invalid(
            "gradient coordinates exceed finite range".into(),
        ));
    }
    Ok(Some(kind))
}

impl Coordinates {
    fn point(
        &self,
        definition: &Definition<'_, '_>,
        x: &str,
        y: &str,
        default_x: f64,
        default_y: f64,
    ) -> Result<Point, Error> {
        Ok(Point {
            x: self.coordinate(definition.attribute(x), default_x, self.size.width)?,
            y: self.coordinate(definition.attribute(y), default_y, self.size.height)?,
        })
    }

    fn coordinate(&self, value: Option<&str>, default: f64, length: f64) -> Result<f64, Error> {
        let number = value
            .map(color::number_or_percent)
            .transpose()?
            .unwrap_or(default);
        if self.user_space && value.is_none_or(|s| s.trim().ends_with('%')) {
            Ok(number * length)
        } else {
            Ok(number)
        }
    }

    fn radial(&self, definition: &Definition<'_, '_>) -> Result<GradientKind, Error> {
        let center = self.point(definition, "cx", "cy", 0.5, 0.5)?;
        let focus = Point {
            x: definition.attribute("fx").map_or(Ok(center.x), |value| {
                self.coordinate(Some(value), 0.0, self.size.width)
            })?,
            y: definition.attribute("fy").map_or(Ok(center.y), |value| {
                self.coordinate(Some(value), 0.0, self.size.height)
            })?,
        };
        if (definition.attribute("fx").is_some() && !near(focus.x, center.x))
            || (definition.attribute("fy").is_some() && !near(focus.y, center.y))
        {
            return Err(Error::Unsupported(
                "radial gradient focal point differs from its center".into(),
            ));
        }
        if let Some(value) = definition.attribute("fr") {
            if color::number_or_percent(value)? != 0.0 {
                return Err(Error::Unsupported("radial gradient focal radius".into()));
            }
        }
        let radius = self.coordinate(
            definition.attribute("r"),
            0.5,
            self.size.width.hypot(self.size.height) / std::f64::consts::SQRT_2,
        )?;
        if radius < 0.0 {
            return Err(Error::Invalid("radial gradient radius is negative".into()));
        }
        let x_scale = self.transform.a.hypot(self.transform.b);
        let y_scale = self.transform.c.hypot(self.transform.d);
        let dot = (self.transform.a / x_scale) * (self.transform.c / y_scale)
            + (self.transform.b / x_scale) * (self.transform.d / y_scale);
        if radius != 0.0 && (!near(x_scale, y_scale) || !near(dot, 0.0)) {
            return Err(Error::Unsupported(
                "elliptical radial gradient cannot be represented by VectorDrawable".into(),
            ));
        }
        Ok(GradientKind::Radial {
            center: self.transform.apply(center),
            radius: radius * x_scale,
        })
    }
}

fn linear(start: Point, end: Point, transform: Transform) -> GradientKind {
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let length = dx.hypot(dy);
    let start = transform.apply(start);
    if length == 0.0 {
        return GradientKind::Linear { start, end: start };
    }
    // A gradient is a scalar field: its normal transforms by A^-T, not A.
    // Normalize first to avoid squaring the original vector length.
    let (nx, ny) = (dx / length, dy / length);
    let determinant = transform.determinant();
    let normal = Point {
        x: (transform.d * nx - transform.b * ny) / determinant,
        y: (-transform.c * nx + transform.a * ny) / determinant,
    };
    let normal_length = normal.x.hypot(normal.y);
    let span = length / normal_length;
    let end = Point {
        x: start.x + (normal.x / normal_length) * span,
        y: start.y + (normal.y / normal_length) * span,
    };
    GradientKind::Linear { start, end }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn finite(kind: &GradientKind) -> bool {
    match kind {
        GradientKind::Linear { start, end } => [start.x, start.y, end.x, end.y]
            .iter()
            .all(|v| v.is_finite()),
        GradientKind::Radial { center, radius } => {
            [center.x, center.y, *radius].iter().all(|v| v.is_finite())
        }
    }
}
