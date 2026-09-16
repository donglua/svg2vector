use crate::geometry::{Bounds, Transform};
use crate::model::Paint;
use crate::{Error, Size};

#[path = "paint_color.rs"]
mod color;
#[path = "paint_coords.rs"]
mod coords;
#[path = "paint_gradient.rs"]
mod gradient;
#[path = "paint_validate.rs"]
mod validate;

pub(crate) struct PaintContext<'a, 'input> {
    pub document: &'a roxmltree::Document<'input>,
    pub viewport: Size,
    /// Bounds in the path's local coordinate system, before `transform`.
    pub bounds: Bounds,
    pub transform: Transform,
    pub current_color: &'a str,
}

pub(crate) fn resolve(
    value: &str,
    opacity: f64,
    context: &PaintContext<'_, '_>,
) -> Result<Paint, Error> {
    if !opacity.is_finite() {
        return Err(Error::Invalid("paint opacity must be finite".into()));
    }
    let opacity = opacity.clamp(0.0, 1.0);
    let value = value.trim();
    if value == "none" {
        return Ok(Paint::None);
    }
    if value.starts_with("url(") {
        return match svgtypes::Paint::from_str(value) {
            Ok(svgtypes::Paint::FuncIRI(id, None)) => gradient::resolve(id, opacity, context),
            Ok(svgtypes::Paint::FuncIRI(_, Some(_))) => {
                Err(Error::Unsupported("paint URL fallbacks".into()))
            }
            _ => Err(Error::Invalid(format!(
                "expected local paint reference: {value}"
            ))),
        };
    }
    let color = color::parse(value, context.current_color)?;
    Ok(Paint::Solid {
        color: color::android(color),
        opacity,
    })
}

#[cfg(test)]
#[path = "paint_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "paint_gradient_tests.rs"]
mod gradient_tests;

#[cfg(test)]
#[path = "paint_coords_tests.rs"]
mod coords_tests;
