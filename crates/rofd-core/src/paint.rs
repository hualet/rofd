use crate::{Error, Result};

/// Shape used where two stroked path segments meet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineJoin {
    /// Extend segment edges until they meet.
    Miter,
    /// Join segments with a circular arc.
    Round,
    /// Join segments with a straight bevel.
    Bevel,
}

/// Shape used at an open stroked path endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineCap {
    /// Stop the stroke at the endpoint.
    Butt,
    /// Extend the endpoint with a semicircle.
    Round,
    /// Extend the endpoint with a half-width square.
    Square,
}

/// Effective validated geometry of a stroked object.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub(crate) line_width: f64,
    pub(crate) line_join: LineJoin,
    pub(crate) line_cap: LineCap,
    pub(crate) dash_offset: f64,
    pub(crate) dash_pattern: Vec<f64>,
    pub(crate) miter_limit: f64,
}

impl StrokeStyle {
    /// Returns the positive stroke width in millimetres.
    pub fn line_width(&self) -> f64 {
        self.line_width
    }

    /// Returns the line-join shape.
    pub fn line_join(&self) -> LineJoin {
        self.line_join
    }

    /// Returns the line-cap shape.
    pub fn line_cap(&self) -> LineCap {
        self.line_cap
    }

    /// Returns the non-negative dash phase in millimetres.
    pub fn dash_offset(&self) -> f64 {
        self.dash_offset
    }

    /// Returns the positive dash lengths, or an empty slice for a solid stroke.
    pub fn dash_pattern(&self) -> &[f64] {
        &self.dash_pattern
    }

    /// Returns the positive miter limit.
    pub fn miter_limit(&self) -> f64 {
        self.miter_limit
    }
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            line_width: 0.353,
            line_join: LineJoin::Miter,
            line_cap: LineCap::Butt,
            dash_offset: 0.0,
            dash_pattern: Vec::new(),
            miter_limit: 3.528,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PaintParameters {
    pub(crate) line_width: Option<f64>,
    pub(crate) line_join: Option<LineJoin>,
    pub(crate) line_cap: Option<LineCap>,
    pub(crate) dash_offset: Option<f64>,
    pub(crate) dash_pattern: Option<Vec<f64>>,
    pub(crate) miter_limit: Option<f64>,
    pub(crate) fill_color: Option<Color>,
    pub(crate) stroke_color: Option<Color>,
}

impl PaintParameters {
    pub(crate) fn inherit(&mut self, child: &Self) {
        if child.line_width.is_some() {
            self.line_width = child.line_width;
        }
        if child.line_join.is_some() {
            self.line_join = child.line_join;
        }
        if child.line_cap.is_some() {
            self.line_cap = child.line_cap;
        }
        if child.dash_offset.is_some() {
            self.dash_offset = child.dash_offset;
        }
        if child.dash_pattern.is_some() {
            self.dash_pattern.clone_from(&child.dash_pattern);
        }
        if child.miter_limit.is_some() {
            self.miter_limit = child.miter_limit;
        }
        if child.fill_color.is_some() {
            self.fill_color = child.fill_color;
        }
        if child.stroke_color.is_some() {
            self.stroke_color = child.stroke_color;
        }
    }

    pub(crate) fn stroke_style(&self) -> StrokeStyle {
        let defaults = StrokeStyle::default();
        StrokeStyle {
            line_width: self.line_width.unwrap_or(defaults.line_width),
            line_join: self.line_join.unwrap_or(defaults.line_join),
            line_cap: self.line_cap.unwrap_or(defaults.line_cap),
            dash_offset: self.dash_offset.unwrap_or(defaults.dash_offset),
            dash_pattern: self.dash_pattern.clone().unwrap_or_default(),
            miter_limit: self.miter_limit.unwrap_or(defaults.miter_limit),
        }
    }
}

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
