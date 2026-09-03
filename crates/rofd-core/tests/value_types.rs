use rofd_core::{Error, LoadOptions, Rect, Strictness};

#[test]
fn default_limits_are_finite_and_non_zero() {
    let options = LoadOptions::default();
    assert_eq!(options.strictness, Strictness::Lenient);
    assert!(options.limits.max_entries > 0);
    assert!(options.limits.max_entry_size > 0);
    assert!(options.limits.max_total_size >= options.limits.max_entry_size);
    assert!(options.limits.max_path_commands > 0);
    assert_eq!(options.limits.max_page_objects, 100_000);
    assert_eq!(options.limits.max_page_block_depth, 64);
}

#[test]
fn rect_rejects_invalid_numbers() {
    let error = Rect::parse("0 0 NaN 297").unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidValue {
            field: "rectangle",
            ..
        }
    ));
}

#[test]
fn rect_requires_exactly_four_values() {
    assert!(Rect::parse("0 0 210").is_err());
    assert!(Rect::parse("0 0 210 297 1").is_err());
}
