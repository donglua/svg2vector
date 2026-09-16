use super::{Bounds, Point, Transform, number};
use crate::Error;
use svgtypes::{SimplePathSegment, SimplifyingPathParser};

#[derive(Debug, Clone, Default)]
pub(crate) struct PathData {
    commands: Vec<Command>,
}
#[derive(Debug, Clone)]
enum Command {
    Move(Point),
    Line(Point),
    Cubic([Point; 3]),
    Quad([Point; 2]),
    Close,
}
impl Command {
    fn points(&self) -> &[Point] {
        match self {
            Self::Move(p) | Self::Line(p) => std::slice::from_ref(p),
            Self::Cubic(points) => points,
            Self::Quad(points) => points,
            Self::Close => &[],
        }
    }
}
impl PathData {
    pub fn parse(input: &str) -> Result<Self, Error> {
        let mut commands = Vec::new();
        for segment in SimplifyingPathParser::from(input) {
            let segment = segment.map_err(|e| Error::Invalid(format!("path data: {e}")))?;
            let command = match segment {
                SimplePathSegment::MoveTo { x, y } => Command::Move(Point { x, y }),
                SimplePathSegment::LineTo { x, y } => Command::Line(Point { x, y }),
                SimplePathSegment::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => Command::Cubic([
                    Point { x: x1, y: y1 },
                    Point { x: x2, y: y2 },
                    Point { x, y },
                ]),
                SimplePathSegment::Quadratic { x1, y1, x, y } => {
                    Command::Quad([Point { x: x1, y: y1 }, Point { x, y }])
                }
                SimplePathSegment::ClosePath => Command::Close,
            };
            commands.push(command);
        }
        let result = Self { commands };
        result.validate()?;
        Ok(result)
    }
    pub fn transformed(&self, transform: Transform) -> Self {
        let commands = self
            .commands
            .iter()
            .map(|command| match command {
                Command::Move(p) => Command::Move(transform.apply(*p)),
                Command::Line(p) => Command::Line(transform.apply(*p)),
                Command::Cubic(points) => Command::Cubic(points.map(|p| transform.apply(p))),
                Command::Quad(points) => Command::Quad(points.map(|p| transform.apply(p))),
                Command::Close => Command::Close,
            })
            .collect();
        Self { commands }
    }
    /// Control-point bounds match the Path2D bounds used by the upstream gradient pass.
    pub fn bounds(&self) -> Bounds {
        let mut points = self.commands.iter().flat_map(Command::points);
        let Some(first) = points.next() else {
            return Bounds::default();
        };
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (first.x, first.x, first.y, first.y);
        for p in points {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        Bounds {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
    pub fn validate(&self) -> Result<(), Error> {
        if self
            .commands
            .iter()
            .flat_map(Command::points)
            .all(|p| p.x.is_finite() && p.y.is_finite())
        {
            Ok(())
        } else {
            Err(Error::Invalid("non-finite path coordinates".into()))
        }
    }
    pub fn to_android(&self) -> String {
        let mut output = String::new();
        for command in &self.commands {
            output.push(match command {
                Command::Move(_) => 'M',
                Command::Line(_) => 'L',
                Command::Cubic(_) => 'C',
                Command::Quad(_) => 'Q',
                Command::Close => 'Z',
            });
            for (i, p) in command.points().iter().enumerate() {
                if i > 0 {
                    output.push(' ');
                }
                output.push_str(&number(p.x));
                output.push(',');
                output.push_str(&number(p.y));
            }
        }
        output
    }
}
