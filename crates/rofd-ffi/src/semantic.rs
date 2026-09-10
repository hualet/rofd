use crate::error::{
    boundary_with_inputs, FfiError, HandleOutput, InputRanges, OutputSet, ScalarOutput,
    TextCharFields, TextCharOutput,
};
use crate::handles::{
    drop_raw_handle, handle_ref, HandleToken, PageHandle, StringHandle, TextLayoutHandle,
};
use crate::{
    rofd_error_t, rofd_page_t, rofd_rect_t, rofd_status_t, rofd_string_t, rofd_text_char_t,
    rofd_text_layout_t, ROFD_STATUS_INTERNAL, ROFD_STATUS_PAGE_OUT_OF_RANGE,
    ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY, ROFD_TEXT_CHAR_SYNTHESIZED_SEPARATOR,
};
use rofd_core::{Rect, TextChar, TextCharFlags, TextGeometryPrecision};
use std::ffi::{c_char, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

fn handle_inputs<Token: HandleToken>(handle: *const Token) -> Result<InputRanges, ()> {
    let mut inputs = InputRanges::new();
    inputs.push(handle.cast::<Token::Storage>())?;
    Ok(inputs)
}

unsafe fn page_ref<'a>(page: *const rofd_page_t) -> Result<&'a PageHandle, FfiError> {
    if page.is_null() {
        return Err(FfiError::invalid_argument("page handle is NULL"));
    }
    // SAFETY: The caller supplies a live immutable page token for this borrow.
    Ok(unsafe { handle_ref(page) })
}

unsafe fn layout_ref<'a>(
    layout: *const rofd_text_layout_t,
) -> Result<&'a TextLayoutHandle, FfiError> {
    if layout.is_null() {
        return Err(FfiError::invalid_argument("text layout handle is NULL"));
    }
    // SAFETY: The caller supplies a live immutable layout token for this borrow.
    Ok(unsafe { handle_ref(layout) })
}

fn owned_string(text: &str) -> Result<Box<StringHandle>, FfiError> {
    let bytes = CString::new(text.replace('\0', "\\0"))
        .expect("replacing embedded NUL bytes must produce a valid C string");
    Ok(Box::new(StringHandle { bytes }))
}

// SAFETY: Callers forward valid output storage and immutable input regions.
// Invalid input address layouts use the existing empty-input error transaction.
unsafe fn semantic_boundary<Outputs: OutputSet>(
    error: *mut *mut rofd_error_t,
    outputs: Outputs,
    inputs: Result<InputRanges, ()>,
    operation: impl FnOnce() -> Result<Outputs::Staged, FfiError>,
) -> rofd_status_t {
    match inputs {
        // SAFETY: All writes and reads occur only after the common boundary preflight.
        Ok(inputs) => unsafe { boundary_with_inputs(error, outputs, inputs, operation) },
        // SAFETY: The normal boundary can safely publish a malformed-input failure.
        Err(()) => unsafe {
            boundary_with_inputs(error, outputs, InputRanges::new(), || {
                Err(FfiError::invalid_argument(
                    "input storage is misaligned or overflows",
                ))
            })
        },
    }
}

/// Returns independently owned canonical UTF-8 page text, including separators.
///
/// The string survives freeing the page and document; release it with
/// [`rofd_string_free`]. Layout offsets index these exact bytes.
///
/// # Safety
/// `page` must be null or a live immutable page handle. `text` is required and
/// must be writable and aligned when non-null; `error` is optional. Every output
/// must be mutually disjoint and disjoint from page storage, live for the call,
/// and not accessed concurrently. Null required arguments return invalid argument.
#[no_mangle]
pub unsafe extern "C" fn rofd_page_get_text(
    page: *const rofd_page_t,
    text: *mut *mut rofd_string_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    // SAFETY: The common boundary preflights page storage before writes or borrowing.
    unsafe {
        semantic_boundary(
            error,
            HandleOutput::required(text),
            handle_inputs(page),
            || owned_string(page_ref(page)?.inner.text()?.as_str()),
        )
    }
}

