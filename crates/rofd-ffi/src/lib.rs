#![allow(non_camel_case_types)]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![doc = "Stable C ABI for rofd."]

mod abi;
mod document;
mod error;
mod handles;
mod renderer;
mod report;

pub use abi::*;
pub use document::{
    rofd_document_free, rofd_document_get_page, rofd_document_get_page_count, rofd_document_open,
    rofd_page_free, rofd_page_get_index, rofd_page_get_size_mm,
};
pub use error::{rofd_error_free, rofd_error_get_message, rofd_error_get_status};
pub use handles::{
    rofd_document_t, rofd_error_t, rofd_page_t, rofd_render_report_t, rofd_renderer_t,
    rofd_string_t, rofd_text_layout_t, rofd_text_search_t, rofd_text_selection_t,
};
pub use renderer::{
    rofd_renderer_free, rofd_renderer_get_pixel_size, rofd_renderer_new,
    rofd_renderer_render_page_cairo,
};
pub use report::{
    rofd_render_report_free, rofd_render_report_get_count, rofd_render_report_get_diagnostic,
};
