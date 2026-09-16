//! Affine geometry used by the AOSP-style flattening pass.
use crate::Error;
use svgtypes::{Length, LengthUnit, TransformListParser, TransformListToken};

#[path = "geometry_path.rs"]
mod path;
pub(crate) use path::PathData;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Point {
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}
impl Transform {
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };
    pub const fn translate(x: f64, y: f64) -> Self {
        Self {
            e: x,
            f: y,
            ..Self::IDENTITY
        }
    }
    pub const fn scale(x: f64, y: f64) -> Self {
        Self {
            a: x,
            d: y,
            ..Self::IDENTITY
        }
    }
    /// Matrix product self * rhs; rhs is applied first.
    pub const fn multiply(self, rhs: Self) -> Self {
        Self {
            a: self.a * rhs.a + self.c * rhs.b,
            b: self.b * rhs.a + self.d * rhs.b,
            c: self.a * rhs.c + self.c * rhs.d,
            d: self.b * rhs.c + self.d * rhs.d,
            e: self.a * rhs.e + self.c * rhs.f + self.e,
            f: self.b * rhs.e + self.d * rhs.f + self.f,
        }
    }
    pub const fn apply(self, p: Point) -> Point {
        Point {
            x: self.a * p.x + self.c * p.y + self.e,
            y: self.b * p.x + self.d * p.y + self.f,
        }
    }
    pub const fn determinant(self) -> f64 {
        self.a * self.d - self.b * self.c
    }
    pub fn parse(input: &str) -> Result<Self, Error> {
        let mut result = Self::IDENTITY;
        for token in TransformListParser::from(input) {
            let token = token.map_err(|e| Error::Invalid(format!("transform: {e}")))?;
            let next = match token {
                TransformListToken::Matrix { a, b, c, d, e, f } => Self { a, b, c, d, e, f },
                TransformListToken::Translate { tx, ty } => Self::translate(tx, ty),
                TransformListToken::Scale { sx, sy } => Self::scale(sx, sy),
                TransformListToken::Rotate { angle } => {
                    let (sin, cos) = angle.to_radians().sin_cos();
                    Self {
                        a: cos,
                        b: sin,
                        c: -sin,
                        d: cos,
                        ..Self::IDENTITY
                    }
                }
                TransformListToken::SkewX { angle } => Self {
                    c: angle.to_radians().tan(),
                    ..Self::IDENTITY
                },
                TransformListToken::SkewY { angle } => Self {
                    b: angle.to_radians().tan(),
                    ..Self::IDENTITY
                },
            };
            result = result.multiply(next);
        }
        if [result.a, result.b, result.c, result.d, result.e, result.f]
            .into_iter()
            .all(f64::is_finite)
        {
            Ok(result)
        } else {
            Err(Error::Invalid("non-finite transformation matrix".into()))
        }
    }
}

pub(crate) fn number(value: f64) -> String {
    if value.abs() < 0.000_000_5 {
        return "0".into();
    }
    let formatted = format!("{value:.6}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

pub(crate) fn parse_length(input: &str) -> Result<f64, Error> {
    let value: Length = input
        .trim()
        .parse()
        .map_err(|e| Error::Invalid(format!("length {input:?}: {e}")))?;
    let scale = match value.unit {
        LengthUnit::None | LengthUnit::Px => 1.0,
        LengthUnit::In => 96.0,
        LengthUnit::Cm => 96.0 / 2.54,
        LengthUnit::Mm => 96.0 / 25.4,
        LengthUnit::Pt => 96.0 / 72.0,
        LengthUnit::Pc => 16.0,
        LengthUnit::Em | LengthUnit::Ex | LengthUnit::Percent => {
            return Err(Error::Unsupported(format!("relative length {input:?}")));
        }
    };
    let result = value.number * scale;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(Error::Invalid(format!("non-finite length {input:?}")))
    }
}

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod tests;