/// Returns owned UTF-8 source glyphs intersecting an area in physical-page mm.
///
/// Positive-area intersections are emitted in canonical order, without
/// synthesized separators. Empty areas return empty strings. Nonfinite values
/// or edges and negative dimensions return invalid argument.
///
/// # Safety
/// `page` must be null or a live immutable page. Non-null `area_mm` must be an
/// aligned, complete initialized readable rectangle. Both arguments and `text`
/// are required. Non-null outputs must be aligned, writable, mutually disjoint,
/// and disjoint from the entire rectangle and page storage. All storage must
/// remain live without conflicting concurrent access for the call.
#[no_mangle]
pub unsafe extern "C" fn rofd_page_get_text_for_area(
    page: *const rofd_page_t,
    area_mm: *const rofd_rect_t,
    text: *mut *mut rofd_string_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let inputs = handle_inputs(page).and_then(|mut inputs| {
        inputs.push(area_mm)?;
        Ok(inputs)
    });
    // SAFETY: Input/output disjointness is checked before reading the rectangle or page.
    unsafe {
        semantic_boundary(error, HandleOutput::required(text), inputs, || {
            let page = page_ref(page)?;
            let area = area_mm
                .as_ref()
                .ok_or_else(|| FfiError::invalid_argument("area is NULL"))?;
            let text = page.inner.text()?.text_for_area(Rect {
                x: area.x_mm,
                y: area.y_mm,
                width: area.width_mm,
                height: area.height_mm,
            })?;
            owned_string(&text)
        })
    }
}

/// Borrows a string's NUL-terminated canonical UTF-8 bytes; null returns null.
///
/// # Safety
/// A non-null handle must be live and immutable. The returned pointer must not
/// be written to and remains valid only until [`rofd_string_free`].
#[no_mangle]
pub unsafe extern "C" fn rofd_string_get_data(text: *const rofd_string_t) -> *const c_char {
    catch_unwind(AssertUnwindSafe(|| {
        if text.is_null() {
            return ptr::null();
        }
        // SAFETY: The caller guarantees a live immutable string for the borrow.
        unsafe { handle_ref(text).bytes.as_ptr() }
    }))
    .unwrap_or(ptr::null())
}

/// Returns the UTF-8 byte length excluding the terminating NUL; null returns zero.
///
/// # Safety
/// A non-null handle must be live, immutable and not freed during this call.
#[no_mangle]
pub unsafe extern "C" fn rofd_string_get_length(text: *const rofd_string_t) -> usize {
    catch_unwind(AssertUnwindSafe(|| {
        if text.is_null() {
            return 0;
        }
        // SAFETY: The caller guarantees a live immutable string for the borrow.
        unsafe { handle_ref(text).bytes.as_bytes().len() }
    }))
    .unwrap_or(0)
}

/// Frees an owned string; null is a no-op. Borrowed data becomes invalid.
///
/// # Safety
/// A non-null handle must be live and uniquely owned, freed exactly once, and
/// have no concurrent readers or outstanding uses of its borrowed data.
#[no_mangle]
pub unsafe extern "C" fn rofd_string_free(text: *mut rofd_string_t) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !text.is_null() {
            // SAFETY: The caller transfers the unique live allocation exactly once.
            unsafe { drop_raw_handle(text) };
        }
    }));
}

/// Returns an owned layout snapshot, one record per canonical Unicode scalar.
///
/// The snapshot survives freeing the page and document. Offsets index the exact
/// canonical bytes returned by [`rofd_page_get_text`].
///
/// # Safety
/// `page` must be null or a live immutable page. `layout` is required; `error` is
/// optional. Non-null outputs must be aligned, writable, mutually disjoint and
/// disjoint from page storage, and not accessed concurrently during the call.
#[no_mangle]
pub unsafe extern "C" fn rofd_page_get_text_layout(
    page: *const rofd_page_t,
    layout: *mut *mut rofd_text_layout_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    // SAFETY: The boundary checks page storage against every output before writes.
    unsafe {
        semantic_boundary(
            error,
            HandleOutput::required(layout),
            handle_inputs(page),
            || {
                Ok(Box::new(TextLayoutHandle {
                    characters: page_ref(page)?.inner.text()?.characters().to_vec(),
                }))
            },
        )
    }
}

