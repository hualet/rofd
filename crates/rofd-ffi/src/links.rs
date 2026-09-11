use crate::abi::*;
use crate::document::page_ref;
use crate::error::{
    boundary_with_inputs, ActionOutput, DestinationOutput, FfiError, HandleOutput, ScalarOutput,
};
use crate::handles::{
    drop_raw_handle, handle_ref, rofd_error_t, rofd_link_list_t, rofd_page_t, LinkListHandle,
    OwnedAction, OwnedLink,
};
use crate::navigation::handle_inputs;
use std::panic::{catch_unwind, AssertUnwindSafe};

unsafe fn list_ref<'a>(links: *const rofd_link_list_t) -> Result<&'a LinkListHandle, FfiError> {
    if links.is_null() {
        return Err(FfiError::invalid_argument("link list handle is NULL"));
    }
    // SAFETY: Caller promises a live immutable list for the duration of this borrow.
    Ok(unsafe { handle_ref(links) })
}

fn link_at(list: &LinkListHandle, index: usize) -> Result<&OwnedLink, FfiError> {
    list.links.get(index).ok_or_else(|| {
        FfiError::new(
            ROFD_STATUS_PAGE_OUT_OF_RANGE,
            format!("link index {index} is out of range"),
        )
    })
}

fn action_at(list: &LinkListHandle, link: usize, index: usize) -> Result<&OwnedAction, FfiError> {
    link_at(list, link)?.actions.get(index).ok_or_else(|| {
        FfiError::new(
            ROFD_STATUS_PAGE_OUT_OF_RANGE,
            format!("link action index {index} is out of range"),
        )
    })
}

/// Copies page links into an independently owned immutable snapshot.
///
/// A page without links yields an empty non-null list. Each entry represents
/// one source action with one or more conservative page-space millimetre
/// rectangles. Page/root actions precede effective content, objects precede
/// children, and visible annotation actions follow page content. Event names
/// remain inspectable; consumers should filter CLICK for mouse hit testing.
/// No action is executed, including file URIs and attachment actions.
///
/// First query can add parse warnings. Invalid explicit regions fail in strict
/// mode or skip that link with a warning in lenient mode; they never become
/// whole-page regions. Snapshot data survives page/document free.
/// Invalid address layouts leave every output untouched. Overlap leaves
/// ordinary outputs untouched but may publish an independent error. Ordinary
/// failures null a valid list output. All list queries follow these contracts.
///
/// # Safety
/// `page` and `links` are required. A non-null page must remain live and immutable.
/// Non-null links and optional error slots must be aligned, writable, mutually
/// disjoint and disjoint from page storage. All storage must stay live without
/// conflicting concurrent access for the call. Free the result exactly once.
#[no_mangle]
pub unsafe extern "C" fn rofd_page_get_links(
    page: *const rofd_page_t,
    links: *mut *mut rofd_link_list_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(page) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: Range preflight precedes all handle reads and output writes.
    unsafe {
        boundary_with_inputs(error, HandleOutput::required(links), inputs, || {
            Ok(Box::new(LinkListHandle {
                links: page_ref(page)?
                    .inner
                    .links()?
                    .iter()
                    .map(|link| OwnedLink {
                        regions: link.regions.clone(),
                        actions: link.actions.iter().map(OwnedAction::from).collect(),
                    })
                    .collect(),
            }))
        })
    }
}

/// Returns the link count; ordinary failures zero a valid output.
///
/// # Safety
/// Links and count are required. A non-null list must be live and immutable.
/// Non-null count/optional error outputs must be aligned, writable, mutually
/// disjoint and disjoint from list storage, with no concurrent conflicting access.
#[no_mangle]
pub unsafe extern "C" fn rofd_link_list_get_count(
    links: *const rofd_link_list_t,
    count: *mut usize,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(links) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: The boundary validates all input/output ranges before borrowing.
    unsafe {
        boundary_with_inputs(error, ScalarOutput::required(count), inputs, || {
            Ok(list_ref(links)?.links.len())
        })
    }
}

/// Returns a link's region count. Invalid indices return PAGE_OUT_OF_RANGE.
/// Ordinary failures zero a valid count; preflight follows [`rofd_page_get_links`].
///
/// # Safety
/// Links and count are required. The immutable list and aligned writable count
/// and optional error slots must be live and mutually disjoint, including list
/// storage, without conflicting concurrent access for the call.
#[no_mangle]
pub unsafe extern "C" fn rofd_link_list_get_region_count(
    links: *const rofd_link_list_t,
    link_index: usize,
    count: *mut usize,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(links) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: The boundary validates all input/output ranges before borrowing.
    unsafe {
        boundary_with_inputs(error, ScalarOutput::required(count), inputs, || {
            Ok(link_at(list_ref(links)?, link_index)?.regions.len())
        })
    }
}

