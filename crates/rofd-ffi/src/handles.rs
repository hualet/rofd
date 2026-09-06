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
opaque_handle!(rofd_page_t, "Opaque owned OFD page handle.");
opaque_handle!(rofd_renderer_t, "Opaque owned OFD renderer handle.");
opaque_handle!(
    rofd_render_report_t,
    "Opaque owned OFD render report handle."
);
opaque_handle!(rofd_error_t, "Opaque owned OFD error handle.");

pub(crate) struct ErrorHandle {
    pub(crate) status: rofd_status_t,
    pub(crate) message: CString,
}

pub(crate) struct DocumentHandle {
    pub(crate) inner: rofd_core::Document,
}

pub(crate) struct PageHandle {
    pub(crate) inner: rofd_core::Page,
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

impl token_private::Sealed for rofd_page_t {}

impl HandleToken for rofd_page_t {
    type Storage = PageHandle;
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
    use super::{DocumentHandle, PageHandle};

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn document_and_page_storage_are_send_and_sync() {
        assert_send_sync::<DocumentHandle>();
        assert_send_sync::<PageHandle>();
    }
}
