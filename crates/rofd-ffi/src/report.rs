use crate::error::{
    boundary_with_inputs, DiagnosticFields, DiagnosticOutput, FfiError, InputRanges, ScalarOutput,
};
use crate::handles::{drop_raw_handle, handle_ref, OwnedDiagnostic, RenderReportHandle};
use crate::{
    rofd_error_t, rofd_render_diagnostic_t, rofd_render_report_t, rofd_status_t,
    ROFD_DIAGNOSTIC_FONT_FALLBACK, ROFD_DIAGNOSTIC_IMAGE_BORDER_UNSUPPORTED,
    ROFD_DIAGNOSTIC_IMAGE_MASK_UNSUPPORTED, ROFD_DIAGNOSTIC_IMAGE_SUBSTITUTION_UNSUPPORTED,
    ROFD_DIAGNOSTIC_MISSING_GLYPH, ROFD_DIAGNOSTIC_UNSUPPORTED_OBJECT, ROFD_STATUS_INTERNAL,
    ROFD_STATUS_PAGE_OUT_OF_RANGE,
};
use rofd_render::{RenderDiagnostic, RenderDiagnosticKind, RenderReport};
use std::ffi::CString;
use std::panic::{catch_unwind, AssertUnwindSafe};

impl RenderReportHandle {
    pub(crate) fn from_render_report(report: RenderReport) -> Result<Self, FfiError> {
        let diagnostics = report
            .diagnostics()
            .iter()
            .map(OwnedDiagnostic::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { diagnostics })
    }
}

impl TryFrom<&RenderDiagnostic> for OwnedDiagnostic {
    type Error = FfiError;

    fn try_from(diagnostic: &RenderDiagnostic) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: diagnostic_kind(diagnostic.kind())?,
            object_id: diagnostic.object_id(),
            message: visible_c_string(diagnostic.message()),
        })
    }
}

fn diagnostic_kind(kind: &RenderDiagnosticKind) -> Result<u32, FfiError> {
    match kind {
        RenderDiagnosticKind::UnsupportedObject { .. } => Ok(ROFD_DIAGNOSTIC_UNSUPPORTED_OBJECT),
        RenderDiagnosticKind::FontFallback { .. } => Ok(ROFD_DIAGNOSTIC_FONT_FALLBACK),
        RenderDiagnosticKind::MissingGlyph { .. } => Ok(ROFD_DIAGNOSTIC_MISSING_GLYPH),
        RenderDiagnosticKind::ImageSubstitutionUnsupported { .. } => {
            Ok(ROFD_DIAGNOSTIC_IMAGE_SUBSTITUTION_UNSUPPORTED)
        }
        RenderDiagnosticKind::ImageMaskUnsupported { .. } => {
            Ok(ROFD_DIAGNOSTIC_IMAGE_MASK_UNSUPPORTED)
        }
        RenderDiagnosticKind::ImageBorderUnsupported => {
            Ok(ROFD_DIAGNOSTIC_IMAGE_BORDER_UNSUPPORTED)
        }
        _ => Err(FfiError::new(
            ROFD_STATUS_INTERNAL,
            "renderer returned a diagnostic kind unknown to C ABI v1",
        )),
    }
}

fn visible_c_string(message: &str) -> CString {
    CString::new(message.replace('\0', "\\0"))
        .expect("replacing embedded NUL bytes must produce a valid C string")
}

unsafe fn report_ref<'a>(
    report: *const rofd_render_report_t,
) -> Result<&'a RenderReportHandle, FfiError> {
    if report.is_null() {
        return Err(FfiError::invalid_argument("render report handle is NULL"));
    }
    // SAFETY: The caller guarantees a live report token borrowed immutably for this call.
    Ok(unsafe { handle_ref::<rofd_render_report_t>(report) })
}

fn report_inputs(report: *const rofd_render_report_t) -> Result<InputRanges, ()> {
    let mut inputs = InputRanges::new();
    inputs.push(report.cast::<RenderReportHandle>())?;
    Ok(inputs)
}

/// Returns the number of diagnostics retained by a render report.
///
/// # Safety
///
/// `report` and `diagnostic_count` are required; null returns invalid argument.
/// A non-null `report` must be a live handle returned by this library, kept
/// immutable and not freed for the call. `diagnostic_count` must be writable and
/// aligned. `error` is optional; when non-null, it and `diagnostic_count` must be
/// mutually disjoint writable outputs. Report storage must be disjoint from
/// every non-null output, and none may be accessed concurrently.
#[no_mangle]
pub unsafe extern "C" fn rofd_render_report_get_count(
    report: *const rofd_render_report_t,
    diagnostic_count: *mut usize,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let inputs = match report_inputs(report) {
        Ok(inputs) => inputs,
        Err(()) => {
            // SAFETY: An empty input set lets the normal boundary publish this preflight error.
            return unsafe {
                boundary_with_inputs(
                    error,
                    ScalarOutput::required(diagnostic_count),
                    InputRanges::new(),
                    || {
                        Err(FfiError::invalid_argument(
                            "render report handle is misaligned or overflows",
                        ))
                    },
                )
            };
        }
    };
    // SAFETY: The boundary validates output layout and input/output disjointness before writes.
    unsafe {
        boundary_with_inputs(
            error,
            ScalarOutput::required(diagnostic_count),
            inputs,
            || Ok(report_ref(report)?.diagnostics.len()),
        )
    }
}

