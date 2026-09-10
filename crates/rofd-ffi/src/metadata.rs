use crate::document::document_ref;
use crate::error::{
    boundary_with_inputs, FfiError, HandleOutput, InputRanges, ScalarOutput, WarningFields,
    WarningOutput,
};
use crate::handles::{
    drop_raw_handle, handle_ref, HandleToken, MetadataHandle, OwnedWarning, WarningListHandle,
};
use crate::{
    rofd_document_t, rofd_error_t, rofd_metadata_t, rofd_status_t, rofd_warning_list_t,
    rofd_warning_t, ROFD_STATUS_INVALID_ARGUMENT, ROFD_STATUS_PAGE_OUT_OF_RANGE,
    ROFD_WARNING_ANNOTATION_SKIPPED, ROFD_WARNING_DOCUMENT_PAGE_AREA_MISSING,
    ROFD_WARNING_HISTORICAL_DOC_BODY_SKIPPED, ROFD_WARNING_PAGE_AREA_FALLBACK,
    ROFD_WARNING_SIGNATURE_SKIPPED, ROFD_WARNING_UNKNOWN,
    ROFD_WARNING_UNKNOWN_GRAPHIC_UNIT_SKIPPED,
};
use rofd_core::{Metadata, Warning, WarningCode};
use std::ffi::{c_char, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

fn visible_c_string(text: &str) -> CString {
    CString::new(text.replace('\0', "\\0"))
        .expect("replacing embedded NUL bytes must produce a valid C string")
}

impl From<&Metadata> for MetadataHandle {
    fn from(metadata: &Metadata) -> Self {
        Self {
            document_id: metadata.document_id.as_deref().map(visible_c_string),
            title: metadata.title.as_deref().map(visible_c_string),
            author: metadata.author.as_deref().map(visible_c_string),
            subject: metadata.subject.as_deref().map(visible_c_string),
            abstract_text: metadata.abstract_text.as_deref().map(visible_c_string),
            creator: metadata.creator.as_deref().map(visible_c_string),
            creator_version: metadata.creator_version.as_deref().map(visible_c_string),
            creation_date: metadata.creation_date.as_deref().map(visible_c_string),
            modification_date: metadata.modification_date.as_deref().map(visible_c_string),
            keywords: metadata
                .keywords
                .iter()
                .map(|keyword| visible_c_string(keyword))
                .collect(),
        }
    }
}

fn warning_code(code: WarningCode) -> u32 {
    match code {
        WarningCode::PageAreaFallback => ROFD_WARNING_PAGE_AREA_FALLBACK,
        WarningCode::DocumentPageAreaMissing => ROFD_WARNING_DOCUMENT_PAGE_AREA_MISSING,
        WarningCode::SignatureSkipped => ROFD_WARNING_SIGNATURE_SKIPPED,
        WarningCode::UnknownGraphicUnitSkipped => ROFD_WARNING_UNKNOWN_GRAPHIC_UNIT_SKIPPED,
        WarningCode::AnnotationSkipped => ROFD_WARNING_ANNOTATION_SKIPPED,
        WarningCode::HistoricalDocBodySkipped => ROFD_WARNING_HISTORICAL_DOC_BODY_SKIPPED,
        _ => ROFD_WARNING_UNKNOWN,
    }
}

impl From<Warning> for OwnedWarning {
    fn from(warning: Warning) -> Self {
        Self {
            code: warning_code(warning.code),
            path: visible_c_string(&warning.path),
            message: visible_c_string(&warning.message),
        }
    }
}

fn handle_inputs<Token: HandleToken>(handle: *const Token) -> Result<InputRanges, ()> {
    let mut inputs = InputRanges::new();
    inputs.push(handle.cast::<Token::Storage>())?;
    Ok(inputs)
}

unsafe fn metadata_ref<'a>(
    metadata: *const rofd_metadata_t,
) -> Result<&'a MetadataHandle, FfiError> {
    if metadata.is_null() {
        return Err(FfiError::invalid_argument("metadata handle is NULL"));
    }
    // SAFETY: The caller supplies a live immutable metadata token for the borrow.
    Ok(unsafe { handle_ref(metadata) })
}

