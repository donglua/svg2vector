use crate::geometry::{Bounds, Transform, parse_length};
use crate::{Error, Options, Size};
use roxmltree::Node;

pub(super) struct Dimensions {
    pub size: Size,
    pub viewport: Size,
    pub source_viewport: Size,
    pub transform: Transform,
}

pub(super) fn resolve(root: Node<'_, '_>, options: &Options) -> Result<Dimensions, Error> {
    let view_box = root.attribute("viewBox").map(parse_view_box).transpose()?;
    let width = root
        .attribute("width")
        .map(|v| length(v, view_box.map(|b| b.width)))
        .transpose()?;
    let height = root
        .attribute("height")
        .map(|v| length(v, view_box.map(|b| b.height)))
        .transpose()?;
    let source = match view_box {
        Some(value) => value,
        None => Bounds {
            x: 0.0,
            y: 0.0,
            width: width.ok_or_else(|| Error::Invalid("missing width or viewBox".into()))?,
            height: height.ok_or_else(|| Error::Invalid("missing height or viewBox".into()))?,
        },
    };
    let source_viewport = Size::new(source.width, source.height)?;
    let intrinsic = Size::new(
        width.unwrap_or_else(|| height.map_or(source.width, |h| h * source.width / source.height)),
        height.unwrap_or_else(|| width.map_or(source.height, |w| w * source.height / source.width)),
    )?;
    let mut viewport = source_viewport;
    let mut transform = Transform::translate(-source.x, -source.y);
    if (intrinsic.width / intrinsic.height - source.width / source.height).abs() > 1e-12 {
        viewport = intrinsic;
        transform = fit(source, viewport, root.attribute("preserveAspectRatio"))?;
    } else {
        validate_aspect_ratio(root.attribute("preserveAspectRatio"))?;
    }
    let mut base_size = intrinsic;
    if let Some(canvas) = options.canvas {
        Size::new(canvas.width, canvas.height)?;
        transform = Transform::translate(
            (canvas.width - viewport.width) / 2.0,
            (canvas.height - viewport.height) / 2.0,
        )
        .multiply(transform);
        viewport = canvas;
        base_size = canvas;
    }
    let size = Size::new(
        options.width.unwrap_or_else(|| {
            options
                .height
                .map_or(base_size.width, |h| h * base_size.width / base_size.height)
        }),
        options.height.unwrap_or_else(|| {
            options
                .width
                .map_or(base_size.height, |w| w * base_size.height / base_size.width)
        }),
    )?;
    Ok(Dimensions {
        size,
        viewport,
        source_viewport,
        transform,
    })
}

pub(super) fn parse_view_box(value: &str) -> Result<Bounds, Error> {
    let values = numbers(value)?;
    let [x, y, width, height] = values.as_slice() else {
        return Err(Error::Invalid("viewBox requires four numbers".into()));
    };
    Size::new(*width, *height)?;
    Ok(Bounds {
        x: *x,
        y: *y,
        width: *width,
        height: *height,
    })
}

pub(super) fn numbers(value: &str) -> Result<Vec<f64>, Error> {
    svgtypes::NumberListParser::from(value)
        .map(|item| {
            let number = item.map_err(|error| Error::Invalid(error.to_string()))?;
            if number.is_finite() {
                Ok(number)
            } else {
                Err(Error::Invalid("non-finite number".into()))
            }
        })
        .collect()
}

pub(super) fn length(value: &str, reference: Option<f64>) -> Result<f64, Error> {
    match value.trim().strip_suffix('%') {
        Some(percent) => {
            let reference = reference
                .ok_or_else(|| Error::Unsupported("percentage without a viewport".into()))?;
            let number = percent
                .parse::<f64>()
                .map_err(|_| Error::Invalid(format!("invalid percentage: {value}")))?;
            let result = number * reference / 100.0;
            if result.is_finite() {
                Ok(result)
            } else {
                Err(Error::Invalid("non-finite percentage".into()))
            }
        }
        None => parse_length(value),
    }
}

fn validate_aspect_ratio(value: Option<&str>) -> Result<(&str, &str), Error> {
    let mut parts = value.unwrap_or("xMidYMid meet").split_whitespace();
    let alignment = parts.next().unwrap_or("xMidYMid");
    let mode = parts.next().unwrap_or("meet");
    if parts.next().is_some() || !matches!(mode, "meet" | "slice") {
        return Err(Error::Unsupported("preserveAspectRatio mode".into()));
    }
    if !matches!(
        alignment,
        "none"
            | "xMinYMin"
            | "xMidYMin"
            | "xMaxYMin"
            | "xMinYMid"
            | "xMidYMid"
            | "xMaxYMid"
            | "xMinYMax"
            | "xMidYMax"
            | "xMaxYMax"
    ) {
        return Err(Error::Unsupported(format!(
            "preserveAspectRatio={alignment}"
        )));
    }
    Ok((alignment, mode))
}

fn fit(source: Bounds, viewport: Size, aspect: Option<&str>) -> Result<Transform, Error> {
    let (alignment, mode) = validate_aspect_ratio(aspect)?;
    let sx = viewport.width / source.width;
    let sy = viewport.height / source.height;
    if alignment == "none" {
        return Ok(Transform::scale(sx, sy).multiply(Transform::translate(-source.x, -source.y)));
    }
    let scale = if mode == "slice" {
        sx.max(sy)
    } else {
        sx.min(sy)
    };
    let x = viewport.width - source.width * scale;
    let y = viewport.height - source.height * scale;
    let tx = if alignment.starts_with("xMin") {
        0.0
    } else if alignment.starts_with("xMax") {
        x
    } else {
        x / 2.0
    };
    let ty = if alignment.ends_with("YMin") {
        0.0
    } else if alignment.ends_with("YMax") {
        y
    } else {
        y / 2.0
    };
    Ok(Transform::translate(tx, ty)
        .multiply(Transform::scale(scale, scale))
        .multiply(Transform::translate(-source.x, -source.y)))
}
