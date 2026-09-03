use rofd_core::{Color, Point, Transform};

#[test]
fn point_accepts_finite_coordinates() {
    let point = Point::new(1.25, -2.5).unwrap();

    assert_eq!(point.x, 1.25);
    assert_eq!(point.y, -2.5);
}

#[test]
fn point_rejects_non_finite_coordinates() {
    assert!(Point::new(f64::NAN, 0.0).is_err());
    assert!(Point::new(0.0, f64::INFINITY).is_err());
    assert!(Point::new(f64::NEG_INFINITY, 0.0).is_err());
}

#[test]
fn affine_transform_maps_points() {
    let matrix = Transform::parse("2 0 0 3 4 5").unwrap();

    assert_eq!(
        matrix.apply(Point::new(1.0, 2.0).unwrap()),
        Point::new(6.0, 11.0).unwrap()
    );
}

#[test]
fn transform_then_applies_the_left_transform_first() {
    let translate = Transform::parse("1 0 0 1 2 4").unwrap();
    let scale = Transform::parse("3 0 0 2 0 0").unwrap();
    let point = Point::new(1.0, 1.0).unwrap();

    assert_eq!(
        translate.then(scale).apply(point),
        Point::new(9.0, 10.0).unwrap()
    );
    assert_eq!(Transform::IDENTITY.apply(point), point);
}

#[test]
fn transform_rejects_invalid_values() {
    assert!(Transform::parse("1 0 0 1 NaN 0").is_err());
    assert!(Transform::parse("1 0 0 1 inf 0").is_err());
    assert!(Transform::parse("1 0 0 1 -inf 0").is_err());
    assert!(Transform::parse("1 0 0 1 0").is_err());
    assert!(Transform::parse("1 0 0 1 0 0 0").is_err());
    assert!(Transform::parse("1 0 0 1 zero 0").is_err());
}

#[test]
fn rgb_color_parses_channels_and_alpha() {
    assert_eq!(
        Color::parse_rgb("12 34 56", None).unwrap(),
        Color {
            red: 12,
            green: 34,
            blue: 56,
            alpha: 255,
        }
    );
    assert_eq!(
        Color::parse_rgb("12 34 56", Some("78")).unwrap(),
        Color {
            red: 12,
            green: 34,
            blue: 56,
            alpha: 78,
        }
    );
    assert_eq!(Color::BLACK, Color::parse_rgb("0 0 0", None).unwrap());
}

#[test]
fn color_rejects_invalid_channels_and_alpha() {
    assert!(Color::parse_rgb("256 0 0", None).is_err());
    assert!(Color::parse_rgb("-1 0 0", None).is_err());
    assert!(Color::parse_rgb("0 0", None).is_err());
    assert!(Color::parse_rgb("0 0 0 0", None).is_err());
    assert!(Color::parse_rgb("NaN 0 0", None).is_err());
    assert!(Color::parse_rgb("inf 0 0", None).is_err());
    assert!(Color::parse_rgb("0 0 0", Some("256")).is_err());
    assert!(Color::parse_rgb("0 0 0", Some("-1")).is_err());
    assert!(Color::parse_rgb("0 0 0", Some("NaN")).is_err());
    assert!(Color::parse_rgb("0 0 0", Some("inf")).is_err());
}
