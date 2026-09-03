use crate::{Error, Result};

/// A finite point expressed in OFD millimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
}

impl Point {
    /// Creates a point from two finite coordinates.
    pub fn new(x: f64, y: f64) -> Result<Self> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::InvalidValue {
                field: "point",
                value: format!("{x} {y}"),
            });
        }

        Ok(Self { x, y })
    }
}

/// A finite two-dimensional affine transform.
///
/// Points are mapped as `x' = a*x + c*y + e` and
/// `y' = b*x + d*y + f`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// Horizontal scale or rotation component.
    pub a: f64,
    /// Vertical shear or rotation component.
    pub b: f64,
    /// Horizontal shear or rotation component.
    pub c: f64,
    /// Vertical scale or rotation component.
    pub d: f64,
    /// Horizontal translation.
    pub e: f64,
    /// Vertical translation.
    pub f: f64,
}

impl Transform {
    /// The transform that leaves every point unchanged.
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// Parses six whitespace-separated finite matrix components.
    pub fn parse(value: &str) -> Result<Self> {
        let values = value
            .split_whitespace()
            .map(str::parse::<f64>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| invalid_transform(value))?;
        let &[a, b, c, d, e, f] = values.as_slice() else {
            return Err(invalid_transform(value));
        };

        if values.iter().any(|number| !number.is_finite()) {
            return Err(invalid_transform(value));
        }

        Ok(Self { a, b, c, d, e, f })
    }

    /// Composes two transforms, applying `self` first and `next` second.
    pub fn then(self, next: Self) -> Self {
        Self {
            a: next.a * self.a + next.c * self.b,
            b: next.b * self.a + next.d * self.b,
            c: next.a * self.c + next.c * self.d,
            d: next.b * self.c + next.d * self.d,
            e: next.a * self.e + next.c * self.f + next.e,
            f: next.b * self.e + next.d * self.f + next.f,
        }
    }

    /// Applies this transform to a point.
    pub fn apply(self, point: Point) -> Point {
        Point {
            x: self.a * point.x + self.c * point.y + self.e,
            y: self.b * point.x + self.d * point.y + self.f,
        }
    }
}

fn invalid_transform(value: &str) -> Error {
    Error::InvalidValue {
        field: "transform",
        value: value.to_owned(),
    }
}

/// A rectangle expressed in OFD millimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Left coordinate.
    pub x: f64,
    /// Top coordinate.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

impl Rect {
    /// Parses four whitespace-separated finite numbers.
    pub fn parse(value: &str) -> Result<Self> {
        let values = value
            .split_whitespace()
            .map(str::parse::<f64>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| Error::InvalidValue {
                field: "rectangle",
                value: value.to_owned(),
            })?;

        if values.len() != 4 || values.iter().any(|number| !number.is_finite()) {
            return Err(Error::InvalidValue {
                field: "rectangle",
                value: value.to_owned(),
            });
        }

        Ok(Self {
            x: values[0],
            y: values[1],
            width: values[2],
            height: values[3],
        })
    }
}
