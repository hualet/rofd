use crate::abi::ROFD_FIND_OPTIONS_V1_SIZE;
use crate::error::{
    boundary_with_inputs, FfiError, HandleOutput, InputRanges, OutputSet, ScalarOutput,
    TextCharFields, TextCharOutput, TextMatchFields, TextMatchOutput,
};
use crate::handles::{
    drop_raw_handle, handle_ref, HandleToken, PageHandle, StringHandle, TextLayoutHandle,
    TextSearchHandle, TextSelectionHandle,
};
use crate::{
    rofd_error_t, rofd_find_options_t, rofd_page_t, rofd_rect_t, rofd_status_t, rofd_string_t,
    rofd_text_char_t, rofd_text_layout_t, rofd_text_match_t, rofd_text_search_t,
    rofd_text_selection_t, ROFD_FIND_CASE_SENSITIVE, ROFD_FIND_WHOLE_WORDS, ROFD_SELECTION_GLYPH,
    ROFD_SELECTION_LINE, ROFD_SELECTION_WORD, ROFD_STATUS_INTERNAL, ROFD_STATUS_PAGE_OUT_OF_RANGE,
    ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY, ROFD_TEXT_CHAR_SYNTHESIZED_SEPARATOR,
};
use rofd_core::{
    FindOptions, Rect, SelectionStyle, TextChar, TextCharFlags, TextGeometryPrecision, TextMatch,
};
use std::ffi::{c_char, CStr, CString};
use std::mem::{align_of, size_of};
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

unsafe fn search_ref<'a>(
    search: *const rofd_text_search_t,
) -> Result<&'a TextSearchHandle, FfiError> {
    if search.is_null() {
        return Err(FfiError::invalid_argument("text search handle is NULL"));
    }
    // SAFETY: The caller supplies a live immutable search token for this borrow.
    Ok(unsafe { handle_ref(search) })
}

unsafe fn selection_ref<'a>(
    selection: *const rofd_text_selection_t,
) -> Result<&'a TextSelectionHandle, FfiError> {
    if selection.is_null() {
        return Err(FfiError::invalid_argument("text selection handle is NULL"));
    }
    // SAFETY: The caller supplies a live immutable selection token for this borrow.
    Ok(unsafe { handle_ref(selection) })
}

fn c_string(text: &str) -> CString {
    CString::new(text.replace('\0', "\\0"))
        .expect("replacing embedded NUL bytes must produce a valid C string")
}

