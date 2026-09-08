use crate::{Error, Result};

/// A finite point expressed in OFD millimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    x: f64,
    y: f64,
}

impl Point {
    /// Creates a point from two finite coordinates.
    pub fn new(x: f64, y: f64) -> Result<Self> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::InvalidValue {
                field: "point",
                value: format!("{x} {y}"),
                path: None,
            });
        }

        Ok(Self { x, y })
    }

    /// Returns the horizontal coordinate.
    pub fn x(self) -> f64 {
        self.x
    }

    /// Returns the vertical coordinate.
    pub fn y(self) -> f64 {
        self.y
    }
}

/// A finite two-dimensional affine transform.
///
/// Points are mapped as `x' = a*x + c*y + e` and
/// `y' = b*x + d*y + f`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
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

    /// Creates an affine transform from six finite matrix components.
    pub fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Result<Self> {
        if [a, b, c, d, e, f]
            .iter()
            .any(|component| !component.is_finite())
        {
            return Err(invalid_transform(&format!("{a} {b} {c} {d} {e} {f}")));
        }

        Ok(Self { a, b, c, d, e, f })
    }

    /// Parses six whitespace-separated finite matrix components.
    pub fn parse(value: &str) -> Result<Self> {
        let mut components = value.split_whitespace();
        let [Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)] = [(); 6].map(|()| {
            components
                .next()
                .and_then(|component| component.parse::<f64>().ok())
        }) else {
            return Err(invalid_transform(value));
        };
        if components.next().is_some() {
            return Err(invalid_transform(value));
        }

        Self::new(a, b, c, d, e, f).map_err(|_| invalid_transform(value))
    }

    /// Composes two transforms, applying `self` first and `next` second.
    ///
    /// Returns an error if composition overflows to a non-finite component.
    pub fn then(self, next: Self) -> Result<Self> {
        Self::new(
            next.a * self.a + next.c * self.b,
            next.b * self.a + next.d * self.b,
            next.a * self.c + next.c * self.d,
            next.b * self.c + next.d * self.d,
            next.a * self.e + next.c * self.f + next.e,
            next.b * self.e + next.d * self.f + next.f,
        )
    }

    /// Returns the determinant of the linear part (`a*d - b*c`).
    pub fn determinant(self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// Returns whether the linear part is singular, collapsing every point
    /// onto a line or a single point.
    ///
    /// A singular transform has no inverse, so renderers cannot draw content
    /// using it; such content is invisible and should be skipped.
    pub fn is_singular(self) -> bool {
        self.determinant() == 0.0
    }

    /// Applies this transform to a point.
    ///
    /// Returns an error if mapping overflows to a non-finite coordinate.
    pub fn apply(self, point: Point) -> Result<Point> {
        Point::new(
            self.a * point.x + self.c * point.y + self.e,
            self.b * point.x + self.d * point.y + self.f,
        )
    }

    /// Returns the horizontal scale or rotation component.
    pub fn a(self) -> f64 {
        self.a
    }

    /// Returns the vertical shear or rotation component.
    pub fn b(self) -> f64 {
        self.b
    }

    /// Returns the horizontal shear or rotation component.
    pub fn c(self) -> f64 {
        self.c
    }

    /// Returns the vertical scale or rotation component.
    pub fn d(self) -> f64 {
        self.d
    }

    /// Returns the horizontal translation.
    pub fn e(self) -> f64 {
        self.e
    }

    /// Returns the vertical translation.
    pub fn f(self) -> f64 {
        self.f
    }
}

fn invalid_transform(value: &str) -> Error {
    Error::InvalidValue {
        field: "transform",
        value: value.to_owned(),
        path: None,
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
                path: None,
            })?;

        if values.len() != 4 || values.iter().any(|number| !number.is_finite()) {
            return Err(Error::InvalidValue {
                field: "rectangle",
                value: value.to_owned(),
                path: None,
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