/// Copies one conservative axis-aligned region in physical-page millimetres.
///
/// Coordinates already include the containing object's and ancestors' applicable
/// transforms, independent of rendering DPI/rotation. A fallback object Boundary
/// is in parent coordinates and does not receive the object's CTM twice.
/// Invalid link/region indices return PAGE_OUT_OF_RANGE. Ordinary failures zero
/// the complete rectangle; preflight follows [`rofd_page_get_links`].
///
/// # Safety
/// Links and region are required. The list must be live and immutable. Non-null
/// region/optional error outputs must be aligned, writable, mutually disjoint
/// and disjoint from list storage, with no conflicting concurrent access.
#[no_mangle]
pub unsafe extern "C" fn rofd_link_list_get_region(
    links: *const rofd_link_list_t,
    link_index: usize,
    region_index: usize,
    region_mm: *mut rofd_rect_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(links) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: The boundary validates all ranges before borrowing or writing.
    unsafe {
        boundary_with_inputs(error, ScalarOutput::required(region_mm), inputs, || {
            let region = link_at(list_ref(links)?, link_index)?
                .regions
                .get(region_index)
                .ok_or_else(|| {
                    FfiError::new(
                        ROFD_STATUS_PAGE_OUT_OF_RANGE,
                        format!("link region index {region_index} is out of range"),
                    )
                })?;
            Ok(rofd_rect_t {
                x_mm: region.x,
                y_mm: region.y,
                width_mm: region.width,
                height_mm: region.height,
            })
        })
    }
}

/// Returns the number of ordered actions on one link (currently one).
/// Invalid indices return PAGE_OUT_OF_RANGE; ordinary failures zero the count.
///
/// # Safety
/// Links and count are required. The immutable list and aligned writable count
/// and optional error slots must be live and mutually disjoint, including list
/// storage, without conflicting concurrent access for the call.
#[no_mangle]
pub unsafe extern "C" fn rofd_link_list_get_action_count(
    links: *const rofd_link_list_t,
    link_index: usize,
    count: *mut usize,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(links) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: The boundary validates all input/output ranges before borrowing.
    unsafe {
        boundary_with_inputs(error, ScalarOutput::required(count), inputs, || {
            Ok(link_at(list_ref(links)?, link_index)?.actions.len())
        })
    }
}

/// Borrows one inert action. Strings remain valid until [`rofd_link_list_free`].
///
/// Initialize struct_size. Ordinary failures clear the known prefix including
/// padding, preserve struct_size and leave unknown tails untouched. Invalid
/// indices return PAGE_OUT_OF_RANGE; preflight follows [`rofd_page_get_links`].
///
/// # Safety
/// Links/action are required. The non-null list must remain live and immutable.
/// Non-null action storage must be aligned/readable for its initialized size,
/// and writable for a supported prefix. Action/optional error outputs must be
/// mutually disjoint and disjoint from list storage, with no concurrent conflicts.
/// Borrowed strings are read-only and may not outlive the list.
#[no_mangle]
pub unsafe extern "C" fn rofd_link_list_get_action(
    links: *const rofd_link_list_t,
    link_index: usize,
    action_index: usize,
    action: *mut rofd_action_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(links) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: A layout-valid pointer has a readable initialized size by contract.
    let output = unsafe { ActionOutput::required(action) };
    // SAFETY: Range preflight precedes handle reads and output writes.
    unsafe {
        boundary_with_inputs(error, output, inputs, || {
            Ok(action_at(list_ref(links)?, link_index, action_index)?.fields())
        })
    }
}

/// Borrows an internal jump destination; non-Goto actions return UNSUPPORTED.
///
/// Check HAS_PAGE_INDEX before navigating. Unresolved Goto actions succeed with
/// NO_INDEX and no HAS_PAGE_INDEX, never a fabricated page zero. Other HAS_* bits
/// distinguish absent coordinates/zoom from explicit zero. Coordinates are
/// physical-page millimetres; zero zoom retains the current zoom. Record
/// transactions and string lifetimes follow [`rofd_link_list_get_action`].
///
/// # Safety
/// Links/destination are required. A non-null list must remain live and immutable.
/// Non-null destination storage must be aligned/readable for initialized size and
/// writable for its supported prefix. Destination/optional error outputs must be
/// mutually disjoint and disjoint from list storage, live without conflicting
/// concurrent access. Borrowed mode strings are read-only and may not outlive list.
#[no_mangle]
pub unsafe extern "C" fn rofd_link_list_get_action_destination(
    links: *const rofd_link_list_t,
    link_index: usize,
    action_index: usize,
    destination: *mut rofd_destination_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(links) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: A layout-valid pointer has a readable initialized size by contract.
    let output = unsafe { DestinationOutput::required(destination) };
    // SAFETY: Range preflight precedes handle reads and output writes.
    unsafe {
        boundary_with_inputs(error, output, inputs, || {
            action_at(list_ref(links)?, link_index, action_index)?.destination_fields()
        })
    }
}

/// Frees a link snapshot; null is a no-op and all borrowed strings expire.
///
/// # Safety
/// A non-null list must be live, uniquely owned and freed exactly once, without
/// concurrent readers or subsequent uses of its borrowed strings.
#[no_mangle]
pub unsafe extern "C" fn rofd_link_list_free(links: *mut rofd_link_list_t) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !links.is_null() {
            // SAFETY: Caller transfers the unique live allocation exactly once.
            unsafe { drop_raw_handle(links) };
        }
    }));
}