fn owned_string(text: &str) -> Result<Box<StringHandle>, FfiError> {
    Ok(Box::new(StringHandle {
        bytes: c_string(text),
    }))
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

struct FindOptionsInput;

impl FindOptionsInput {
    unsafe fn from_ffi(
        options: *const rofd_find_options_t,
        result_limit: usize,
    ) -> Result<FindOptions, FfiError> {
        if options.is_null() {
            let mut options = FindOptions::default();
            options.max_results = options.max_results.min(result_limit);
            return Ok(options);
        }
        // SAFETY: Input preflight validated a readable, aligned size field.
        let struct_size = unsafe { (*options).struct_size as usize };
        if struct_size < ROFD_FIND_OPTIONS_V1_SIZE {
            return Err(FfiError::invalid_argument(format!(
                "find options struct_size {struct_size} is smaller than v1 size {ROFD_FIND_OPTIONS_V1_SIZE}"
            )));
        }
        // SAFETY: The accepted size covers the complete permanent v1 prefix.
        let flags = unsafe { (*options).flags };
        let known_flags = ROFD_FIND_CASE_SENSITIVE | ROFD_FIND_WHOLE_WORDS;
        if flags & !known_flags != 0 {
            return Err(FfiError::invalid_argument(format!(
                "unknown text search flags {:#x}",
                flags & !known_flags
            )));
        }
        // SAFETY: The accepted size covers the complete permanent v1 prefix.
        let max_results = unsafe { (*options).max_results };
        if max_results == 0 {
            return Err(FfiError::invalid_argument(
                "find options max_results must be positive",
            ));
        }
        Ok(FindOptions {
            case_sensitive: flags & ROFD_FIND_CASE_SENSITIVE != 0,
            whole_words: flags & ROFD_FIND_WHOLE_WORDS != 0,
            max_results: max_results.min(result_limit),
        })
    }
}

unsafe fn find_input_ranges(
    page: *const rofd_page_t,
    query: *const c_char,
    options: *const rofd_find_options_t,
) -> Result<InputRanges, ()> {
    let mut inputs = handle_inputs(page)?;
    if !query.is_null() {
        // SAFETY: The caller promises a readable NUL-terminated character array.
        let query_size = unsafe { CStr::from_ptr(query) }.to_bytes_with_nul().len();
        inputs.push_region(query.cast(), query_size, 1)?;
    }
    if !options.is_null() {
        // SAFETY: The caller promises a readable initialized first u32. Use an unaligned read so
        // malformed alignment can be reported by the range preflight rather than invoked here.
        let declared_size = unsafe { options.cast::<u32>().read_unaligned() } as usize;
        let readable_size = if declared_size >= ROFD_FIND_OPTIONS_V1_SIZE {
            ROFD_FIND_OPTIONS_V1_SIZE
        } else {
            size_of::<u32>()
        };
        inputs.push_region(
            options.cast(),
            readable_size,
            align_of::<rofd_find_options_t>(),
        )?;
    }
    Ok(inputs)
}

unsafe fn page_find_text(
    page: *const rofd_page_t,
    query: *const c_char,
    options: *const rofd_find_options_t,
    search: *mut *mut rofd_text_search_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    // SAFETY: Input ranges are inspected only under the caller's readable-input contract.
    let inputs = unsafe { find_input_ranges(page, query, options) };
    // SAFETY: The boundary validates input/output disjointness before initialization or queries.
    unsafe {
        semantic_boundary(error, HandleOutput::required(search), inputs, || {
            let page = page_ref(page)?;
            if query.is_null() {
                return Err(FfiError::invalid_argument("text search query is NULL"));
            }
            // SAFETY: Input preflight completed and the caller keeps the string live for the call.
            let query = CStr::from_ptr(query)
                .to_str()
                .map_err(|_| FfiError::invalid_argument("text search query is not valid UTF-8"))?;
            if query.is_empty() {
                return Err(FfiError::invalid_argument("text search query is empty"));
            }
            let options = FindOptionsInput::from_ffi(
                options,
                page.inner.resource_limits().max_text_characters_per_page,
            )?;
            let matches = page
                .inner
                .text()?
                .find(query, options)
                .map_err(FfiError::from)?;
            Ok(Box::new(TextSearchHandle { matches }))
        })
    }
}

/// Finds text using case-insensitive, non-whole-word defaults.
///
/// # Safety
/// `page` must be null or a live immutable page. `query` must be null or a
/// readable NUL-terminated byte string that remains live for the call. `search`
/// is required and `error` optional; every non-null output must be aligned,
/// writable, mutually disjoint and disjoint from all live input storage.
#[no_mangle]
pub unsafe extern "C" fn rofd_page_find_text(
    page: *const rofd_page_t,
    query: *const c_char,
    search: *mut *mut rofd_text_search_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    // SAFETY: This is the default-options form of the common implementation.
    unsafe { page_find_text(page, query, ptr::null(), search, error) }
}

/// Finds text using a versioned options record.
///
/// Null options select defaults. Unknown flags, a zero result limit, invalid
/// UTF-8 and empty queries return invalid argument. A larger options tail is
/// ignored; result count is clamped to the page's semantic resource limit.
///
/// # Safety
/// The page and query requirements match [`rofd_page_find_text`]. Non-null
/// `options` must be aligned and readable through an initialized `struct_size`;
/// a supported size promises the complete v1 prefix. All inputs and live handle
/// storage must be disjoint from the required `search` and optional `error`
/// outputs, which must also be aligned, writable and mutually disjoint.
#[no_mangle]
pub unsafe extern "C" fn rofd_page_find_text_with_options(
    page: *const rofd_page_t,
    query: *const c_char,
    options: *const rofd_find_options_t,
    search: *mut *mut rofd_text_search_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    // SAFETY: The common implementation owns all preflight and transactional writes.
    unsafe { page_find_text(page, query, options, search, error) }
}

/// Returns the number of owned search matches.
///
/// # Safety
/// `search` must be null or a live immutable handle. `count` is required and
/// `error` optional; non-null outputs must be writable, aligned, mutually
/// disjoint and disjoint from search storage for the complete call.
#[no_mangle]
pub unsafe extern "C" fn rofd_text_search_get_count(
    search: *const rofd_text_search_t,
    count: *mut usize,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    // SAFETY: The boundary protects live search storage before scalar publication.
    unsafe {
        semantic_boundary(
            error,
            ScalarOutput::required(count),
            handle_inputs(search),
            || Ok(search_ref(search)?.matches.len()),
        )
    }
}

fn text_match_fields(found: &TextMatch) -> Result<TextMatchFields, FfiError> {
    let range = found.utf8_range();
    let utf8_length = range
        .end
        .checked_sub(range.start)
        .ok_or_else(|| FfiError::new(ROFD_STATUS_INTERNAL, "canonical match range is reversed"))?;
    let rect = found.rect_mm().unwrap_or(Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    });
    Ok(TextMatchFields {
        utf8_offset: range.start,
        utf8_length,
        rect_mm: rofd_rect_t {
            x_mm: rect.x,
            y_mm: rect.y,
            width_mm: rect.width,
            height_mm: rect.height,
        },
    })
}

