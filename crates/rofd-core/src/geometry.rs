use crate::{Error, Result};

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
