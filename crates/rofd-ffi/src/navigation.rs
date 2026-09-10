use crate::abi::*;
use crate::document::document_ref;
use crate::error::{
    boundary_with_inputs, ActionFields, ActionOutput, DestinationFields, DestinationOutput,
    FfiError, HandleOutput, InputRanges, OutlineNodeFields, OutlineNodeOutput, ScalarOutput,
};
use crate::handles::{
    drop_raw_handle, handle_ref, rofd_document_t, rofd_error_t, rofd_outline_t, HandleToken,
    OutlineHandle, OwnedAction, OwnedOutlineNode,
};
use rofd_core::{Action, ActionEvent, ActionKind, DestinationMode};
use std::ffi::{c_char, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

pub(crate) fn visible_c_string(value: &str) -> CString {
    CString::new(value.replace('\0', "\\0")).expect("replacing NUL bytes produces a valid C string")
}

fn optional_string(value: &Option<String>) -> Option<CString> {
    value.as_deref().map(visible_c_string)
}

fn borrowed(value: &Option<CString>) -> *const c_char {
    value.as_ref().map_or(ptr::null(), |value| value.as_ptr())
}

fn destination_mode(mode: &DestinationMode) -> (u32, &str) {
    match mode {
        DestinationMode::Xyz => (ROFD_DESTINATION_XYZ, "XYZ"),
        DestinationMode::Fit => (ROFD_DESTINATION_FIT, "Fit"),
        DestinationMode::FitH => (ROFD_DESTINATION_FIT_H, "FitH"),
        DestinationMode::FitV => (ROFD_DESTINATION_FIT_V, "FitV"),
        DestinationMode::FitR => (ROFD_DESTINATION_FIT_R, "FitR"),
        DestinationMode::Unknown(name) => (ROFD_DESTINATION_UNKNOWN, name),
        _ => (ROFD_DESTINATION_UNKNOWN, "Unknown"),
    }
}

impl From<&Action> for OwnedAction {
    fn from(action: &Action) -> Self {
        let (event, event_name) = match &action.event {
            ActionEvent::DocumentOpen => (ROFD_ACTION_EVENT_DOCUMENT_OPEN, "DO"),
            ActionEvent::PageOpen => (ROFD_ACTION_EVENT_PAGE_OPEN, "PO"),
            ActionEvent::Click => (ROFD_ACTION_EVENT_CLICK, "CLICK"),
            ActionEvent::Unknown(name) => (ROFD_ACTION_EVENT_UNKNOWN, name.as_str()),
            _ => (ROFD_ACTION_EVENT_UNKNOWN, "Unknown"),
        };
        let mut result = Self {
            kind: ROFD_ACTION_UNKNOWN,
            event,
            flags: 0,
            type_name: visible_c_string("Unknown"),
            event_name: visible_c_string(event_name),
            uri: None,
            uri_base: None,
            attachment_id: None,
            bookmark: None,
            destination: None,
            destination_mode_name: None,
        };
        match &action.kind {
            ActionKind::Goto {
                destination,
                bookmark,
            } => {
                result.kind = ROFD_ACTION_GOTO;
                result.type_name = visible_c_string("Goto");
                result.bookmark = optional_string(bookmark);
                result.destination = destination.clone();
                result.destination_mode_name = destination
                    .as_ref()
                    .map(|destination| visible_c_string(destination_mode(&destination.mode).1));
            }
            ActionKind::Uri { uri, base } => {
                result.kind = ROFD_ACTION_URI;
                result.type_name = visible_c_string("URI");
                result.uri = Some(visible_c_string(uri));
                result.uri_base = optional_string(base);
            }
            ActionKind::Attachment {
                attachment_id,
                new_window,
            } => {
                result.kind = ROFD_ACTION_ATTACHMENT;
                result.type_name = visible_c_string("GotoA");
                result.attachment_id = Some(visible_c_string(attachment_id));
                result.flags = if *new_window {
                    ROFD_ACTION_NEW_WINDOW
                } else {
                    0
                };
            }
            ActionKind::Unknown { type_name } => {
                result.type_name = visible_c_string(type_name);
            }
            _ => {}
        }
        result
    }
}

impl OwnedAction {
    pub(crate) fn fields(&self) -> ActionFields {
        ActionFields {
            kind: self.kind,
            event: self.event,
            flags: self.flags,
            type_name: self.type_name.as_ptr(),
            event_name: self.event_name.as_ptr(),
            uri: borrowed(&self.uri),
            uri_base: borrowed(&self.uri_base),
            attachment_id: borrowed(&self.attachment_id),
            bookmark: borrowed(&self.bookmark),
        }
    }

    pub(crate) fn destination_fields(&self) -> Result<DestinationFields, FfiError> {
        if self.kind != ROFD_ACTION_GOTO {
            return Err(FfiError::new(
                ROFD_STATUS_UNSUPPORTED,
                "action is not an internal jump",
            ));
        }
        let mut result = DestinationFields {
            kind: ROFD_DESTINATION_UNKNOWN,
            flags: 0,
            page_index: ROFD_NO_INDEX,
            page_id: 0,
            mode_name: borrowed(&self.destination_mode_name),
            left_mm: 0.0,
            top_mm: 0.0,
            right_mm: 0.0,
            bottom_mm: 0.0,
            zoom: 0.0,
        };
        if let Some(destination) = &self.destination {
            result.kind = destination_mode(&destination.mode).0;
            result.page_id = destination.page_id;
            result.flags = ROFD_DESTINATION_HAS_PAGE_ID;
            if let Some(index) = destination.page_index {
                result.flags |= ROFD_DESTINATION_HAS_PAGE_INDEX;
                result.page_index = index;
            }
            for (value, flag) in [
                (destination.left, ROFD_DESTINATION_HAS_LEFT),
                (destination.top, ROFD_DESTINATION_HAS_TOP),
                (destination.right, ROFD_DESTINATION_HAS_RIGHT),
                (destination.bottom, ROFD_DESTINATION_HAS_BOTTOM),
                (destination.zoom, ROFD_DESTINATION_HAS_ZOOM),
            ] {
                if value.is_some() {
                    result.flags |= flag;
                }
            }
            result.left_mm = destination.left.unwrap_or(0.0);
            result.top_mm = destination.top.unwrap_or(0.0);
            result.right_mm = destination.right.unwrap_or(0.0);
            result.bottom_mm = destination.bottom.unwrap_or(0.0);
            result.zoom = destination.zoom.unwrap_or(0.0);
        }
        Ok(result)
    }
}

impl From<&rofd_core::OutlineNode> for OwnedOutlineNode {
    fn from(node: &rofd_core::OutlineNode) -> Self {
        Self {
            title: visible_c_string(&node.title),
            parent: node.parent,
            first_child: node.first_child,
            next_sibling: node.next_sibling,
            expanded: node.expanded,
            actions: node.actions.iter().map(OwnedAction::from).collect(),
        }
    }
}

pub(crate) fn handle_inputs<Token: HandleToken>(handle: *const Token) -> Result<InputRanges, ()> {
    let mut inputs = InputRanges::new();
    inputs.push(handle.cast::<Token::Storage>())?;
    Ok(inputs)
}

unsafe fn outline_ref<'a>(outline: *const rofd_outline_t) -> Result<&'a OutlineHandle, FfiError> {
    if outline.is_null() {
        return Err(FfiError::invalid_argument("outline handle is NULL"));
    }
    // SAFETY: Caller promises a live immutable outline token for this borrow.
    Ok(unsafe { handle_ref(outline) })
}

fn node_at(outline: &OutlineHandle, index: usize) -> Result<&OwnedOutlineNode, FfiError> {
    outline.nodes.get(index).ok_or_else(|| {
        FfiError::new(
            ROFD_STATUS_PAGE_OUT_OF_RANGE,
            format!("outline node index {index} is out of range"),
        )
    })
}

fn action_at(outline: &OutlineHandle, node: usize, index: usize) -> Result<&OwnedAction, FfiError> {
    node_at(outline, node)?.actions.get(index).ok_or_else(|| {
        FfiError::new(
            ROFD_STATUS_PAGE_OUT_OF_RANGE,
            format!("outline action index {index} is out of range"),
        )
    })
}

/// Copies the document's outline into an independently owned preorder snapshot.
///
/// Missing outlines yield an empty non-null snapshot. Navigation is parsed on
/// demand and may add parse warnings. Nodes/actions/strings survive document
/// free; release the snapshot with [`rofd_outline_free`]. No action is executed.
/// Invalid address layouts leave all outputs untouched; overlaps preserve
/// ordinary outputs but may publish an independent error. Other failures null
/// the result. The same transaction rules apply to all outline queries.
///
/// # Safety
/// `document` and `outline` are required. A non-null document must remain live
/// and immutable. Non-null `outline` and optional `error` must be aligned,
/// writable, mutually disjoint and disjoint from document storage. All storage
/// must remain live without conflicting concurrent access for the call.
#[no_mangle]
pub unsafe extern "C" fn rofd_document_get_outline(
    document: *const rofd_document_t,
    outline: *mut *mut rofd_outline_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(document) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: The boundary checks all ranges before borrowing input or writing output.
    unsafe {
        boundary_with_inputs(error, HandleOutput::required(outline), inputs, || {
            Ok(Box::new(OutlineHandle {
                nodes: document_ref(document)?
                    .inner
                    .outline()?
                    .iter()
                    .map(OwnedOutlineNode::from)
                    .collect(),
            }))
        })
    }
}

/// Returns the number of preorder nodes; ordinary failures zero a valid count.
///
/// # Safety
/// `outline` and `count` are required. Non-null outline storage must remain live
/// and immutable; non-null count/optional error slots must be aligned, writable,
/// mutually disjoint and disjoint from it, without conflicting concurrent access.
#[no_mangle]
pub unsafe extern "C" fn rofd_outline_get_count(
    outline: *const rofd_outline_t,
    count: *mut usize,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(outline) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: Range preflight precedes every read/write of the transaction.
    unsafe {
        boundary_with_inputs(error, ScalarOutput::required(count), inputs, || {
            Ok(outline_ref(outline)?.nodes.len())
        })
    }
}

/// Borrows one node by zero-based preorder index, with absent relations NO_INDEX.
///
/// Set the output's `struct_size` to cover v1. Supported prefixes are cleared
/// including padding on ordinary failures, with captured size restored and
/// unknown tails preserved. Undersized records remain untouched. Invalid
/// indices return PAGE_OUT_OF_RANGE. Title remains borrowed until outline free.
///
/// # Safety
/// `outline` and `node` are required. A non-null outline must remain live and
/// immutable. Non-null node storage must be aligned and readable for its
/// initialized size; a supported prefix must be writable. Node/optional error
/// outputs must be mutually disjoint and disjoint from outline storage, remain
/// live, and have no conflicting concurrent access. Borrowed strings are read-only.
#[no_mangle]
pub unsafe extern "C" fn rofd_outline_get_node(
    outline: *const rofd_outline_t,
    index: usize,
    node: *mut rofd_outline_node_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(outline) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: Caller provides a readable initialized size when layout is valid.
    let output = unsafe { OutlineNodeOutput::required(node) };
    // SAFETY: Boundary preflight protects every input and output region.
    unsafe {
        boundary_with_inputs(error, output, inputs, || {
            let node = node_at(outline_ref(outline)?, index)?;
            Ok(OutlineNodeFields {
                expanded: u32::from(node.expanded),
                title: node.title.as_ptr(),
                parent: node.parent.unwrap_or(ROFD_NO_INDEX),
                first_child: node.first_child.unwrap_or(ROFD_NO_INDEX),
                next_sibling: node.next_sibling.unwrap_or(ROFD_NO_INDEX),
                action_count: node.actions.len(),
            })
        })
    }
}

/// Borrows one inert action in its node's declaration order.
///
/// Record transactions follow [`rofd_outline_get_node`]. Strings (including
/// unknown type/event names) live until outline free; embedded NUL is escaped
/// as a visible backslash followed by zero. Irrelevant/absent payloads are null.
/// URI/base values are not resolved, opened or executed by this library.
///
/// # Safety
/// `outline` and `action` are required. A non-null outline must be live and
/// immutable. Non-null action storage must be aligned/readable for initialized
/// size, and writable for a supported prefix. Action/optional error slots must
/// be mutually disjoint and disjoint from outline storage, remain live and
/// without conflicting concurrent access. Borrowed strings must not be modified.
#[no_mangle]
pub unsafe extern "C" fn rofd_outline_get_action(
    outline: *const rofd_outline_t,
    node_index: usize,
    action_index: usize,
    action: *mut rofd_action_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(outline) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: Caller provides a readable initialized size when layout is valid.
    let output = unsafe { ActionOutput::required(action) };
    // SAFETY: Boundary preflight protects every input and output region.
    unsafe {
        boundary_with_inputs(error, output, inputs, || {
            Ok(action_at(outline_ref(outline)?, node_index, action_index)?.fields())
        })
    }
}

/// Borrows destination information for an internal jump action.
///
/// Non-Goto actions return UNSUPPORTED. Unresolved Goto actions succeed with
/// NO_INDEX and without HAS_PAGE_INDEX; missing bookmarks may have no destination
/// fields at all. Never treat an unavailable destination as page zero. Only
/// fields selected by HAS_* bits carry source values; absent numeric fields are
/// zero, and explicit zero zoom means retain current zoom. Coordinates are
/// absolute physical-page millimetres, independent of render DPI/rotation.
/// Record transactions and string lifetimes follow [`rofd_outline_get_node`].
///
/// # Safety
/// `outline` and `destination` are required. A non-null outline must stay live
/// and immutable. Non-null destination storage must be aligned/readable for
/// initialized size and writable for its supported prefix. Destination/optional
/// error outputs must be mutually disjoint and disjoint from outline storage,
/// live for the call and not concurrently accessed in conflict. Borrowed mode
/// strings must not be modified or retained after freeing the outline.
#[no_mangle]
pub unsafe extern "C" fn rofd_outline_get_action_destination(
    outline: *const rofd_outline_t,
    node_index: usize,
    action_index: usize,
    destination: *mut rofd_destination_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    let Ok(inputs) = handle_inputs(outline) else {
        return ROFD_STATUS_INVALID_ARGUMENT;
    };
    // SAFETY: Caller provides a readable initialized size when layout is valid.
    let output = unsafe { DestinationOutput::required(destination) };
    // SAFETY: Boundary preflight protects every input and output region.
    unsafe {
        boundary_with_inputs(error, output, inputs, || {
            action_at(outline_ref(outline)?, node_index, action_index)?.destination_fields()
        })
    }
}

/// Frees an outline snapshot; null is a no-op and all borrowed strings expire.
///
/// # Safety
/// A non-null snapshot must be live, uniquely owned and freed exactly once,
/// without concurrent readers or subsequent uses of its borrowed strings.
#[no_mangle]
pub unsafe extern "C" fn rofd_outline_free(outline: *mut rofd_outline_t) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !outline.is_null() {
            // SAFETY: Caller transfers the unique live allocation exactly once.
            unsafe { drop_raw_handle(outline) };
        }
    }));
}
