//! Native SVG to Android VectorDrawable conversion.
mod error;
mod geometry;
mod model;
mod paint;
mod parser;
mod writer;

pub use error::Error;

/// Intrinsic dimensions or viewport dimensions, in SVG user units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    /// Construct a positive, finite size.
    pub fn new(width: f64, height: f64) -> Result<Self, Error> {
        if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
            Ok(Self { width, height })
        } else {
            Err(Error::Invalid(
                "dimensions must be finite and greater than zero".into(),
            ))
        }
    }
}

/// Conversion options. No overrides preserves source dimensions and aspect ratio.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Override intrinsic Android width in dp, without scaling path data.
    pub width: Option<f64>,
    /// Override intrinsic Android height in dp, without scaling path data.
    pub height: Option<f64>,
    /// Center the original viewport on this canvas without stretching the artwork.
    pub canvas: Option<Size>,
}

/// Convert SVG source into Android VectorDrawable XML.
pub fn convert(svg: &str, options: &Options) -> Result<String, Error> {
    let drawable = parser::parse(svg, options)?;
    writer::write(&drawable)
}
