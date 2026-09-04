use rofd_core::{Error, PathCommand, PathData, Point};

fn point(x: f64, y: f64) -> Point {
    Point::new(x, y).unwrap()
}

fn assert_invalid(value: &str) {
    assert!(matches!(
        PathData::parse(value).unwrap_err(),
        Error::InvalidValue {
            field: "path data",
            value: offending,
            ..
        } if offending == value
    ));
}

#[test]
fn parses_every_standard_path_command_in_source_order() {
    let path =
        PathData::parse("M 0 1 L 2 3 Q 4 5 6 7 B 8 9 10 11 12 13 A 14 15 16 1 0 17 18 C").unwrap();

    assert_eq!(
        path.commands(),
        &[
            PathCommand::MoveTo(point(0.0, 1.0)),
            PathCommand::LineTo(point(2.0, 3.0)),
            PathCommand::QuadraticTo {
                control: point(4.0, 5.0),
                end: point(6.0, 7.0),
            },
            PathCommand::CubicTo {
                control1: point(8.0, 9.0),
                control2: point(10.0, 11.0),
                end: point(12.0, 13.0),
            },
            PathCommand::ArcTo {
                rx: 14.0,
                ry: 15.0,
                rotation: 16.0,
                large: true,
                sweep: false,
                end: point(17.0, 18.0),
            },
            PathCommand::Close,
        ]
    );
}

#[test]
fn parses_multiple_subpaths_and_letters_adjacent_to_numbers() {
    let path = PathData::parse("M0 0L200 0 C M-2.5 3L4 -5C").unwrap();

    assert_eq!(
        path.commands(),
        &[
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::LineTo(point(200.0, 0.0)),
            PathCommand::Close,
            PathCommand::MoveTo(point(-2.5, 3.0)),
            PathCommand::LineTo(point(4.0, -5.0)),
            PathCommand::Close,
        ]
    );
}

#[test]
fn parses_negative_decimal_and_exponent_numbers() {
    let path = PathData::parse("M -1.25 +2e1 L 3E-2 -.5").unwrap();

    assert_eq!(
        path.commands(),
        &[
            PathCommand::MoveTo(point(-1.25, 20.0)),
            PathCommand::LineTo(point(0.03, -0.5)),
        ]
    );
}

#[test]
fn rejects_empty_input_and_malformed_arities() {
    for value in [
        "",
        "   ",
        "M",
        "M 0",
        "M 0 0 L 1",
        "M 0 0 Q 1 2 3",
        "M 0 0 B 1 2 3 4 5",
        "M 0 0 A 1 2 3 0 1 4",
        "M 0 0 C 1",
        "M 0 0 1 2",
    ] {
        assert_invalid(value);
    }
}

#[test]
fn rejects_unknown_commands_and_malformed_numbers() {
    for value in ["M 0 0 Z", "m 0 0", "M 0 nope", "M 1e 0", "M . 0"] {
        assert_invalid(value);
    }
}

#[test]
fn rejects_non_finite_numbers_and_negative_arc_radii() {
    for value in [
        "M NaN 0",
        "M inf 0",
        "M -inf 0",
        "M 1e309 0",
        "M 0 0 A -1 2 0 0 1 3 4",
        "M 0 0 A 1 -2 0 0 1 3 4",
    ] {
        assert_invalid(value);
    }
}

#[test]
fn rejects_arc_flags_other_than_numeric_zero_or_one() {
    for value in [
        "M 0 0 A 1 2 0 2 0 3 4",
        "M 0 0 A 1 2 0 0 -1 3 4",
        "M 0 0 A 1 2 0 1.0 0 3 4",
    ] {
        assert_invalid(value);
    }
}

#[test]
fn rejects_drawing_commands_before_the_first_move() {
    for value in [
        "L 0 0",
        "Q 0 0 1 1",
        "B 0 0 1 1 2 2",
        "A 1 1 0 0 0 2 2",
        "C",
    ] {
        assert_invalid(value);
    }
}

#[test]
fn enforces_the_command_limit_before_adding_repeated_close_commands() {
    let value = "M 0 0 C C C";
    let error = PathData::parse_with_limit(value, 3).unwrap_err();

    assert!(matches!(
        error,
        Error::LimitExceeded(message)
            if message.contains("path command count 4") && message.contains("limit 3")
    ));

    for nonempty_path in ["M 0 0", "M"] {
        let zero_limit = PathData::parse_with_limit(nonempty_path, 0).unwrap_err();
        assert!(matches!(
            zero_limit,
            Error::LimitExceeded(message)
                if message.contains("path command count 1") && message.contains("limit 0")
        ));
    }
}

#[test]
fn signed_numbers_may_start_without_whitespace_after_a_number() {
    for (value, expected) in [("M1-2", point(1.0, -2.0)), ("M1e-2+3", point(0.01, 3.0))] {
        assert_eq!(
            PathData::parse(value).unwrap().commands(),
            &[PathCommand::MoveTo(expected)]
        );
    }
}

#[test]
fn rejects_doubled_signs_and_malformed_exponents() {
    for value in [
        "M--1 2", "M+-1 2", "M1e--2 3", "M1e+-2 3", "M1ee2 3", "M1e+ 3",
    ] {
        assert_invalid(value);
    }
}

#[test]
fn accepts_all_ascii_whitespace_defined_by_xml() {
    let path = PathData::parse("\tM\r1\n2 L 3\t4\r\n").unwrap();

    assert_eq!(
        path.commands(),
        &[
            PathCommand::MoveTo(point(1.0, 2.0)),
            PathCommand::LineTo(point(3.0, 4.0)),
        ]
    );
}

#[test]
fn close_preserves_the_active_subpath_for_following_commands() {
    let path = PathData::parse("M0 0 C C L1 1 M2 2 C").unwrap();

    assert_eq!(
        path.commands(),
        &[
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::Close,
            PathCommand::Close,
            PathCommand::LineTo(point(1.0, 1.0)),
            PathCommand::MoveTo(point(2.0, 2.0)),
            PathCommand::Close,
        ]
    );
}
