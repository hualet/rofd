use std::ffi::{c_char, CStr};
use std::mem::{align_of, offset_of, size_of, MaybeUninit};
use std::ptr;

use rofd_ffi::{
    rofd_abi_version, rofd_find_options_init, rofd_find_options_t, rofd_library_version,
    rofd_load_options_init, rofd_load_options_t, rofd_rect_t, rofd_render_diagnostic_t,
    rofd_render_options_init, rofd_render_options_t, rofd_renderer_options_init,
    rofd_renderer_options_t, rofd_text_char_t, rofd_text_match_t, ROFD_ABI_VERSION,
    ROFD_DIAGNOSTIC_FONT_FALLBACK, ROFD_DIAGNOSTIC_IMAGE_BORDER_UNSUPPORTED,
    ROFD_DIAGNOSTIC_IMAGE_MASK_UNSUPPORTED, ROFD_DIAGNOSTIC_IMAGE_SUBSTITUTION_UNSUPPORTED,
    ROFD_DIAGNOSTIC_MISSING_GLYPH, ROFD_DIAGNOSTIC_UNSUPPORTED_OBJECT, ROFD_FIND_CASE_SENSITIVE,
    ROFD_FIND_WHOLE_WORDS, ROFD_IMAGE_INTERPOLATION_BILINEAR, ROFD_IMAGE_INTERPOLATION_NEAREST,
    ROFD_SELECTION_GLYPH, ROFD_SELECTION_LINE, ROFD_SELECTION_WORD, ROFD_STATUS_INTERNAL,
    ROFD_STATUS_INVALID_ARGUMENT, ROFD_STATUS_INVALID_DOCUMENT, ROFD_STATUS_IO,
    ROFD_STATUS_LIMIT_EXCEEDED, ROFD_STATUS_OK, ROFD_STATUS_OUT_OF_MEMORY,
    ROFD_STATUS_PAGE_OUT_OF_RANGE, ROFD_STATUS_RENDER_ERROR, ROFD_STATUS_UNSUPPORTED,
    ROFD_STRICTNESS_LENIENT, ROFD_STRICTNESS_STRICT, ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY,
    ROFD_TEXT_CHAR_SYNTHESIZED_SEPARATOR,
};

const LOAD_OPTIONS_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_load_options_t, strictness) + size_of::<u32>(),
    align_of::<u32>(),
);
const RENDERER_OPTIONS_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_renderer_options_t, image_cache_bytes) + size_of::<u64>(),
    max_alignment(&[
        align_of::<u32>(),
        align_of::<*const *const c_char>(),
        align_of::<usize>(),
        align_of::<u64>(),
    ]),
);
const RENDER_OPTIONS_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_render_options_t, max_raster_bytes) + size_of::<u64>(),
    max_alignment(&[align_of::<u32>(), align_of::<f64>(), align_of::<u64>()]),
);
const FIND_OPTIONS_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_find_options_t, max_results) + size_of::<usize>(),
    max_alignment(&[align_of::<u32>(), align_of::<usize>()]),
);
const TEXT_CHAR_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_text_char_t, object_id) + size_of::<u64>(),
    max_alignment(&[
        align_of::<u32>(),
        align_of::<usize>(),
        align_of::<rofd_rect_t>(),
        align_of::<u64>(),
    ]),
);
const TEXT_MATCH_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_text_match_t, rect_mm) + size_of::<rofd_rect_t>(),
    max_alignment(&[
        align_of::<u32>(),
        align_of::<usize>(),
        align_of::<rofd_rect_t>(),
    ]),
);

