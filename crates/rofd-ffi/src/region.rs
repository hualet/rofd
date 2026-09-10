use crate::abi::ROFD_PIXEL_RECT_V1_SIZE;
use crate::document::page_ref;
use crate::error::{boundary_with_inputs, FfiError, HandleOutput, ScalarOutput};
use crate::handles::RenderReportHandle;
use crate::renderer::{render_input_ranges, renderer_ref, RenderOptionsInput};
use crate::{
    rofd_error_t, rofd_page_t, rofd_pixel_rect_t, rofd_render_options_t, rofd_render_report_t,
    rofd_renderer_t, rofd_status_t, ROFD_STATUS_INVALID_ARGUMENT,
};
use rofd_render::{CairoRenderer, PixelRect};
use std::ptr;

/// Computes the final pixel canvas independently of full-page raster allocation.
///
/// Positive canvas dimensions up to i32::MAX are supported. The full-page Cairo
/// limit and raster budget are checked only when a render target is requested.
/// Null options use defaults. Invalid input or output address layouts leave all outputs
/// untouched. Overlaps leave ordinary outputs untouched but may publish an error
/// to an independent valid error slot. Other failures zero valid scalar outputs.
///
/// # Safety
/// Non-null renderer/page pointers must be live immutable matching library
/// handles. Non-null options must be aligned with initialized struct_size and
/// a readable supported prefix. Width and height are required, aligned writable
/// scalar slots; error is optional. All outputs must be mutually disjoint and
/// disjoint from handle storage and options, live and exclusively writable for
/// the call. Inputs must remain live without concurrent mutation.
#[no_mangle]
pub unsafe extern "C" fn rofd_renderer_get_pixel_canvas_size(
    renderer: *const rofd_renderer_t,
    page: *const rofd_page_t,
    options: *const rofd_render_options_t,
    width: *mut i32,
    height: *mut i32,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = render_input_ranges(renderer, page, ptr::null_mut(), options) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: The shared boundary preflights outputs and inputs before any read/write.
    unsafe {
        boundary_with_inputs(
            error,
            (
                ScalarOutput::required(width),
                ScalarOutput::required(height),
            ),
            inputs,
            || {
                let _ = renderer_ref(renderer)?;
                let page = page_ref(page)?;
                let options = RenderOptionsInput::from_ffi(options)?;
                CairoRenderer::pixel_canvas_size(&page.inner, &options).map_err(FfiError::from)
            },
        )
    }
}

/// Renders a pixel viewport into a borrowed Cairo context.
///
/// The viewport is required, lies wholly within the final full-page pixel canvas,
/// and maps its origin to target (0, 0). The mm clip keeps its original page-space
/// meaning. Target and working surface limits apply to viewport dimensions.
/// A non-null report receives an owned diagnostic snapshot on success and null
/// on failure. Invalid input or output address layouts leave outputs untouched. Overlaps
/// preserve ordinary outputs but may populate an independent valid error slot.
///
/// # Safety
/// Non-null renderer/page pointers must be live immutable matching handles.
/// Non-null options and viewport must be aligned with readable initialized
/// struct_size fields and supported prefixes. Context must be a live Cairo
/// context with live target and referenced objects, accessible on this thread
/// without concurrent mutation; one temporary reference is taken and released.
/// Optional report and error slots must be aligned, writable, mutually disjoint,
/// and disjoint from all handle storage, input prefixes and the Cairo address.
/// All storage must remain live without conflicting access for the entire call.
#[no_mangle]
pub unsafe extern "C" fn rofd_renderer_render_page_region_cairo(
    renderer: *const rofd_renderer_t,
    page: *const rofd_page_t,
    context: *mut cairo::ffi::cairo_t,
    options: *const rofd_render_options_t,
    viewport: *const rofd_pixel_rect_t,
    report: *mut *mut rofd_render_report_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let inputs = render_input_ranges(renderer, page, context, options).and_then(|mut inputs| {
        inputs.push(viewport)?;
        Ok(inputs)
    });
    let Ok(inputs) = inputs else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    let render = || {
        // SAFETY: Called only after shared input/output preflight. All pointers obey
        // the exported contract; null pointers and undersized records are rejected.
        unsafe {
            let renderer = renderer_ref(renderer)?;
            let page = page_ref(page)?;
            if context.is_null() || viewport.is_null() {
                return Err(FfiError::invalid_argument(
                    "context and viewport are required",
                ));
            }
            if (*viewport).struct_size < ROFD_PIXEL_RECT_V1_SIZE as u32 {
                return Err(FfiError::invalid_argument(
                    "viewport struct_size is smaller than v1",
                ));
            }
            let viewport = PixelRect {
                x: (*viewport).x,
                y: (*viewport).y,
                width: (*viewport).width,
                height: (*viewport).height,
            };
            let options = RenderOptionsInput::from_ffi(options)?;
            let context = cairo::Context::from_raw_none(context);
            CairoRenderer
                .render_page_region_with_services(
                    &page.inner,
                    &context,
                    &options,
                    viewport,
                    &renderer.font_resolver,
                    &renderer.image_decoder,
                )
                .map_err(FfiError::from)
        }
    };
    // SAFETY: The common boundary contains panics and transactionally publishes outputs.
    unsafe {
        if report.is_null() {
            boundary_with_inputs(error, (), inputs, || {
                render()?;
                Ok(())
            })
        } else {
            boundary_with_inputs(error, HandleOutput::required(report), inputs, || {
                Ok(Box::new(RenderReportHandle::from_render_report(render()?)?))
            })
        }
    }
}
