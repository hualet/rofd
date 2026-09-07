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
    assert_eq!(options.limits.max_xml_depth, 256);
    assert_eq!(options.limits.max_resource_files, 32);
    assert_eq!(options.limits.max_resources, 100_000);
    assert_eq!(options.limits.max_font_bytes, 64 * 1024 * 1024);
    assert_eq!(options.limits.max_encoded_image_bytes, 64 * 1024 * 1024);
    assert_eq!(options.limits.max_decoded_image_pixels, 100_000_000);
    assert_eq!(options.limits.max_decoded_image_bytes, 400 * 1024 * 1024);
    assert_eq!(options.limits.max_text_characters_per_page, 1_000_000);
    assert_eq!(options.limits.max_glyphs_per_page, 1_000_000);
    assert_eq!(options.limits.max_text_expansion_entries, 2_000_000);
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

#[test]
fn rect_rejects_unicode_format_characters_in_numbers() {
    // U+202C (pop directional formatting) shows up in files produced by
    // rich-text tooling; it is not a number and not skipped as whitespace.
    assert!(Rect::parse("0 0 15\u{202c} 10").is_err());
    assert!(Rect::parse("0 0 \u{202c}210 297").is_err());
}
