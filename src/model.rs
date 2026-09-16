use crate::Size;
use crate::geometry::PathData;

#[derive(Debug)]
pub(crate) struct Drawable {
    pub size: Size,
    pub viewport: Size,
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
pub(crate) enum Node {
    Path(Box<VectorPath>),
    ClipGroup {
        clips: Vec<ClipPath>,
        nodes: Vec<Node>,
    },
}

#[derive(Debug)]
pub(crate) struct ClipPath {
    pub data: PathData,
    pub fill_rule: FillRule,
}

#[derive(Debug)]
pub(crate) struct VectorPath {
    pub data: PathData,
    pub fill: Paint,
    pub fill_rule: FillRule,
    pub stroke: Paint,
    pub stroke_width: f64,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub miter_limit: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FillRule {
    NonZero,
    EvenOdd,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineCap {
    Butt,
    Round,
    Square,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone)]
pub(crate) enum Paint {
    None,
    Solid { color: String, opacity: f64 },
    Gradient(Gradient),
}

#[derive(Debug, Clone)]
pub(crate) struct Gradient {
    pub kind: GradientKind,
    pub stops: Vec<Stop>,
    pub spread: Spread,
}
#[derive(Debug, Clone)]
pub(crate) enum GradientKind {
    Linear {
        start: crate::geometry::Point,
        end: crate::geometry::Point,
    },
    Radial {
        center: crate::geometry::Point,
        radius: f64,
    },
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum Spread {
    Clamp,
    Repeat,
    Mirror,
}
#[derive(Debug, Clone)]
pub(crate) struct Stop {
    pub offset: f64,
    pub color: String,
}