/// Returns one borrowed diagnostic from a render report.
///
/// On entry, `diagnostic->struct_size` must declare at least the complete v1
/// boundary. The transaction clears exactly that v1 prefix, including padding,
/// restores the caller's declared size, and preserves any unknown tail bytes.
/// The message pointer is borrowed and remains valid only until `report` is
/// freed.
///
/// # Safety
///
/// `report` must be null or a live report handle returned by this library and
/// remain immutable and live for the call. Non-null `diagnostic` must be aligned
/// and readable for `struct_size`; when it declares v1, the complete v1 prefix
/// must be writable. The report storage and every output slot, including
/// `error`, must be distinct and non-overlapping, remain live, and not be
/// accessed concurrently.
#[no_mangle]
pub unsafe extern "C" fn rofd_render_report_get_diagnostic(
    report: *const rofd_render_report_t,
    diagnostic_index: usize,
    diagnostic: *mut rofd_render_diagnostic_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    // SAFETY: The caller guarantees the diagnostic prefix is readable when non-null.
    let output = unsafe { DiagnosticOutput::required(diagnostic) };
    let inputs = match report_inputs(report) {
        Ok(inputs) => inputs,
        Err(()) => {
            // SAFETY: The normal transaction initializes a valid diagnostic output on failure.
            return unsafe {
                boundary_with_inputs(error, output, InputRanges::new(), || {
                    Err(FfiError::invalid_argument(
                        "render report handle is misaligned or overflows",
                    ))
                })
            };
        }
    };
    // SAFETY: The boundary preflights the report storage against the record and error outputs.
    unsafe {
        boundary_with_inputs(error, output, inputs, || {
            let report = report_ref(report)?;
            let diagnostic = report.diagnostics.get(diagnostic_index).ok_or_else(|| {
                FfiError::new(
                    ROFD_STATUS_PAGE_OUT_OF_RANGE,
                    format!("diagnostic index {diagnostic_index} is out of range"),
                )
            })?;
            Ok(DiagnosticFields {
                kind: diagnostic.kind,
                object_id: diagnostic.object_id,
                message: diagnostic.message.as_ptr(),
            })
        })
    }
}

/// Frees an owned render report handle. A null handle is a no-op.
///
/// # Safety
///
/// A non-null handle must be live, uniquely owned by the caller, not previously
/// freed, and not used concurrently. Every borrowed diagnostic message becomes
/// invalid when this function returns.
#[no_mangle]
pub unsafe extern "C" fn rofd_render_report_free(report: *mut rofd_render_report_t) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if report.is_null() {
            return;
        }
        // SAFETY: The caller transfers the unique allocation to its sealed token mapping once.
        unsafe { drop_raw_handle::<rofd_render_report_t>(report) };
    }));
}

#[cfg(test)]
mod tests {
    use super::{diagnostic_kind, visible_c_string};
    use crate::{
        ROFD_DIAGNOSTIC_FONT_FALLBACK, ROFD_DIAGNOSTIC_IMAGE_BORDER_UNSUPPORTED,
        ROFD_DIAGNOSTIC_IMAGE_MASK_UNSUPPORTED, ROFD_DIAGNOSTIC_IMAGE_SUBSTITUTION_UNSUPPORTED,
        ROFD_DIAGNOSTIC_MISSING_GLYPH, ROFD_DIAGNOSTIC_UNSUPPORTED_OBJECT,
    };
    use rofd_core::UnsupportedObjectKind;
    use rofd_render::RenderDiagnosticKind;

    #[test]
    fn all_v1_diagnostic_kinds_have_the_exact_public_values() {
        let kinds = [
            (
                RenderDiagnosticKind::UnsupportedObject {
                    kind: UnsupportedObjectKind::Text,
                },
                ROFD_DIAGNOSTIC_UNSUPPORTED_OBJECT,
            ),
            (
                RenderDiagnosticKind::FontFallback {
                    character: 'a',
                    scalar_index: 0,
                    requested: "A".to_owned(),
                    selected: "B".to_owned(),
                },
                ROFD_DIAGNOSTIC_FONT_FALLBACK,
            ),
            (
                RenderDiagnosticKind::MissingGlyph {
                    character: 'a',
                    scalar_index: 0,
                    used_visible_replacement: true,
                },
                ROFD_DIAGNOSTIC_MISSING_GLYPH,
            ),
            (
                RenderDiagnosticKind::ImageSubstitutionUnsupported { resource_id: 1 },
                ROFD_DIAGNOSTIC_IMAGE_SUBSTITUTION_UNSUPPORTED,
            ),
            (
                RenderDiagnosticKind::ImageMaskUnsupported { resource_id: 1 },
                ROFD_DIAGNOSTIC_IMAGE_MASK_UNSUPPORTED,
            ),
            (
                RenderDiagnosticKind::ImageBorderUnsupported,
                ROFD_DIAGNOSTIC_IMAGE_BORDER_UNSUPPORTED,
            ),
        ];
        for (kind, expected) in kinds {
            let Ok(actual) = diagnostic_kind(&kind) else {
                panic!("current diagnostic kind must have a C ABI mapping");
            };
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn embedded_nul_is_made_visible() {
        assert_eq!(visible_c_string("left\0right").to_bytes(), b"left\\0right");
    }
}