unsafe fn warnings_ref<'a>(
    warnings: *const rofd_warning_list_t,
) -> Result<&'a WarningListHandle, FfiError> {
    if warnings.is_null() {
        return Err(FfiError::invalid_argument("warning list handle is NULL"));
    }
    // SAFETY: The caller supplies a live immutable warning-list token for the borrow.
    Ok(unsafe { handle_ref(warnings) })
}

/// Copies metadata into an independently owned immutable snapshot.
///
/// The result survives freeing its document. Release it with [`rofd_metadata_free`].
/// Malformed addresses leave all outputs untouched; overlaps preserve ordinary
/// outputs but may publish to an independent error slot. Other failures null the result.
///
/// # Safety
/// `document` is required and must be null or a live immutable document handle.
/// `metadata` is required and `error` is optional. Non-null output slots must be
/// aligned, writable, mutually disjoint and disjoint from document storage.
/// All storage must stay live without conflicting concurrent access for the call.
#[no_mangle]
pub unsafe extern "C" fn rofd_document_get_metadata(
    document: *const rofd_document_t,
    metadata: *mut *mut rofd_metadata_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(document) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: The boundary preflights document storage against all outputs before borrowing/writing.
    unsafe {
        boundary_with_inputs(error, HandleOutput::required(metadata), inputs, || {
            Ok(Box::new(MetadataHandle::from(
                document_ref(document)?.inner.metadata(),
            )))
        })
    }
}

macro_rules! metadata_reader {
    ($name:ident, $field:ident, $description:literal) => {
        #[doc = $description]
        ///
        /// Null handles and absent fields return null. A present empty value
        /// returns a non-null empty string. Embedded NULs are escaped as `\0`.
        ///
        /// # Safety
        /// A non-null handle must be live and immutable, with no concurrent free.
        /// The borrowed UTF-8 string must not be modified or freed and remains
        /// valid only until [`rofd_metadata_free`] releases its owner.
        #[no_mangle]
        pub unsafe extern "C" fn $name(metadata: *const rofd_metadata_t) -> *const c_char {
            catch_unwind(AssertUnwindSafe(|| {
                if metadata.is_null() || handle_inputs(metadata).is_err() {
                    return ptr::null();
                }
                // SAFETY: The caller guarantees a live immutable metadata token for this borrow.
                unsafe {
                    handle_ref(metadata)
                        .$field
                        .as_ref()
                        .map_or(ptr::null(), |value| value.as_ptr())
                }
            }))
            .unwrap_or(ptr::null())
        }
    };
}

metadata_reader!(
    rofd_metadata_get_document_id,
    document_id,
    "Borrows the document identifier."
);
metadata_reader!(
    rofd_metadata_get_title,
    title,
    "Borrows the document title."
);
metadata_reader!(
    rofd_metadata_get_author,
    author,
    "Borrows the document author."
);
metadata_reader!(
    rofd_metadata_get_subject,
    subject,
    "Borrows the document subject."
);
metadata_reader!(
    rofd_metadata_get_abstract,
    abstract_text,
    "Borrows the document abstract."
);
metadata_reader!(
    rofd_metadata_get_creator,
    creator,
    "Borrows the producing application name."
);
metadata_reader!(
    rofd_metadata_get_creator_version,
    creator_version,
    "Borrows the producing application version."
);
metadata_reader!(
    rofd_metadata_get_creation_date,
    creation_date,
    "Borrows the creation date exactly as stored by the producer."
);
metadata_reader!(
    rofd_metadata_get_modification_date,
    modification_date,
    "Borrows the modification date exactly as stored by the producer."
);

/// Returns the number of keywords, including repeated and empty entries.
///
/// Invalid address layouts preserve all outputs; overlaps preserve the count
/// but may publish an independent error. Other failures zero the count.
///
/// # Safety
/// `metadata` and `count` are required. A non-null metadata handle must remain
/// live and immutable. Non-null `count` and optional `error` must be aligned,
/// writable, mutually disjoint and disjoint from metadata storage, and remain
/// live without conflicting concurrent access for the entire call.
#[no_mangle]
pub unsafe extern "C" fn rofd_metadata_get_keyword_count(
    metadata: *const rofd_metadata_t,
    count: *mut usize,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(metadata) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: The boundary checks inputs and outputs before any handle borrow or output write.
    unsafe {
        boundary_with_inputs(error, ScalarOutput::required(count), inputs, || {
            Ok(metadata_ref(metadata)?.keywords.len())
        })
    }
}