#[test]
fn numeric_constants_match_the_c_header() {
    assert_eq!(size_of::<rofd_ffi::rofd_status_t>(), 4);
    assert_eq!(ROFD_ABI_VERSION, 1);

    assert_eq!(ROFD_STATUS_OK, 0);
    assert_eq!(ROFD_STATUS_INVALID_ARGUMENT, 1);
    assert_eq!(ROFD_STATUS_IO, 2);
    assert_eq!(ROFD_STATUS_INVALID_DOCUMENT, 3);
    assert_eq!(ROFD_STATUS_UNSUPPORTED, 4);
    assert_eq!(ROFD_STATUS_LIMIT_EXCEEDED, 5);
    assert_eq!(ROFD_STATUS_PAGE_OUT_OF_RANGE, 6);
    assert_eq!(ROFD_STATUS_RENDER_ERROR, 7);
    assert_eq!(ROFD_STATUS_OUT_OF_MEMORY, 8);
    assert_eq!(ROFD_STATUS_INTERNAL, 255);

    assert_eq!(ROFD_STRICTNESS_LENIENT, 0);
    assert_eq!(ROFD_STRICTNESS_STRICT, 1);
    assert_eq!(ROFD_IMAGE_INTERPOLATION_NEAREST, 0);
    assert_eq!(ROFD_IMAGE_INTERPOLATION_BILINEAR, 1);
    assert_eq!(ROFD_FIND_CASE_SENSITIVE, 1 << 0);
    assert_eq!(ROFD_FIND_WHOLE_WORDS, 1 << 1);
    assert_eq!(ROFD_TEXT_CHAR_SYNTHESIZED_SEPARATOR, 1 << 0);
    assert_eq!(ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY, 1 << 1);
    assert_eq!(ROFD_SELECTION_GLYPH, 0);
    assert_eq!(ROFD_SELECTION_WORD, 1);
    assert_eq!(ROFD_SELECTION_LINE, 2);

    assert_eq!(ROFD_DIAGNOSTIC_UNSUPPORTED_OBJECT, 1);
    assert_eq!(ROFD_DIAGNOSTIC_FONT_FALLBACK, 2);
    assert_eq!(ROFD_DIAGNOSTIC_MISSING_GLYPH, 3);
    assert_eq!(ROFD_DIAGNOSTIC_IMAGE_SUBSTITUTION_UNSUPPORTED, 4);
    assert_eq!(ROFD_DIAGNOSTIC_IMAGE_MASK_UNSUPPORTED, 5);
    assert_eq!(ROFD_DIAGNOSTIC_IMAGE_BORDER_UNSUPPORTED, 6);
}

#[test]
fn version_exports_report_the_abi_and_crate_versions() {
    assert_eq!(rofd_abi_version(), 1);

    let version = rofd_library_version();
    assert!(!version.is_null());
    let version = unsafe { CStr::from_ptr(version) };
    assert_eq!(version.to_bytes(), env!("CARGO_PKG_VERSION").as_bytes());
}