/// Copies one versioned search-match record by index.
///
/// # Safety
/// `search` must be null or a live immutable handle. A non-null `found` must
/// expose a readable initialized size field; supported records must be aligned
/// and writable for the complete v1 prefix. It and optional `error` must be
/// mutually disjoint and disjoint from live search storage.
#[no_mangle]
pub unsafe extern "C" fn rofd_text_search_get_match(
    search: *const rofd_text_search_t,
    index: usize,
    found: *mut rofd_text_match_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    // SAFETY: The caller supplies a readable size field before transaction entry.
    let output = unsafe { TextMatchOutput::required(found) };
    // SAFETY: The boundary validates record layout and live input disjointness.
    unsafe {
        semantic_boundary(error, output, handle_inputs(search), || {
            let found = search_ref(search)?.matches.get(index).ok_or_else(|| {
                FfiError::new(
                    ROFD_STATUS_PAGE_OUT_OF_RANGE,
                    format!("text match index {index} is out of range"),
                )
            })?;
            text_match_fields(found)
        })
    }
}

/// Frees an owned text-search result. Null is a no-op.
///
/// # Safety
/// A non-null handle must be live, uniquely owned, not used concurrently, and
/// freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn rofd_text_search_free(search: *mut rofd_text_search_t) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !search.is_null() {
            // SAFETY: The caller transfers the unique live allocation exactly once.
            unsafe { drop_raw_handle(search) };
        }
    }));
}

fn selection_style(style: u32) -> Result<SelectionStyle, FfiError> {
    match style {
        ROFD_SELECTION_GLYPH => Ok(SelectionStyle::Glyph),
        ROFD_SELECTION_WORD => Ok(SelectionStyle::Word),
        ROFD_SELECTION_LINE => Ok(SelectionStyle::Line),
        _ => Err(FfiError::invalid_argument(format!(
            "unknown text selection style {style}"
        ))),
    }
}

/// Returns an owned styled selection for a page rectangle in millimetres.
///
/// # Safety
/// `page` must be null or a live immutable page. `selection_mm` must be null or
/// an aligned, complete readable rectangle. `selection` is required and `error`
/// optional; outputs must be aligned, writable, mutually disjoint and disjoint
/// from page and rectangle storage, all of which remain live for the call.
#[no_mangle]
pub unsafe extern "C" fn rofd_page_get_selected_text(
    page: *const rofd_page_t,
    style: u32,
    selection_mm: *const rofd_rect_t,
    selection: *mut *mut rofd_text_selection_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let inputs = handle_inputs(page).and_then(|mut inputs| {
        inputs.push(selection_mm)?;
        Ok(inputs)
    });
    // SAFETY: The boundary validates all detectable input/output overlap before reads or writes.
    unsafe {
        semantic_boundary(error, HandleOutput::required(selection), inputs, || {
            let page = page_ref(page)?;
            let selection_mm = selection_mm
                .as_ref()
                .ok_or_else(|| FfiError::invalid_argument("selection rectangle is NULL"))?;
            let style = selection_style(style)?;
            let selected = page.inner.text()?.select(
                Rect {
                    x: selection_mm.x_mm,
                    y: selection_mm.y_mm,
                    width: selection_mm.width_mm,
                    height: selection_mm.height_mm,
                },
                style,
            )?;
            Ok(Box::new(TextSelectionHandle {
                text: c_string(selected.text()),
                regions: selected.regions().to_vec(),
            }))
        })
    }
}

