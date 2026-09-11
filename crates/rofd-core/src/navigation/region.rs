//! Conservative, renderer-independent bounds for explicit action regions.

use super::{
    actions::ActionParser,
    xml::{NavigationXml, XmlNode},
};
use crate::{Error, Point, Rect, Result, Transform};

#[derive(Default)]
struct Bounds {
    value: Option<(f64, f64, f64, f64)>,
}
impl Bounds {
    fn point(&mut self, point: Point) {
        let (x, y) = (point.x(), point.y());
        self.value = Some(match self.value {
            Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x), b.max(y)),
            None => (x, y, x, y),
        });
    }
    fn finish(self) -> Result<Rect> {
        let (x, y, right, bottom) = self.value.ok_or_else(invalid_geometry)?;
        let (width, height) = (right - x, bottom - y);
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(invalid_geometry());
        }
        Ok(Rect {
            x,
            y,
            width,
            height,
        })
    }
}

fn invalid_geometry() -> Error {
    Error::InvalidStructure {
        path: String::new(),
        message: "action region has empty, degenerate, or unrepresentable geometry".into(),
    }
}

pub(crate) fn translation(rect: Rect) -> Result<Transform> {
    Transform::new(1.0, 0.0, 0.0, 1.0, rect.x, rect.y)
}

pub(crate) fn map_rect(rect: Rect, transform: Transform) -> Result<Rect> {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return Err(invalid_geometry());
    }
    let mut bounds = Bounds::default();
    for (x, y) in [
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x, rect.y + rect.height),
        (rect.x + rect.width, rect.y + rect.height),
    ] {
        bounds.point(transform.apply(Point::new(x, y)?)?);
    }
    bounds.finish()
}

fn numbers(value: &str, max: usize, parser: &mut ActionParser<'_>) -> Result<Vec<f64>> {
    let mut result = Vec::new();
    for token in value.split_whitespace() {
        if result.len() >= max {
            return Err(parser.error(format_args!(
                "too many coordinates in action region {value:?}"
            )));
        }
        let number = token
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .ok_or_else(|| {
                parser.error(format_args!(
                    "invalid finite action region coordinate {value:?}"
                ))
            })?;
        result.push(number);
    }
    Ok(result)
}

fn point(node: &XmlNode, field: &str, parser: &mut ActionParser<'_>) -> Result<Point> {
    let value = parser.required(node, field)?;
    let values = numbers(value, 2, parser)?;
    if values.len() != 2 {
        return Err(parser.error(format_args!(
            "action region {field} requires two coordinates"
        )));
    }
    Point::new(values[0], values[1])
}

fn boolean(node: &XmlNode, field: &str, parser: &mut ActionParser<'_>) -> Result<bool> {
    match parser.required(node, field)? {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        value => Err(parser.error(format_args!("invalid action arc {field} {value:?}"))),
    }
}