#[test]
fn records_have_portable_c_abi_prefixes() {
    assert_eq!(size_of::<rofd_ffi::rofd_status_t>(), size_of::<u32>());
    assert_eq!(offset_of!(rofd_load_options_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_renderer_options_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_render_options_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_find_options_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_rect_t, x_mm), 0);
    assert_eq!(offset_of!(rofd_text_char_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_text_match_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_render_diagnostic_t, struct_size), 0);

    assert!(size_of::<rofd_load_options_t>() <= u32::MAX as usize);
    assert!(size_of::<rofd_renderer_options_t>() <= u32::MAX as usize);
    assert!(size_of::<rofd_render_options_t>() <= u32::MAX as usize);
    assert!(size_of::<rofd_find_options_t>() <= u32::MAX as usize);
    assert!(size_of::<rofd_text_char_t>() <= u32::MAX as usize);
    assert!(size_of::<rofd_text_match_t>() <= u32::MAX as usize);
    assert!(align_of::<rofd_renderer_options_t>() >= align_of::<*const c_char>());
    assert!(align_of::<rofd_render_options_t>() >= align_of::<f64>());
    assert!(align_of::<rofd_find_options_t>() >= align_of::<usize>());
    assert!(align_of::<rofd_text_char_t>() >= align_of::<rofd_rect_t>());
    assert!(align_of::<rofd_text_char_t>() >= align_of::<u64>());
    assert!(align_of::<rofd_text_match_t>() >= align_of::<rofd_rect_t>());
    assert!(align_of::<rofd_render_diagnostic_t>() >= align_of::<u64>());

    assert_eq!(size_of::<rofd_find_options_t>(), FIND_OPTIONS_V1_SIZE);
    assert_eq!(size_of::<rofd_text_char_t>(), TEXT_CHAR_V1_SIZE);
    assert_eq!(size_of::<rofd_text_match_t>(), TEXT_MATCH_V1_SIZE);
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn records_have_the_x86_64_linux_c_layout() {
    assert_eq!(size_of::<rofd_load_options_t>(), 8);
    assert_eq!(align_of::<rofd_load_options_t>(), 4);
    assert_eq!(offset_of!(rofd_load_options_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_load_options_t, strictness), 4);

    assert_eq!(size_of::<rofd_renderer_options_t>(), 40);
    assert_eq!(align_of::<rofd_renderer_options_t>(), 8);
    assert_eq!(offset_of!(rofd_renderer_options_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_renderer_options_t, fallback_families), 8);
    assert_eq!(
        offset_of!(rofd_renderer_options_t, fallback_family_count),
        16
    );
    assert_eq!(offset_of!(rofd_renderer_options_t, max_font_bytes), 24);
    assert_eq!(offset_of!(rofd_renderer_options_t, image_cache_bytes), 32);

    assert_eq!(size_of::<rofd_render_options_t>(), 88);
    assert_eq!(align_of::<rofd_render_options_t>(), 8);
    assert_eq!(offset_of!(rofd_render_options_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_render_options_t, dpi), 8);
    assert_eq!(offset_of!(rofd_render_options_t, scale), 16);
    assert_eq!(offset_of!(rofd_render_options_t, rotation_degrees), 24);
    assert_eq!(offset_of!(rofd_render_options_t, background_rgba), 28);
    assert_eq!(offset_of!(rofd_render_options_t, has_clip), 32);
    assert_eq!(offset_of!(rofd_render_options_t, clip_x_mm), 40);
    assert_eq!(offset_of!(rofd_render_options_t, clip_y_mm), 48);
    assert_eq!(offset_of!(rofd_render_options_t, clip_width_mm), 56);
    assert_eq!(offset_of!(rofd_render_options_t, clip_height_mm), 64);
    assert_eq!(offset_of!(rofd_render_options_t, image_interpolation), 72);
    assert_eq!(offset_of!(rofd_render_options_t, max_raster_bytes), 80);

    assert_eq!(size_of::<rofd_find_options_t>(), 16);
    assert_eq!(align_of::<rofd_find_options_t>(), 8);
    assert_eq!(offset_of!(rofd_find_options_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_find_options_t, flags), 4);
    assert_eq!(offset_of!(rofd_find_options_t, max_results), 8);

    assert_eq!(size_of::<rofd_rect_t>(), 32);
    assert_eq!(align_of::<rofd_rect_t>(), 8);
    assert_eq!(offset_of!(rofd_rect_t, x_mm), 0);
    assert_eq!(offset_of!(rofd_rect_t, y_mm), 8);
    assert_eq!(offset_of!(rofd_rect_t, width_mm), 16);
    assert_eq!(offset_of!(rofd_rect_t, height_mm), 24);

    assert_eq!(size_of::<rofd_text_char_t>(), 72);
    assert_eq!(align_of::<rofd_text_char_t>(), 8);
    assert_eq!(offset_of!(rofd_text_char_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_text_char_t, utf8_offset), 8);
    assert_eq!(offset_of!(rofd_text_char_t, utf8_length), 16);
    assert_eq!(offset_of!(rofd_text_char_t, rect_mm), 24);
    assert_eq!(offset_of!(rofd_text_char_t, flags), 56);
    assert_eq!(offset_of!(rofd_text_char_t, object_id), 64);

    assert_eq!(size_of::<rofd_text_match_t>(), 56);
    assert_eq!(align_of::<rofd_text_match_t>(), 8);
    assert_eq!(offset_of!(rofd_text_match_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_text_match_t, utf8_offset), 8);
    assert_eq!(offset_of!(rofd_text_match_t, utf8_length), 16);
    assert_eq!(offset_of!(rofd_text_match_t, rect_mm), 24);

    assert_eq!(size_of::<rofd_render_diagnostic_t>(), 24);
    assert_eq!(align_of::<rofd_render_diagnostic_t>(), 8);
    assert_eq!(offset_of!(rofd_render_diagnostic_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_render_diagnostic_t, kind), 4);
    assert_eq!(offset_of!(rofd_render_diagnostic_t, object_id), 8);
    assert_eq!(offset_of!(rofd_render_diagnostic_t, message), 16);
}

#[test]
fn option_initializers_replace_every_field_with_documented_defaults() {
    let mut load = rofd_load_options_t {
        struct_size: 0,
        strictness: ROFD_STRICTNESS_STRICT,
    };
    let fallback = ptr::NonNull::<*const c_char>::dangling().as_ptr();
    let mut renderer = rofd_renderer_options_t {
        struct_size: 0,
        fallback_families: fallback,
        fallback_family_count: 7,
        max_font_bytes: 1,
        image_cache_bytes: 2,
    };
    let mut render = rofd_render_options_t {
        struct_size: 0,
        dpi: 1.0,
        scale: 2.0,
        rotation_degrees: 90,
        background_rgba: 0,
        has_clip: 1,
        clip_x_mm: 1.0,
        clip_y_mm: 2.0,
        clip_width_mm: 3.0,
        clip_height_mm: 4.0,
        image_interpolation: ROFD_IMAGE_INTERPOLATION_NEAREST,
        max_raster_bytes: 3,
    };
    let mut find = rofd_find_options_t {
        struct_size: 0,
        flags: ROFD_FIND_CASE_SENSITIVE | ROFD_FIND_WHOLE_WORDS,
        max_results: 1,
    };

    unsafe {
        rofd_load_options_init(&mut load, LOAD_OPTIONS_V1_SIZE);
        rofd_renderer_options_init(&mut renderer, RENDERER_OPTIONS_V1_SIZE);
        rofd_render_options_init(&mut render, RENDER_OPTIONS_V1_SIZE);
        rofd_find_options_init(&mut find, FIND_OPTIONS_V1_SIZE);
    }

    assert_eq!(load.struct_size, LOAD_OPTIONS_V1_SIZE as u32);
    assert_eq!(load.strictness, ROFD_STRICTNESS_LENIENT);

    assert_eq!(renderer.struct_size, RENDERER_OPTIONS_V1_SIZE as u32);
    assert!(renderer.fallback_families.is_null());
    assert_eq!(renderer.fallback_family_count, 0);
    assert_eq!(renderer.max_font_bytes, 64 * 1024 * 1024);
    assert_eq!(renderer.image_cache_bytes, 64 * 1024 * 1024);

    assert_eq!(render.struct_size, RENDER_OPTIONS_V1_SIZE as u32);
    assert_eq!(render.dpi, 96.0);
    assert_eq!(render.scale, 1.0);
    assert_eq!(render.rotation_degrees, 0);
    assert_eq!(render.background_rgba, 0xffff_ffff);
    assert_eq!(render.has_clip, 0);
    assert_eq!(render.clip_x_mm, 0.0);
    assert_eq!(render.clip_y_mm, 0.0);
    assert_eq!(render.clip_width_mm, 0.0);
    assert_eq!(render.clip_height_mm, 0.0);
    assert_eq!(
        render.image_interpolation,
        ROFD_IMAGE_INTERPOLATION_BILINEAR
    );
    assert_eq!(render.max_raster_bytes, 256 * 1024 * 1024);

    assert_eq!(find.struct_size, FIND_OPTIONS_V1_SIZE as u32);
    assert_eq!(find.flags, 0);
    assert_eq!(find.max_results, 10_000);
}

#[test]
fn all_option_initializers_are_null_safe() {
    unsafe {
        rofd_load_options_init(ptr::null_mut(), usize::MAX);
        rofd_renderer_options_init(ptr::null_mut(), usize::MAX);
        rofd_render_options_init(ptr::null_mut(), usize::MAX);
        rofd_find_options_init(ptr::null_mut(), usize::MAX);
    }
}

#[test]
fn option_initializers_leave_undersized_buffers_untouched() {
    let mut load = MaybeUninit::<rofd_load_options_t>::uninit();
    let mut renderer = MaybeUninit::<rofd_renderer_options_t>::uninit();
    let mut render = MaybeUninit::<rofd_render_options_t>::uninit();
    let mut find = MaybeUninit::<rofd_find_options_t>::uninit();

    unsafe {
        fill_bytes(load.as_mut_ptr(), 0xa5);
        fill_bytes(renderer.as_mut_ptr(), 0xa5);
        fill_bytes(render.as_mut_ptr(), 0xa5);
        fill_bytes(find.as_mut_ptr(), 0xa5);

        rofd_load_options_init(load.as_mut_ptr(), LOAD_OPTIONS_V1_SIZE - 1);
        rofd_renderer_options_init(renderer.as_mut_ptr(), RENDERER_OPTIONS_V1_SIZE - 1);
        rofd_render_options_init(render.as_mut_ptr(), RENDER_OPTIONS_V1_SIZE - 1);
        rofd_find_options_init(find.as_mut_ptr(), FIND_OPTIONS_V1_SIZE - 1);

        assert!(object_bytes(load.as_ptr()).iter().all(|byte| *byte == 0xa5));
        assert!(object_bytes(renderer.as_ptr())
            .iter()
            .all(|byte| *byte == 0xa5));
        assert!(object_bytes(render.as_ptr())
            .iter()
            .all(|byte| *byte == 0xa5));
        assert!(object_bytes(find.as_ptr()).iter().all(|byte| *byte == 0xa5));
    }
}

#[test]
fn option_initializers_zero_padding_before_writing_fields() {
    let mut load = MaybeUninit::<rofd_load_options_t>::uninit();
    let mut renderer = MaybeUninit::<rofd_renderer_options_t>::uninit();
    let mut render = MaybeUninit::<rofd_render_options_t>::uninit();
    let mut find = MaybeUninit::<rofd_find_options_t>::uninit();

    unsafe {
        fill_bytes(load.as_mut_ptr(), 0xa5);
        fill_bytes(renderer.as_mut_ptr(), 0xa5);
        fill_bytes(render.as_mut_ptr(), 0xa5);
        fill_bytes(find.as_mut_ptr(), 0xa5);

        rofd_load_options_init(load.as_mut_ptr(), LOAD_OPTIONS_V1_SIZE);
        rofd_renderer_options_init(renderer.as_mut_ptr(), RENDERER_OPTIONS_V1_SIZE);
        rofd_render_options_init(render.as_mut_ptr(), RENDER_OPTIONS_V1_SIZE);
        rofd_find_options_init(find.as_mut_ptr(), FIND_OPTIONS_V1_SIZE);

        assert_padding_zero(
            load.as_ptr(),
            &[
                (
                    offset_of!(rofd_load_options_t, struct_size),
                    size_of::<u32>(),
                ),
                (
                    offset_of!(rofd_load_options_t, strictness),
                    size_of::<u32>(),
                ),
            ],
        );
        assert_padding_zero(
            renderer.as_ptr(),
            &[
                (
                    offset_of!(rofd_renderer_options_t, struct_size),
                    size_of::<u32>(),
                ),
                (
                    offset_of!(rofd_renderer_options_t, fallback_families),
                    size_of::<*const *const c_char>(),
                ),
                (
                    offset_of!(rofd_renderer_options_t, fallback_family_count),
                    size_of::<usize>(),
                ),
                (
                    offset_of!(rofd_renderer_options_t, max_font_bytes),
                    size_of::<u64>(),
                ),
                (
                    offset_of!(rofd_renderer_options_t, image_cache_bytes),
                    size_of::<u64>(),
                ),
            ],
        );
        assert_padding_zero(
            render.as_ptr(),
            &[
                (
                    offset_of!(rofd_render_options_t, struct_size),
                    size_of::<u32>(),
                ),
                (offset_of!(rofd_render_options_t, dpi), size_of::<f64>()),
                (offset_of!(rofd_render_options_t, scale), size_of::<f64>()),
                (
                    offset_of!(rofd_render_options_t, rotation_degrees),
                    size_of::<u32>(),
                ),
                (
                    offset_of!(rofd_render_options_t, background_rgba),
                    size_of::<u32>(),
                ),
                (
                    offset_of!(rofd_render_options_t, has_clip),
                    size_of::<u32>(),
                ),
                (
                    offset_of!(rofd_render_options_t, clip_x_mm),
                    size_of::<f64>(),
                ),
                (
                    offset_of!(rofd_render_options_t, clip_y_mm),
                    size_of::<f64>(),
                ),
                (
                    offset_of!(rofd_render_options_t, clip_width_mm),
                    size_of::<f64>(),
                ),
                (
                    offset_of!(rofd_render_options_t, clip_height_mm),
                    size_of::<f64>(),
                ),
                (
                    offset_of!(rofd_render_options_t, image_interpolation),
                    size_of::<u32>(),
                ),
                (
                    offset_of!(rofd_render_options_t, max_raster_bytes),
                    size_of::<u64>(),
                ),
            ],
        );
        assert_padding_zero(
            find.as_ptr(),
            &[
                (
                    offset_of!(rofd_find_options_t, struct_size),
                    size_of::<u32>(),
                ),
                (offset_of!(rofd_find_options_t, flags), size_of::<u32>()),
                (
                    offset_of!(rofd_find_options_t, max_results),
                    size_of::<usize>(),
                ),
            ],
        );
    }
}

#[repr(C)]
struct Extended<T> {
    options: T,
    tail: [u8; 16],
}

#[test]
fn option_initializers_preserve_oversized_record_tails() {
    let mut load = MaybeUninit::<Extended<rofd_load_options_t>>::uninit();
    let mut renderer = MaybeUninit::<Extended<rofd_renderer_options_t>>::uninit();
    let mut render = MaybeUninit::<Extended<rofd_render_options_t>>::uninit();
    let mut find = MaybeUninit::<Extended<rofd_find_options_t>>::uninit();

    unsafe {
        fill_bytes(load.as_mut_ptr(), 0xa5);
        fill_bytes(renderer.as_mut_ptr(), 0xa5);
        fill_bytes(render.as_mut_ptr(), 0xa5);
        fill_bytes(find.as_mut_ptr(), 0xa5);

        rofd_load_options_init(
            ptr::addr_of_mut!((*load.as_mut_ptr()).options),
            LOAD_OPTIONS_V1_SIZE + size_of::<[u8; 16]>(),
        );
        rofd_renderer_options_init(
            ptr::addr_of_mut!((*renderer.as_mut_ptr()).options),
            RENDERER_OPTIONS_V1_SIZE + size_of::<[u8; 16]>(),
        );
        rofd_render_options_init(
            ptr::addr_of_mut!((*render.as_mut_ptr()).options),
            RENDER_OPTIONS_V1_SIZE + size_of::<[u8; 16]>(),
        );
        rofd_find_options_init(
            ptr::addr_of_mut!((*find.as_mut_ptr()).options),
            FIND_OPTIONS_V1_SIZE + size_of::<[u8; 16]>(),
        );

        let load = load.assume_init();
        let renderer = renderer.assume_init();
        let render = render.assume_init();
        let find = find.assume_init();

        assert_eq!(load.options.struct_size as usize, LOAD_OPTIONS_V1_SIZE);
        assert_eq!(
            renderer.options.struct_size as usize,
            RENDERER_OPTIONS_V1_SIZE
        );
        assert_eq!(render.options.struct_size as usize, RENDER_OPTIONS_V1_SIZE);
        assert_eq!(find.options.struct_size as usize, FIND_OPTIONS_V1_SIZE);
        assert_eq!(load.tail, [0xa5; 16]);
        assert_eq!(renderer.tail, [0xa5; 16]);
        assert_eq!(render.tail, [0xa5; 16]);
        assert_eq!(find.tail, [0xa5; 16]);
    }
}

unsafe fn fill_bytes<T>(value: *mut T, byte: u8) {
    unsafe {
        value.cast::<u8>().write_bytes(byte, size_of::<T>());
    }
}

unsafe fn object_bytes<T>(value: *const T) -> Vec<u8> {
    unsafe { std::slice::from_raw_parts(value.cast::<u8>(), size_of::<T>()).to_vec() }
}

unsafe fn assert_padding_zero<T>(value: *const T, fields: &[(usize, usize)]) {
    let bytes = unsafe { object_bytes(value) };
    for (index, byte) in bytes.into_iter().enumerate() {
        let is_field = fields
            .iter()
            .any(|(offset, size)| (*offset..(*offset + *size)).contains(&index));
        if !is_field {
            assert_eq!(byte, 0, "padding byte {index} was not zero");
        }
    }
}

const fn c_record_size(last_field_end: usize, alignment: usize) -> usize {
    last_field_end.div_ceil(alignment) * alignment
}

const fn max_alignment(alignments: &[usize]) -> usize {
    let mut maximum = 1;
    let mut index = 0;
    while index < alignments.len() {
        if alignments[index] > maximum {
            maximum = alignments[index];
        }
        index += 1;
    }
    maximum
}