/// Borrows a keyword by zero-based index in declaration order.
///
/// The UTF-8 string, with embedded NULs escaped as `\0`, remains valid until
/// [`rofd_metadata_free`]. Out-of-range indices return `ROFD_STATUS_PAGE_OUT_OF_RANGE`.
/// Invalid address layouts preserve all outputs; overlaps preserve the keyword
/// output but may publish an independent error. Other failures null the keyword.
///
/// # Safety
/// `metadata` and `keyword` are required. A non-null metadata handle must stay
/// live and immutable. Non-null `keyword` and optional `error` must be aligned,
/// writable, mutually disjoint and disjoint from metadata storage for the call.
/// Neither outputs nor input storage may be accessed concurrently in conflict.
/// The returned string must not be modified or freed separately.
#[no_mangle]
pub unsafe extern "C" fn rofd_metadata_get_keyword(
    metadata: *const rofd_metadata_t,
    index: usize,
    keyword: *mut *const c_char,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(metadata) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: The boundary checks inputs and outputs before any handle borrow or output write.
    unsafe {
        boundary_with_inputs(error, ScalarOutput::required(keyword), inputs, || {
            metadata_ref(metadata)?
                .keywords
                .get(index)
                .map(|keyword| keyword.as_ptr())
                .ok_or_else(|| {
                    FfiError::new(
                        ROFD_STATUS_PAGE_OUT_OF_RANGE,
                        format!("keyword index {index} is out of range"),
                    )
                })
        })
    }
}

/// Frees metadata and invalidates its borrowed strings. Null is a no-op.
///
/// # Safety
/// A non-null handle must be live, uniquely owned and freed exactly once, with
/// no concurrent readers or subsequent uses of borrowed strings.
#[no_mangle]
pub unsafe extern "C" fn rofd_metadata_free(metadata: *mut rofd_metadata_t) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !metadata.is_null() {
            // SAFETY: The caller transfers the unique live allocation exactly once.
            unsafe {
                drop_raw_handle(metadata);
            }
        }
    }));
}

/// Copies currently collected parse warnings into an immutable owned snapshot.
///
/// This call does not load pages or force lazy parsing. Subsequent page or
/// resource queries may add document warnings; existing snapshots never change.
/// The result survives document/page free and must be freed with [`rofd_warning_list_free`].
/// Invalid address layouts preserve all outputs; overlaps preserve the result
/// but may publish an independent error. Other failures null the result.
///
/// # Safety
/// `document` and `warnings` are required; non-null document handles must remain
/// live and immutable. Non-null `warnings` and optional `error` must be aligned,
/// writable, mutually disjoint and disjoint from document storage. All storage
/// must remain live without conflicting concurrent access for the entire call.
#[no_mangle]
pub unsafe extern "C" fn rofd_document_get_warnings(
    document: *const rofd_document_t,
    warnings: *mut *mut rofd_warning_list_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(document) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: The boundary preflights the document against all outputs before borrowing/writing.
    unsafe {
        boundary_with_inputs(error, HandleOutput::required(warnings), inputs, || {
            Ok(Box::new(WarningListHandle {
                warnings: document_ref(document)?
                    .inner
                    .warnings()
                    .into_iter()
                    .map(OwnedWarning::from)
                    .collect(),
            }))
        })
    }
}

/// Returns the number of parse warnings retained by a snapshot.
///
/// Invalid address layouts preserve all outputs; overlaps preserve the count
/// but may publish an independent error. Other failures zero the count.
///
/// # Safety
/// `warnings` and `count` are required. A non-null snapshot must stay live and
/// immutable. Non-null `count` and optional `error` must be aligned, writable,
/// mutually disjoint and disjoint from snapshot storage. All storage must remain
/// live without conflicting concurrent access for the entire call.
#[no_mangle]
pub unsafe extern "C" fn rofd_warning_list_get_count(
    warnings: *const rofd_warning_list_t,
    count: *mut usize,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(warnings) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: The boundary checks inputs and outputs before any handle borrow or output write.
    unsafe {
        boundary_with_inputs(error, ScalarOutput::required(count), inputs, || {
            Ok(warnings_ref(warnings)?.warnings.len())
        })
    }
}

