use crate::{Error, Result};

/// An RGB color with an alpha channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    /// Red channel in the inclusive range `0..=255`.
    pub red: u8,
    /// Green channel in the inclusive range `0..=255`.
    pub green: u8,
    /// Blue channel in the inclusive range `0..=255`.
    pub blue: u8,
    /// Alpha channel in the inclusive range `0..=255`.
    pub alpha: u8,
}

impl Color {
    /// Fully opaque black.
    pub const BLACK: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 255,
    };

    /// Parses three whitespace-separated RGB channels and an optional alpha channel.
    ///
    /// Every channel must be an integer in the inclusive range `0..=255`. An omitted
    /// alpha channel defaults to fully opaque.
    pub fn parse_rgb(value: &str, alpha: Option<&str>) -> Result<Self> {
        let mut channels = value.split_whitespace();
        let [Some(red), Some(green), Some(blue)] = [(); 3].map(|()| {
            channels
                .next()
                .and_then(|channel| channel.parse::<u8>().ok())
        }) else {
            return Err(invalid_color(value));
        };
        if channels.next().is_some() {
            return Err(invalid_color(value));
        }
        let alpha = alpha
            .map(str::parse::<u8>)
            .transpose()
            .map_err(|_| invalid_alpha(alpha.unwrap_or_default()))?
            .unwrap_or(255);

        Ok(Self {
            red,
            green,
            blue,
            alpha,
        })
    }
}

fn invalid_color(value: &str) -> Error {
    Error::InvalidValue {
        field: "color",
        value: value.to_owned(),
        path: None,
    }
}

fn invalid_alpha(value: &str) -> Error {
    Error::InvalidValue {
        field: "alpha",
        value: value.to_owned(),
        path: None,
    }
}
