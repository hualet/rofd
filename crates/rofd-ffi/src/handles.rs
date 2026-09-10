use crate::rofd_status_t;
use std::ffi::CString;

macro_rules! opaque_handle {
    ($name:ident, $documentation:literal) => {
        #[doc = $documentation]
        #[repr(C)]
        pub struct $name {
            _private: [u8; 0],
        }
    };
}

opaque_handle!(rofd_document_t, "Opaque owned OFD document handle.");
opaque_handle!(
    rofd_metadata_t,
    "Opaque independently owned document metadata snapshot."
);
opaque_handle!(
    rofd_warning_list_t,
    "Opaque independently owned parse-warning snapshot."
);
opaque_handle!(rofd_page_t, "Opaque owned OFD page handle.");
opaque_handle!(
    rofd_outline_t,
    "Opaque independently owned document outline snapshot."
);
opaque_handle!(rofd_renderer_t, "Opaque owned OFD renderer handle.");
opaque_handle!(
    rofd_render_report_t,
    "Opaque owned OFD render report handle."
);
opaque_handle!(rofd_error_t, "Opaque owned OFD error handle.");
opaque_handle!(rofd_string_t, "Opaque owned UTF-8 string handle.");
opaque_handle!(rofd_text_layout_t, "Opaque owned page text layout handle.");
opaque_handle!(
    rofd_text_search_t,
    "Opaque owned text-search result handle."
);
opaque_handle!(
    rofd_text_selection_t,
    "Opaque owned text-selection result handle."
);

pub(crate) struct ErrorHandle {
    pub(crate) status: rofd_status_t,
    pub(crate) message: CString,
}

pub(crate) struct DocumentHandle {
    pub(crate) inner: rofd_core::Document,
}

pub(crate) struct MetadataHandle {
    pub(crate) document_id: Option<CString>,
    pub(crate) title: Option<CString>,
    pub(crate) author: Option<CString>,
    pub(crate) subject: Option<CString>,
    pub(crate) abstract_text: Option<CString>,
    pub(crate) creator: Option<CString>,
    pub(crate) creator_version: Option<CString>,
    pub(crate) creation_date: Option<CString>,
    pub(crate) modification_date: Option<CString>,
    pub(crate) keywords: Vec<CString>,
}

pub(crate) struct OwnedWarning {
    pub(crate) code: u32,
    pub(crate) path: CString,
    pub(crate) message: CString,
}

pub(crate) struct WarningListHandle {
    pub(crate) warnings: Vec<OwnedWarning>,
}

pub(crate) struct PageHandle {
    pub(crate) inner: rofd_core::Page,
}

pub(crate) struct OwnedAction {
    pub(crate) kind: u32,
    pub(crate) event: u32,
    pub(crate) flags: u32,
    pub(crate) type_name: CString,
    pub(crate) event_name: CString,
    pub(crate) uri: Option<CString>,
    pub(crate) uri_base: Option<CString>,
    pub(crate) attachment_id: Option<CString>,
    pub(crate) bookmark: Option<CString>,
    pub(crate) destination: Option<rofd_core::Destination>,
    pub(crate) destination_mode_name: Option<CString>,
}

pub(crate) struct OwnedOutlineNode {
    pub(crate) title: CString,
    pub(crate) parent: Option<usize>,
    pub(crate) first_child: Option<usize>,
    pub(crate) next_sibling: Option<usize>,
    pub(crate) expanded: bool,
    pub(crate) actions: Vec<OwnedAction>,
}

pub(crate) struct OutlineHandle {
    pub(crate) nodes: Vec<OwnedOutlineNode>,
}

pub(crate) struct RendererHandle {
    pub(crate) font_resolver: rofd_render::SystemFontResolver,
    pub(crate) image_decoder: rofd_render::ImageDecoder,
}

pub(crate) struct OwnedDiagnostic {
    pub(crate) kind: u32,
    pub(crate) object_id: u64,
    pub(crate) message: CString,
}

pub(crate) struct RenderReportHandle {
    pub(crate) diagnostics: Vec<OwnedDiagnostic>,
}

pub(crate) struct StringHandle {
    pub(crate) bytes: CString,
}

pub(crate) struct TextLayoutHandle {
    pub(crate) characters: Vec<rofd_core::TextChar>,
}

pub(crate) struct TextSearchHandle {
    pub(crate) matches: Vec<rofd_core::TextMatch>,
}

pub(crate) struct TextSelectionHandle {
    pub(crate) text: CString,
    pub(crate) regions: Vec<rofd_core::Rect>,
}