/// Returns one borrowed warning by zero-based index.
///
/// Set `warning->struct_size` to at least the complete v1 record size. The
/// transaction clears that prefix including padding, restores the captured size
/// and preserves any unknown tail bytes. Undersized records remain untouched.
/// Invalid indices return `ROFD_STATUS_PAGE_OUT_OF_RANGE`; other ordinary failures
/// leave a valid prefix zeroed except for `struct_size`. Invalid address layouts
/// preserve all outputs; overlaps preserve the record but may publish an independent error.
///
/// # Safety
/// `warnings` and `warning` are required. A non-null snapshot must remain live
/// and immutable. Non-null warning storage must be aligned and readable for its
/// initialized `struct_size`; a supported prefix must be writable. The record
/// and optional aligned writable `error` slot must be mutually disjoint and
/// disjoint from snapshot storage, without conflicting concurrent access.
/// Returned UTF-8 path/message pointers must not be modified or freed and remain
/// valid only until [`rofd_warning_list_free`]. Embedded NULs are escaped as `\0`.
#[no_mangle]
pub unsafe extern "C" fn rofd_warning_list_get_warning(
    warnings: *const rofd_warning_list_t,
    index: usize,
    warning: *mut rofd_warning_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(warnings) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: The caller supplies a readable initialized size when the address layout is valid.
    let output = unsafe { WarningOutput::required(warning) };
    // SAFETY: The boundary preflights the snapshot storage against the record and error slot.
    unsafe {
        boundary_with_inputs(error, output, inputs, || {
            let warning = warnings_ref(warnings)?.warnings.get(index).ok_or_else(|| {
                FfiError::new(
                    ROFD_STATUS_PAGE_OUT_OF_RANGE,
                    format!("warning index {index} is out of range"),
                )
            })?;
            Ok(WarningFields {
                code: warning.code,
                path: warning.path.as_ptr(),
                message: warning.message.as_ptr(),
            })
        })
    }
}

/// Frees a warning snapshot and invalidates its borrowed strings. Null is a no-op.
///
/// # Safety
/// A non-null handle must be live, uniquely owned and freed exactly once, with
/// no concurrent readers or subsequent uses of borrowed warning strings.
#[no_mangle]
pub unsafe extern "C" fn rofd_warning_list_free(warnings: *mut rofd_warning_list_t) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !warnings.is_null() {
            // SAFETY: The caller transfers the unique live allocation exactly once.
            unsafe {
                drop_raw_handle(warnings);
            }
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_known_warning_codes_map_to_their_stable_values() {
        for (code, expected) in [
            (WarningCode::PageAreaFallback, 1),
            (WarningCode::DocumentPageAreaMissing, 2),
            (WarningCode::SignatureSkipped, 3),
            (WarningCode::UnknownGraphicUnitSkipped, 4),
            (WarningCode::AnnotationSkipped, 5),
            (WarningCode::HistoricalDocBodySkipped, 6),
        ] {
            assert_eq!(warning_code(code), expected);
        }
    }

    #[test]
    fn snapshot_strings_escape_embedded_nul_without_truncating_utf8() {
        let metadata = MetadataHandle::from(&Metadata {
            title: Some("左\0右".into()),
            keywords: vec!["甲\0乙".into()],
            ..Metadata::default()
        });
        assert_eq!(metadata.title.unwrap().to_str().unwrap(), "左\\0右");
        assert_eq!(metadata.keywords[0].to_str().unwrap(), "甲\\0乙");
        let warning = OwnedWarning::from(Warning {
            code: WarningCode::AnnotationSkipped,
            path: "路径\0.xml".into(),
            message: "消息\0末尾".into(),
        });
        assert_eq!(warning.path.to_str().unwrap(), "路径\\0.xml");
        assert_eq!(warning.message.to_str().unwrap(), "消息\\0末尾");
    }
}
