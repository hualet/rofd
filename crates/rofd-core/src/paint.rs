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

/// The device colour space a colour value is expressed in.
///
/// GB/T 33190-2016 clause 8.3 supports GRAY, RGB, and CMYK colour spaces;
/// the channel count of a colour value selects the space when no explicit
/// colour-space resource is referenced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ColorSpaceKind {
    /// One grey channel.
    Gray,
    /// Red, green, and blue channels.
    Rgb,
    /// Cyan, magenta, yellow, and black channels.
    Cmyk,
}

impl ColorSpaceKind {
    /// Returns the space implied by a channel count, if any.
    pub(crate) fn from_channel_count(count: usize) -> Option<Self> {
        match count {
            1 => Some(Self::Gray),
            3 => Some(Self::Rgb),
            4 => Some(Self::Cmyk),
            _ => None,
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

    /// Parses colour channels and converts them to an RGB colour.
    ///
    /// `declared_space` is the colour space named by a `ColorSpace` attribute
    /// or palette lookup, when available. The channel count always selects the
    /// effective space when it matches one of Gray/RGB/CMYK; strict mode also
    /// requires it to agree with a declared space. Channel tokens follow the
    /// same strict/compat grammar as [`Color::parse_rgb`] and
    /// [`Color::parse_rgb_compat`].
    pub(crate) fn parse_in_space(
        value: &str,
        alpha: Option<&str>,
        declared_space: Option<ColorSpaceKind>,
        strict: bool,
    ) -> Result<Self> {
        let parse_channel = |token: &str| {
            if strict {
                token
                    .parse::<u8>()
                    .map_err(|_| invalid_color(value))
                    .map(|channel| channel as f64)
            } else {
                parse_compat_channel(token)
                    .map(f64::from)
                    .map_err(|_| invalid_color(value))
            }
        };
        let channels = value
            .split_whitespace()
            .map(parse_channel)
            .collect::<Result<Vec<_>>>()?;
        let space = match ColorSpaceKind::from_channel_count(channels.len()) {
            Some(space) if !strict || declared_space.is_none_or(|declared| declared == space) => {
                space
            }
            _ => {
                return Err(invalid_color(value));
            }
        };
        let [red, green, blue] = convert_channels(&channels, space);
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

/// Parses one ofdrw-style color channel token.
fn parse_compat_channel(token: &str) -> std::result::Result<u8, ()> {
    if let Some(hex) = token.strip_prefix('#') {
        return u8::from_str_radix(hex, 16).map_err(|_| ());
    }
    if token.contains('.') {
        let parsed = token.parse::<f64>().map_err(|_| ())?;
        if parsed.is_finite() && (0.0..=255.0).contains(&parsed) {
            return Ok(parsed as u8);
        }
        return Err(());
    }
    token.parse::<u8>().map_err(|_| ())
}

/// Converts channels in `space` to 8-bit RGB.
///
/// Gray replicates its channel; RGB passes channels through; CMYK converts
/// with `255 * (1 - c/100) * (1 - k/100)` where channels are clamped to the
/// 0..=100 percentage range used by GB/T 33190 CMYK colours. The formula
/// matches ofdrw's `ColorConvert.cmykToRgb` with floating-point arithmetic
/// instead of ofdrw's truncated integer division.
fn convert_channels(channels: &[f64], space: ColorSpaceKind) -> [u8; 3] {
    match space {
        ColorSpaceKind::Gray => {
            let grey = channels[0].round().clamp(0.0, 255.0) as u8;
            [grey, grey, grey]
        }
        ColorSpaceKind::Rgb => [
            channels[0].round().clamp(0.0, 255.0) as u8,
            channels[1].round().clamp(0.0, 255.0) as u8,
            channels[2].round().clamp(0.0, 255.0) as u8,
        ],
        ColorSpaceKind::Cmyk => cmyk_to_rgb(channels[0], channels[1], channels[2], channels[3]),
    }
}

/// Converts one CMYK colour to RGB; see [`convert_channels`].
fn cmyk_to_rgb(cyan: f64, magenta: f64, yellow: f64, black: f64) -> [u8; 3] {
    let percentage = |channel: f64| channel.clamp(0.0, 100.0) / 100.0;
    let factor = |channel: f64| 1.0 - percentage(channel);
    let ink = factor(black);
    let rgb = |a: f64| (255.0 * factor(a) * ink).round().clamp(0.0, 255.0) as u8;
    [rgb(cyan), rgb(magenta), rgb(yellow)]
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

#[cfg(test)]
mod tests {
    use super::{cmyk_to_rgb, Color, ColorSpaceKind};

    #[test]
    fn channel_count_selects_the_color_space() {
        let gray = Color::parse_in_space("128", None, None, true).unwrap();
        assert_eq!((gray.red, gray.green, gray.blue), (128, 128, 128));
        let rgb = Color::parse_in_space("1 2 3", None, None, true).unwrap();
        assert_eq!((rgb.red, rgb.green, rgb.blue), (1, 2, 3));
    }

    #[test]
    fn cmyk_values_convert_with_percentage_clamping() {
        assert_eq!(cmyk_to_rgb(0.0, 0.0, 0.0, 0.0), [255, 255, 255]);
        assert_eq!(cmyk_to_rgb(0.0, 0.0, 0.0, 100.0), [0, 0, 0]);
        assert_eq!(cmyk_to_rgb(100.0, 100.0, 100.0, 0.0), [0, 0, 0]);
        let half = cmyk_to_rgb(50.0, 0.0, 0.0, 0.0);
        assert_eq!(half, [128, 255, 255]);
        let cmyk = Color::parse_in_space("50 20 0 10", None, None, true).unwrap();
        assert_eq!((cmyk.red, cmyk.green, cmyk.blue), (115, 184, 230));
    }

    #[test]
    fn compat_tokens_are_accepted_for_every_space() {
        let gray = Color::parse_in_space("#80", None, None, false).unwrap();
        assert_eq!((gray.red, gray.green, gray.blue), (128, 128, 128));
        // Compat channel parsing truncates floats like ofdrw's (int) cast,
        // so 1.5 becomes channel value 1.
        let cmyk = Color::parse_in_space("1.5 0 0 0", None, None, false).unwrap();
        assert_eq!((cmyk.red, cmyk.green, cmyk.blue), (252, 255, 255));
    }

    #[test]
    fn strict_mode_requires_the_declared_channel_count() {
        assert!(Color::parse_in_space("1 2 3", None, Some(ColorSpaceKind::Cmyk), true).is_err());
        assert!(Color::parse_in_space("1 2", None, None, true).is_err());
        assert!(Color::parse_in_space("1 2 3 4", None, Some(ColorSpaceKind::Rgb), true).is_err());
        assert!(Color::parse_in_space("1 2 3", None, Some(ColorSpaceKind::Rgb), true).is_ok());
    }
}