mod token_private {
    pub(super) trait Sealed {}
}

/// Compile-time mapping from an opaque C token to its one Rust storage type.
///
/// The private supertrait keeps mappings centralized in this module, so an FFI
/// output cannot pair a token with unrelated storage.
#[allow(private_bounds)]
pub(crate) trait HandleToken: token_private::Sealed {
    type Storage;
}

impl token_private::Sealed for rofd_error_t {}

impl HandleToken for rofd_error_t {
    type Storage = ErrorHandle;
}

impl token_private::Sealed for rofd_document_t {}

impl HandleToken for rofd_document_t {
    type Storage = DocumentHandle;
}

impl token_private::Sealed for rofd_metadata_t {}

impl HandleToken for rofd_metadata_t {
    type Storage = MetadataHandle;
}

impl token_private::Sealed for rofd_warning_list_t {}

impl HandleToken for rofd_warning_list_t {
    type Storage = WarningListHandle;
}

impl token_private::Sealed for rofd_page_t {}

impl HandleToken for rofd_page_t {
    type Storage = PageHandle;
}

impl token_private::Sealed for rofd_outline_t {}

impl HandleToken for rofd_outline_t {
    type Storage = OutlineHandle;
}

impl token_private::Sealed for rofd_renderer_t {}

impl HandleToken for rofd_renderer_t {
    type Storage = RendererHandle;
}

impl token_private::Sealed for rofd_render_report_t {}

impl HandleToken for rofd_render_report_t {
    type Storage = RenderReportHandle;
}

impl token_private::Sealed for rofd_string_t {}

impl HandleToken for rofd_string_t {
    type Storage = StringHandle;
}

impl token_private::Sealed for rofd_text_layout_t {}

impl HandleToken for rofd_text_layout_t {
    type Storage = TextLayoutHandle;
}

impl token_private::Sealed for rofd_text_search_t {}

impl HandleToken for rofd_text_search_t {
    type Storage = TextSearchHandle;
}

impl token_private::Sealed for rofd_text_selection_t {}

impl HandleToken for rofd_text_selection_t {
    type Storage = TextSelectionHandle;
}

pub(crate) fn into_raw_handle<Token: HandleToken>(storage: Box<Token::Storage>) -> *mut Token {
    Box::into_raw(storage).cast()
}

pub(crate) unsafe fn handle_ref<'a, Token: HandleToken>(token: *const Token) -> &'a Token::Storage {
    // SAFETY: The caller guarantees that token came from into_raw_handle for this exact sealed
    // Token mapping, remains live for 'a, and is not mutably accessed during the borrow.
    unsafe { &*token.cast::<Token::Storage>() }
}

pub(crate) unsafe fn drop_raw_handle<Token: HandleToken>(token: *mut Token) {
    // SAFETY: The caller transfers the unique allocation created by into_raw_handle for this
    // exact sealed Token mapping. Box::from_raw occurs exactly once for that allocation.
    unsafe { drop(Box::from_raw(token.cast::<Token::Storage>())) };
}

#[cfg(test)]
pub(crate) struct TestHandleToken {
    _private: [u8; 0],
}

#[cfg(test)]
pub(crate) struct TestHandleStorage {
    drops: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(test)]
impl TestHandleStorage {
    pub(crate) fn new(drops: std::sync::Arc<std::sync::atomic::AtomicUsize>) -> Self {
        Self { drops }
    }
}

#[cfg(test)]
impl Drop for TestHandleStorage {
    fn drop(&mut self) {
        self.drops.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
impl token_private::Sealed for TestHandleToken {}

#[cfg(test)]
impl HandleToken for TestHandleToken {
    type Storage = TestHandleStorage;
}

#[cfg(test)]
mod tests {
    use super::{
        DocumentHandle, PageHandle, RenderReportHandle, RendererHandle, StringHandle,
        TextLayoutHandle, TextSearchHandle, TextSelectionHandle,
    };

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn owned_handle_storage_is_send_and_sync() {
        assert_send_sync::<super::OutlineHandle>();
        assert_send_sync::<DocumentHandle>();
        assert_send_sync::<super::MetadataHandle>();
        assert_send_sync::<super::WarningListHandle>();
        assert_send_sync::<PageHandle>();
        assert_send_sync::<RendererHandle>();
        assert_send_sync::<RenderReportHandle>();
        assert_send_sync::<StringHandle>();
        assert_send_sync::<TextLayoutHandle>();
        assert_send_sync::<TextSearchHandle>();
        assert_send_sync::<TextSelectionHandle>();
    }
}
