#![allow(non_camel_case_types)]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![doc = "Stable C ABI for rofd."]

mod abi;
mod error;
mod handles;

pub use abi::*;
pub use error::{rofd_error_free, rofd_error_get_message, rofd_error_get_status};
pub use handles::{
    rofd_document_t, rofd_error_t, rofd_page_t, rofd_render_report_t, rofd_renderer_t,
};
