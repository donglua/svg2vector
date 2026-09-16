use super::dimensions::{length, numbers};
use crate::geometry::PathData;
use crate::{Error, Size};
use roxmltree::Node;

pub(super) fn path(node: Node<'_, '_>, viewport: Size) -> Result<PathData, Error> {
    let shape = Shape { node, viewport };
    match node.tag_name().name() {
        "path" => PathData::parse(node.attribute("d").unwrap_or_default()),
        "rect" => shape.rectangle(),
        "circle" | "ellipse" => shape.ellipse(),
        "line" => PathData::parse(&format!(
            "M{} {}L{} {}",
            shape.x("x1")?,
            shape.y("y1")?,
            shape.x("x2")?,
            shape.y("y2")?
        )),
        "polyline" | "polygon" => {
            let points = numbers(node.attribute("points").unwrap_or_default())?;
            if points.len() % 2 != 0 {
                return Err(Error::Invalid("points require coordinate pairs".into()));
            }
            let mut data = String::new();
            for (index, pair) in points.chunks_exact(2).enumerate() {
                data.push_str(&format!(
                    "{}{} {}",
                    if index == 0 { 'M' } else { 'L' },
                    pair[0],
                    pair[1]
                ));
            }
            if node.has_tag_name("polygon") && !data.is_empty() {
                data.push('Z');
            }
            PathData::parse(&data)
        }
        name => Err(Error::Unsupported(format!("element <{name}>"))),
    }
}

struct Shape<'a, 'input> {
    node: Node<'a, 'input>,
    viewport: Size,
}

impl Shape<'_, '_> {
    fn x(&self, name: &str) -> Result<f64, Error> {
        self.value(name, self.viewport.width)
    }
    fn y(&self, name: &str) -> Result<f64, Error> {
        self.value(name, self.viewport.height)
    }
    fn value(&self, name: &str, reference: f64) -> Result<f64, Error> {
        self.node
            .attribute(name)
            .map_or(Ok(0.0), |value| length(value, Some(reference)))
    }

    fn rectangle(&self) -> Result<PathData, Error> {
        let x = self.x("x")?;
        let y = self.y("y")?;
        let width = self.x("width")?;
        let height = self.y("height")?;
        if width < 0.0 || height < 0.0 {
            return Err(Error::Invalid("negative rectangle dimensions".into()));
        }
        if width == 0.0 || height == 0.0 {
            return Ok(PathData::default());
        }
        let mut rx = self
            .node
            .attribute("rx")
            .map(|v| length(v, Some(self.viewport.width)))
            .transpose()?;
        let mut ry = self
            .node
            .attribute("ry")
            .map(|v| length(v, Some(self.viewport.height)))
            .transpose()?;
        rx = rx.or(ry);
        ry = ry.or(rx);
        let rx = rx.unwrap_or(0.0);
        let ry = ry.unwrap_or(0.0);
        if rx < 0.0 || ry < 0.0 {
            return Err(Error::Invalid("negative rectangle radius".into()));
        }
        let rx = rx.min(width / 2.0);
        let ry = ry.min(height / 2.0);
        if rx == 0.0 || ry == 0.0 {
            return PathData::parse(&format!("M{x} {y}h{width}v{height}h{}Z", -width));
        }
        let right = x + width;
        let bottom = y + height;
        PathData::parse(&format!(
            "M{} {y}H{}A{rx} {ry} 0 0 1 {right} {}V{}A{rx} {ry} 0 0 1 {} {bottom}H{}A{rx} {ry} 0 0 1 {x} {}V{}A{rx} {ry} 0 0 1 {} {y}Z",
            x + rx,
            right - rx,
            y + ry,
            bottom - ry,
            right - rx,
            x + rx,
            bottom - ry,
            y + ry,
            x + rx,
        ))
    }

    fn ellipse(&self) -> Result<PathData, Error> {
        let cx = self.x("cx")?;
        let cy = self.y("cy")?;
        let (rx, ry) = if self.node.has_tag_name("circle") {
            let radius = self.value(
                "r",
                self.viewport.width.hypot(self.viewport.height) / std::f64::consts::SQRT_2,
            )?;
            (radius, radius)
        } else {
            (self.x("rx")?, self.y("ry")?)
        };
        if rx < 0.0 || ry < 0.0 {
            return Err(Error::Invalid("negative ellipse radius".into()));
        }
        if rx == 0.0 || ry == 0.0 {
            return Ok(PathData::default());
        }
        PathData::parse(&format!(
            "M{} {cy}A{rx} {ry} 0 1 1 {} {cy}A{rx} {ry} 0 1 1 {} {cy}Z",
            cx - rx,
            cx + rx,
            cx - rx
        ))
    }
}
