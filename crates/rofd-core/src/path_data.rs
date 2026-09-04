use crate::{Error, Point, ResourceLimits, Result};

/// A single command in an OFD abbreviated path.
#[derive(Clone, Debug, PartialEq)]
pub enum PathCommand {
    /// Starts a new subpath at the given point.
    MoveTo(Point),
    /// Adds a straight line to the given point.
    LineTo(Point),
    /// Adds a quadratic Bézier curve.
    QuadraticTo {
        /// The curve's control point.
        control: Point,
        /// The curve's end point.
        end: Point,
    },
    /// Adds a cubic Bézier curve.
    CubicTo {
        /// The curve's first control point.
        control1: Point,
        /// The curve's second control point.
        control2: Point,
        /// The curve's end point.
        end: Point,
    },
    /// Adds an elliptical arc.
    ArcTo {
        /// The non-negative horizontal radius.
        rx: f64,
        /// The non-negative vertical radius.
        ry: f64,
        /// The ellipse's clockwise rotation in degrees in OFD's y-down space.
        rotation: f64,
        /// Whether to select the larger arc.
        large: bool,
        /// Whether to draw clockwise (`true`) or counter-clockwise (`false`).
        sweep: bool,
        /// The arc's end point.
        end: Point,
    },
    /// Closes the current subpath.
    Close,
}

/// A validated sequence of OFD abbreviated path commands.
#[derive(Clone, Debug, PartialEq)]
pub struct PathData {
    commands: Vec<PathCommand>,
}

impl PathData {
    /// Parses an OFD abbreviated path.
    ///
    /// Each command must be explicit. Drawing and close commands are rejected
    /// until a move command has introduced the first subpath. Closing a
    /// subpath keeps it active, so later drawing or close commands remain
    /// valid until another move starts a new subpath.
    pub fn parse(value: &str) -> Result<Self> {
        Self::parse_with_limit(value, ResourceLimits::default().max_path_commands)
    }

    /// Parses an OFD abbreviated path with an explicit command-count limit.
    ///
    /// The limit is checked before parsing or storing each command. A limit of
    /// zero therefore rejects any input beginning with a recognized command
    /// with [`Error::LimitExceeded`].
    pub fn parse_with_limit(value: &str, max_commands: usize) -> Result<Self> {
        let mut parser = Parser::new(value);
        let mut commands = Vec::new();
        let mut has_subpath = false;

        while let Some(command) = parser.next_command()? {
            if commands.len() >= max_commands {
                let command_count = commands.len().saturating_add(1);
                return Err(Error::LimitExceeded(format!(
                    "path command count {command_count} exceeds limit {max_commands}"
                )));
            }
            let parsed = match command {
                b'M' => {
                    has_subpath = true;
                    PathCommand::MoveTo(parser.point()?)
                }
                b'L' if has_subpath => PathCommand::LineTo(parser.point()?),
                b'Q' if has_subpath => PathCommand::QuadraticTo {
                    control: parser.point()?,
                    end: parser.point()?,
                },
                b'B' if has_subpath => PathCommand::CubicTo {
                    control1: parser.point()?,
                    control2: parser.point()?,
                    end: parser.point()?,
                },
                b'A' if has_subpath => {
                    let rx = parser.number()?;
                    let ry = parser.number()?;
                    let rotation = parser.number()?;
                    let large = parser.flag()?;
                    let sweep = parser.flag()?;
                    let end = parser.point()?;
                    if rx < 0.0 || ry < 0.0 {
                        return Err(invalid_path(value));
                    }
                    PathCommand::ArcTo {
                        rx,
                        ry,
                        rotation,
                        large,
                        sweep,
                        end,
                    }
                }
                b'C' if has_subpath => PathCommand::Close,
                _ => return Err(invalid_path(value)),
            };
            commands.push(parsed);
        }

        if commands.is_empty() {
            return Err(invalid_path(value));
        }

        Ok(Self { commands })
    }

    /// Returns the commands in source order.
    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }
}

struct Parser<'a> {
    value: &'a str,
    offset: usize,
}

impl<'a> Parser<'a> {
    fn new(value: &'a str) -> Self {
        Self { value, offset: 0 }
    }

    fn next_command(&mut self) -> Result<Option<u8>> {
        self.skip_whitespace();
        let Some(command) = self.byte() else {
            return Ok(None);
        };
        if !matches!(command, b'M' | b'L' | b'Q' | b'B' | b'A' | b'C') {
            return Err(invalid_path(self.value));
        }
        self.offset += 1;
        Ok(Some(command))
    }

    fn point(&mut self) -> Result<Point> {
        let x = self.number()?;
        let y = self.number()?;
        Point::new(x, y).map_err(|_| invalid_path(self.value))
    }

    fn flag(&mut self) -> Result<bool> {
        let token = self.number_token()?;
        match token {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(invalid_path(self.value)),
        }
    }

    fn number(&mut self) -> Result<f64> {
        let token = self.number_token()?;
        let number = token.parse::<f64>().map_err(|_| invalid_path(self.value))?;
        if !number.is_finite() {
            return Err(invalid_path(self.value));
        }
        Ok(number)
    }

    fn number_token(&mut self) -> Result<&'a str> {
        self.skip_whitespace();
        let start = self.offset;

        if matches!(self.byte(), Some(b'+' | b'-')) {
            self.offset += 1;
        }

        let integer_start = self.offset;
        self.consume_digits();
        let mut has_digits = self.offset > integer_start;

        if self.byte() == Some(b'.') {
            self.offset += 1;
            let fraction_start = self.offset;
            self.consume_digits();
            has_digits |= self.offset > fraction_start;
        }

        if !has_digits {
            return Err(invalid_path(self.value));
        }

        if matches!(self.byte(), Some(b'e' | b'E')) {
            self.offset += 1;
            if matches!(self.byte(), Some(b'+' | b'-')) {
                self.offset += 1;
            }
            let exponent_start = self.offset;
            self.consume_digits();
            if self.offset == exponent_start {
                return Err(invalid_path(self.value));
            }
        }

        self.value
            .get(start..self.offset)
            .ok_or_else(|| invalid_path(self.value))
    }

    fn consume_digits(&mut self) {
        while matches!(self.byte(), Some(b'0'..=b'9')) {
            self.offset += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.byte(), Some(byte) if byte.is_ascii_whitespace()) {
            self.offset += 1;
        }
    }

    fn byte(&self) -> Option<u8> {
        self.value.as_bytes().get(self.offset).copied()
    }
}

fn invalid_path(value: &str) -> Error {
    Error::InvalidValue {
        field: "path data",
        value: value.to_owned(),
        path: None,
    }
}