pub(crate) fn areas(
    xml: &NavigationXml,
    index: usize,
    transform: Transform,
    commands: &mut usize,
    parser: &mut ActionParser<'_>,
) -> Result<Vec<Rect>> {
    let mut result = Vec::new();
    for &area in &xml.nodes[index].children {
        if !xml.nodes[area].is("Area") {
            return Err(parser.error(format_args!(
                "action Region has a non-native or unknown Area child"
            )));
        }
        let mut current = point(&xml.nodes[area], "Start", parser)?;
        let mut start = current;
        let mut bounds = Bounds::default();
        bounds.point(transform.apply(current)?);
        if xml.nodes[area].children.is_empty() {
            return Err(parser.error(format_args!(
                "action Region Area requires at least one segment"
            )));
        }
        for &segment in &xml.nodes[area].children {
            *commands = commands.checked_sub(1).ok_or_else(|| {
                Error::LimitExceeded(
                    "effective action region commands exceed max_path_commands".into(),
                )
            })?;
            let node = &xml.nodes[segment];
            if !node.children.is_empty() {
                return Err(parser.error(format_args!(
                    "action Region segment cannot contain child elements"
                )));
            }
            if node.is("Move") || node.is("Line") {
                current = point(node, "Point1", parser)?;
                if node.is("Move") {
                    start = current;
                }
                bounds.point(transform.apply(current)?);
            } else if node.is("QuadraticBezier") {
                bounds.point(transform.apply(point(node, "Point1", parser)?)?);
                current = point(node, "Point2", parser)?;
                bounds.point(transform.apply(current)?);
            } else if node.is("CubicBezier") {
                let end = point(node, "Point3", parser)?;
                let control1 = if node.attribute("Point1").is_some() {
                    point(node, "Point1", parser)?
                } else {
                    current
                };
                let control2 = if node.attribute("Point2").is_some() {
                    point(node, "Point2", parser)?
                } else {
                    end
                };
                for point in [control1, control2, end] {
                    bounds.point(transform.apply(point)?);
                }
                current = end;
            } else if node.is("Arc") {
                let end = point(node, "EndPoint", parser)?;
                if end == current {
                    return Err(parser.error(format_args!(
                        "Arc EndPoint must differ from the current point"
                    )));
                }
                let radii = parser.required(node, "EllipseSize")?;
                let mut parsed = Vec::new();
                for token in radii.split_whitespace().take(2) {
                    parsed.extend(numbers(token, 1, parser)?);
                }
                let (rx, ry) = match parsed.as_slice() {
                    [] => (0.0, 0.0),
                    [radius] => (radius.abs(), radius.abs()),
                    [rx, ry] => (rx.abs(), ry.abs()),
                    _ => unreachable!("at most two radii"),
                };
                let angle = numbers(parser.required(node, "RotationAngle")?, 1, parser)?;
                let angle = *angle
                    .first()
                    .ok_or_else(|| parser.error(format_args!("Arc RotationAngle is empty")))?;
                let large = boolean(node, "LargeArc", parser)?;
                let sweep = boolean(node, "SweepDirection", parser)?;
                arc_bounds(
                    &mut bounds,
                    current,
                    end,
                    rx,
                    ry,
                    angle,
                    large,
                    sweep,
                    transform,
                )?;
                current = end;
            } else if node.is("Close") {
                current = start;
                bounds.point(transform.apply(current)?);
            } else {
                return Err(parser.error(format_args!(
                    "unsupported action Region segment {:?}",
                    node.name.local_name
                )));
            }
        }
        result.push(bounds.finish()?);
    }
    if result.is_empty() {
        return Err(parser.error(format_args!("explicit action Region has no areas")));
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn arc_bounds(
    bounds: &mut Bounds,
    start: Point,
    end: Point,
    mut rx: f64,
    mut ry: f64,
    angle: f64,
    large: bool,
    sweep: bool,
    transform: Transform,
) -> Result<()> {
    bounds.point(transform.apply(end)?);
    if rx == 0.0 || ry == 0.0 {
        return Ok(());
    }
    let (sin, cos) = (angle % 360.0).to_radians().sin_cos();
    let dx = start.x() * 0.5 - end.x() * 0.5;
    let dy = start.y() * 0.5 - end.y() * 0.5;
    let (x, y) = (cos * dx + sin * dy, -sin * dx + cos * dy);
    let scale = (x / rx).hypot(y / ry);
    if scale > 1.0 {
        rx *= scale;
        ry *= scale;
    }
    let (px, py) = (x / rx, y / ry);
    let norm = px * px + py * py;
    let coefficient =
        ((1.0 - norm).max(0.0) / norm).sqrt() * if large == sweep { -1.0 } else { 1.0 };
    let (cx, cy) = ((coefficient * py) * rx, (-coefficient * px) * ry);
    let center = Point::new(
        cos * cx - sin * cy + start.x() * 0.5 + end.x() * 0.5,
        sin * cx + cos * cy + start.y() * 0.5 + end.y() * 0.5,
    )?;
    let center = transform.apply(center)?;
    let (ux, uy) = (rx * cos, rx * sin);
    let (vx, vy) = (-ry * sin, ry * cos);
    let ex =
        (transform.a() * ux + transform.c() * uy).hypot(transform.a() * vx + transform.c() * vy);
    let ey =
        (transform.b() * ux + transform.d() * uy).hypot(transform.b() * vx + transform.d() * vy);
    bounds.point(Point::new(center.x() - ex, center.y() - ey)?);
    bounds.point(Point::new(center.x() + ex, center.y() + ey)?);
    Ok(())
}