/// Returns the number of Unicode scalars, including synthesized separators.
///
/// # Safety
/// `layout` must be null or a live immutable layout. `count` is required and
/// `error` optional; non-null outputs must be aligned, writable, mutually
/// disjoint and disjoint from live layout storage, without concurrent access.
#[no_mangle]
pub unsafe extern "C" fn rofd_text_layout_get_count(
    layout: *const rofd_text_layout_t,
    count: *mut usize,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    // SAFETY: Preflight protects live layout storage before the scalar transaction.
    unsafe {
        semantic_boundary(
            error,
            ScalarOutput::required(count),
            handle_inputs(layout),
            || Ok(layout_ref(layout)?.characters.len()),
        )
    }
}

fn text_char_fields(character: &TextChar) -> Result<TextCharFields, FfiError> {
    let range = character.utf8_range();
    let utf8_length = range.end.checked_sub(range.start).ok_or_else(|| {
        FfiError::new(
            ROFD_STATUS_INTERNAL,
            "canonical character range is reversed",
        )
    })?;
    let rect = character.rect_mm().unwrap_or(Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    });
    let mut flags = 0;
    if character
        .flags()
        .contains(TextCharFlags::SYNTHESIZED_SEPARATOR)
    {
        flags |= ROFD_TEXT_CHAR_SYNTHESIZED_SEPARATOR;
    }
    if character.geometry_precision() == TextGeometryPrecision::Conservative {
        flags |= ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY;
    }
    Ok(TextCharFields {
        utf8_offset: range.start,
        utf8_length,
        rect_mm: rofd_rect_t {
            x_mm: rect.x,
            y_mm: rect.y,
            width_mm: rect.width,
            height_mm: rect.height,
        },
        flags,
        object_id: character.object_id().unwrap_or(0),
    })
}

/// Returns a character record by index; out-of-range returns PAGE_OUT_OF_RANGE.
///
/// Set `character->struct_size` to at least the complete v1 record size. The
/// transaction clears the permanent v1 prefix including padding, restores the
/// declared size, and preserves unknown tail bytes. Null or undersized records
/// remain untouched and return invalid argument. No-geometry characters have a
/// zero rectangle; synthesized separators additionally have object ID zero.
///
/// # Safety
/// `layout` must be null or a live immutable layout. Non-null `character` must
/// have a readable initialized u32 size field; a supported record must be
/// aligned and writable for its complete v1 prefix. Outputs including optional
/// `error` must be mutually disjoint and disjoint from layout storage, remain
/// live, and not be accessed concurrently for the call.
#[no_mangle]
pub unsafe extern "C" fn rofd_text_layout_get_char(
    layout: *const rofd_text_layout_t,
    index: usize,
    character: *mut rofd_text_char_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    // SAFETY: The caller supplies a readable size prefix before transaction entry.
    let output = unsafe { TextCharOutput::required(character) };
    // SAFETY: The boundary validates record layout and input/output disjointness.
    unsafe {
        semantic_boundary(error, output, handle_inputs(layout), || {
            let character = layout_ref(layout)?.characters.get(index).ok_or_else(|| {
                FfiError::new(
                    ROFD_STATUS_PAGE_OUT_OF_RANGE,
                    format!("character index {index} is out of range"),
                )
            })?;
            text_char_fields(character)
        })
    }
}

/// Frees an owned layout snapshot. Null is a no-op.
///
/// # Safety
/// A non-null handle must be live and uniquely owned, not used concurrently,
/// and freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn rofd_text_layout_free(layout: *mut rofd_text_layout_t) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !layout.is_null() {
            // SAFETY: The caller transfers the unique live allocation exactly once.
            unsafe { drop_raw_handle(layout) };
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::owned_string;

    #[test]
    fn embedded_nul_is_made_visible() {
        let Ok(string) = owned_string("left\0right") else {
            panic!("embedded NUL should be escaped");
        };
        assert_eq!(string.bytes.to_bytes(), b"left\\0right");
    }
}
