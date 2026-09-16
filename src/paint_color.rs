use crate::Error;
use svgtypes::Color;

pub(super) fn parse(value: &str, current_color: &str) -> Result<Color, Error> {
    let value = value.trim();
    let value = if value.eq_ignore_ascii_case("currentColor") {
        current_color.trim()
    } else {
        value
    };
    if value.eq_ignore_ascii_case("none") {
        return Ok(Color::new_rgba(0, 0, 0, 0));
    }
    let lower = value.to_ascii_lowercase();
    if lower == "rebeccapurple" {
        return Ok(Color::new_rgb(0x66, 0x33, 0x99));
    }
    if let Some(arguments) = lower
        .strip_prefix("rgba(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return rgba(arguments, value);
    }
    value
        .parse()
        .map_err(|_| Error::Invalid(format!("invalid color: {value}")))
}

fn rgba(arguments: &str, original: &str) -> Result<Color, Error> {
    let parts: Vec<_> = arguments.split(',').map(str::trim).collect();
    let [red, green, blue, alpha] = parts.as_slice() else {
        return Err(Error::Invalid(format!("invalid rgba color: {original}")));
    };
    let mut color: Color = format!("rgb({red},{green},{blue})")
        .parse()
        .map_err(|_| Error::Invalid(format!("invalid rgba color: {original}")))?;
    let alpha = number_or_percent(alpha)?;
    if !(0.0..=1.0).contains(&alpha) {
        return Err(Error::Invalid(format!(
            "rgba alpha must be between 0 and 1: {original}"
        )));
    }
    color.alpha = (alpha * 255.0).round() as u8;
    Ok(color)
}

pub(super) fn number_or_percent(value: &str) -> Result<f64, Error> {
    let value = value.trim();
    let (number, factor) = value.strip_suffix('%').map_or((value, 1.0), |n| (n, 0.01));
    let parsed = number
        .parse::<f64>()
        .map_err(|_| Error::Invalid(format!("invalid number: {value}")))?
        * factor;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(Error::Invalid(format!("number must be finite: {value}")))
    }
}

pub(super) fn android(color: Color) -> String {
    if color.alpha == 255 {
        format!("#{:02X}{:02X}{:02X}", color.red, color.green, color.blue)
    } else {
        argb(color, 1.0)
    }
}

pub(super) fn argb(color: Color, opacity: f64) -> String {
    let alpha = (f64::from(color.alpha) * opacity) as u8;
    format!(
        "#{alpha:02X}{:02X}{:02X}{:02X}",
        color.red, color.green, color.blue
    )
}
