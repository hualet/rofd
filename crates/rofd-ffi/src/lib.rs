#![allow(non_camel_case_types)]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![doc = "Stable C ABI for rofd."]

mod abi;
mod document;
mod error;
mod handles;
mod metadata;
mod region;
mod renderer;
mod report;
mod semantic;

pub use abi::*;
pub use document::{
    rofd_document_free, rofd_document_get_page, rofd_document_get_page_count, rofd_document_open,
    rofd_page_free, rofd_page_get_index, rofd_page_get_size_mm,
};
pub use error::{rofd_error_free, rofd_error_get_message, rofd_error_get_status};
pub use handles::{
    rofd_document_t, rofd_error_t, rofd_metadata_t, rofd_page_t, rofd_render_report_t,
    rofd_renderer_t, rofd_string_t, rofd_text_layout_t, rofd_text_search_t, rofd_text_selection_t,
    rofd_warning_list_t,
};
pub use metadata::{
    rofd_document_get_metadata, rofd_document_get_warnings, rofd_metadata_free,
    rofd_metadata_get_abstract, rofd_metadata_get_author, rofd_metadata_get_creation_date,
    rofd_metadata_get_creator, rofd_metadata_get_creator_version, rofd_metadata_get_document_id,
    rofd_metadata_get_keyword, rofd_metadata_get_keyword_count,
    rofd_metadata_get_modification_date, rofd_metadata_get_subject, rofd_metadata_get_title,
    rofd_warning_list_free, rofd_warning_list_get_count, rofd_warning_list_get_warning,
};
pub use region::{rofd_renderer_get_pixel_canvas_size, rofd_renderer_render_page_region_cairo};
pub use renderer::{
    rofd_renderer_free, rofd_renderer_get_pixel_size, rofd_renderer_new,
    rofd_renderer_render_page_cairo,
};
pub use report::{
    rofd_render_report_free, rofd_render_report_get_count, rofd_render_report_get_diagnostic,
};
pub use semantic::{
    rofd_page_find_text, rofd_page_find_text_with_options, rofd_page_get_selected_text,
    rofd_page_get_text, rofd_page_get_text_for_area, rofd_page_get_text_layout, rofd_string_free,
    rofd_string_get_data, rofd_string_get_length, rofd_text_layout_free, rofd_text_layout_get_char,
    rofd_text_layout_get_count, rofd_text_search_free, rofd_text_search_get_count,
    rofd_text_search_get_match, rofd_text_selection_free, rofd_text_selection_get_region,
    rofd_text_selection_get_region_count, rofd_text_selection_get_text,
    rofd_text_selection_get_text_length,
};