/// Borrows selected NUL-terminated UTF-8 text; null returns null.
///
/// # Safety
/// A non-null selection must remain live and immutable through the borrow. The
/// returned pointer is valid only until [`rofd_text_selection_free`].
#[no_mangle]
pub unsafe extern "C" fn rofd_text_selection_get_text(
    selection: *const rofd_text_selection_t,
) -> *const c_char {
    catch_unwind(AssertUnwindSafe(|| {
        if selection.is_null() {
            return ptr::null();
        }
        // SAFETY: The caller guarantees a live immutable selection for the borrow.
        unsafe { handle_ref(selection).text.as_ptr() }
    }))
    .unwrap_or(ptr::null())
}

/// Returns selected UTF-8 byte length excluding the terminating NUL; null is zero.
///
/// # Safety
/// A non-null selection must remain live and immutable for the call.
#[no_mangle]
pub unsafe extern "C" fn rofd_text_selection_get_text_length(
    selection: *const rofd_text_selection_t,
) -> usize {
    catch_unwind(AssertUnwindSafe(|| {
        if selection.is_null() {
            return 0;
        }
        // SAFETY: The caller guarantees a live immutable selection for the read.
        unsafe { handle_ref(selection).text.as_bytes().len() }
    }))
    .unwrap_or(0)
}

/// Returns the number of selected page regions.
///
/// # Safety
/// `selection` must be null or live and immutable. `count` is required and
/// `error` optional; output storage must be aligned, writable, mutually
/// disjoint and disjoint from the live selection handle.
#[no_mangle]
pub unsafe extern "C" fn rofd_text_selection_get_region_count(
    selection: *const rofd_text_selection_t,
    count: *mut usize,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    // SAFETY: The boundary protects live selection storage before publication.
    unsafe {
        semantic_boundary(
            error,
            ScalarOutput::required(count),
            handle_inputs(selection),
            || Ok(selection_ref(selection)?.regions.len()),
        )
    }
}

/// Copies one selected page region by index.
///
/// # Safety
/// `selection` must be null or live and immutable. `region_mm` is required and
/// `error` optional; outputs must be aligned, writable, mutually disjoint and
/// disjoint from live selection storage.
#[no_mangle]
pub unsafe extern "C" fn rofd_text_selection_get_region(
    selection: *const rofd_text_selection_t,
    index: usize,
    region_mm: *mut rofd_rect_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    // SAFETY: The scalar transaction zeroes the rectangle on ordinary failures.
    unsafe {
        semantic_boundary(
            error,
            ScalarOutput::required(region_mm),
            handle_inputs(selection),
            || {
                let region = selection_ref(selection)?
                    .regions
                    .get(index)
                    .ok_or_else(|| {
                        FfiError::new(
                            ROFD_STATUS_PAGE_OUT_OF_RANGE,
                            format!("text selection region index {index} is out of range"),
                        )
                    })?;
                Ok(rofd_rect_t {
                    x_mm: region.x,
                    y_mm: region.y,
                    width_mm: region.width,
                    height_mm: region.height,
                })
            },
        )
    }
}

/// Frees an owned text selection. Null is a no-op.
///
/// # Safety
/// A non-null handle must be live, uniquely owned, not used concurrently, and
/// freed exactly once after all borrowed text accesses complete.
#[no_mangle]
pub unsafe extern "C" fn rofd_text_selection_free(selection: *mut rofd_text_selection_t) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !selection.is_null() {
            // SAFETY: The caller transfers the unique live allocation exactly once.
            unsafe { drop_raw_handle(selection) };
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
