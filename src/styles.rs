use super::dimensions::length;
use super::parser_properties::Properties;
use crate::geometry::Transform;
use crate::model::{FillRule, LineCap, LineJoin};
use crate::{Error, Size};

#[derive(Clone)]
pub(super) struct Style {
    pub fill: String,
    pub stroke: String,
    pub color: String,
    pub opacity: f64,
    local_opacity: f64,
    pub fill_opacity: f64,
    pub stroke_opacity: f64,
    pub stroke_width: f64,
    pub fill_rule: FillRule,
    pub clip_rule: FillRule,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub miter_limit: f64,
    pub displayed: bool,
    pub visible: bool,
    pub clip: Option<String>,
    pub transform: Transform,
    pub non_scaling_stroke: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fill: "black".into(),
            stroke: "none".into(),
            color: "black".into(),
            opacity: 1.0,
            local_opacity: 1.0,
            fill_opacity: 1.0,
            stroke_opacity: 1.0,
            stroke_width: 1.0,
            fill_rule: FillRule::NonZero,
            clip_rule: FillRule::NonZero,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 4.0,
            displayed: true,
            visible: true,
            clip: None,
            transform: Transform::IDENTITY,
            non_scaling_stroke: false,
        }
    }
}

impl Style {
    pub fn derive(&self, properties: &Properties, viewport: Size) -> Result<Self, Error> {
        let mut result = Self {
            local_opacity: 1.0,
            displayed: true,
            clip: None,
            transform: Transform::IDENTITY,
            non_scaling_stroke: false,
            ..self.clone()
        };
        for (name, value) in properties {
            if value == "inherit" {
                match name.as_str() {
                    "opacity" => result.local_opacity = self.local_opacity,
                    "clip-path" => result.clip.clone_from(&self.clip),
                    "transform" => result.transform = self.transform,
                    "display" => result.displayed = self.displayed,
                    "vector-effect" => result.non_scaling_stroke = self.non_scaling_stroke,
                    _ => {}
                }
                continue;
            }
            match name.as_str() {
                "fill" => result.fill.clone_from(value),
                "stroke" => result.stroke.clone_from(value),
                "color" if value.eq_ignore_ascii_case("currentColor") => {}
                "color" => result.color.clone_from(value),
                "opacity" => result.local_opacity = opacity(value)?,
                "fill-opacity" => result.fill_opacity = opacity(value)?,
                "stroke-opacity" => result.stroke_opacity = opacity(value)?,
                "fill-rule" => result.fill_rule = fill_rule(value)?,
                "clip-rule" => result.clip_rule = fill_rule(value)?,
                "stroke-width" => {
                    result.stroke_width = length(
                        value,
                        Some(viewport.width.hypot(viewport.height) / std::f64::consts::SQRT_2),
                    )?;
                    if result.stroke_width < 0.0 {
                        return Err(Error::Invalid("negative stroke width".into()));
                    }
                }
                "stroke-miterlimit" => {
                    result.miter_limit = value
                        .parse()
                        .map_err(|_| Error::Invalid("invalid stroke-miterlimit".into()))?;
                    if !result.miter_limit.is_finite() || result.miter_limit < 1.0 {
                        return Err(Error::Invalid(
                            "stroke-miterlimit must be at least 1".into(),
                        ));
                    }
                }
                "stroke-linecap" => {
                    result.line_cap = match value.as_str() {
                        "butt" => LineCap::Butt,
                        "round" => LineCap::Round,
                        "square" => LineCap::Square,
                        _ => return Err(Error::Unsupported(format!("stroke-linecap={value}"))),
                    }
                }
                "stroke-linejoin" => {
                    result.line_join = match value.as_str() {
                        "miter" => LineJoin::Miter,
                        "round" => LineJoin::Round,
                        "bevel" => LineJoin::Bevel,
                        _ => return Err(Error::Unsupported(format!("stroke-linejoin={value}"))),
                    }
                }
                "display" => result.displayed = value != "none",
                "visibility" => {
                    result.visible = match value.as_str() {
                        "visible" => true,
                        "hidden" | "collapse" => false,
                        _ => return Err(Error::Unsupported(format!("visibility={value}"))),
                    }
                }
                "clip-path" => result.clip = (value != "none").then(|| value.clone()),
                "transform" => {
                    result.transform = if value == "none" {
                        Transform::IDENTITY
                    } else {
                        Transform::parse(value)?
                    }
                }
                "vector-effect" => {
                    result.non_scaling_stroke = match value.as_str() {
                        "none" => false,
                        "non-scaling-stroke" => true,
                        _ => return Err(Error::Unsupported(format!("vector-effect={value}"))),
                    }
                }
                _ => {}
            }
        }
        result.opacity = self.opacity * result.local_opacity;
        Ok(result)
    }
}

fn opacity(value: &str) -> Result<f64, Error> {
    Ok(length(value, Some(1.0))?.clamp(0.0, 1.0))
}

fn fill_rule(value: &str) -> Result<FillRule, Error> {
    match value {
        "nonzero" => Ok(FillRule::NonZero),
        "evenodd" => Ok(FillRule::EvenOdd),
        _ => Err(Error::Unsupported(format!("fill or clip rule {value}"))),
    }
}
