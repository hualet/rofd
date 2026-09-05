/// Handling mode for recoverable conformance problems.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Strictness {
    /// Reject every conformance problem encountered by the implemented parser.
    Strict,
    /// Continue past recoverable problems and expose warnings.
    Lenient,
}

/// Limits applied while loading and parsing package data.
#[derive(Clone, Debug)]
pub struct ResourceLimits {
    /// Maximum number of ZIP entries.
    pub max_entries: usize,
    /// Maximum uncompressed size of one entry.
    pub max_entry_size: u64,
    /// Maximum sum of declared uncompressed entry sizes.
    pub max_total_size: u64,
    /// Maximum number of path commands accepted by the configured operation.
    ///
    /// [`crate::PathData::parse`] uses the default value as a standalone
    /// per-call cap, while [`crate::PathData::parse_with_limit`] accepts the
    /// equivalent cap explicitly. During [`crate::Document`] page conversion,
    /// this configured value is a cumulative budget shared by every path on
    /// one page. The default is 250,000 commands.
    pub max_path_commands: usize,
    /// Maximum number of layers, groups, leaf objects, and clip structures on
    /// one page.
    ///
    /// Each template reference, `Clip`, `Area`, and immediate clip `Path` or
    /// `Text` child consumes one unit even though these structures do not have
    /// object IDs. The limit is enforced during XML preflight, before recursive
    /// deserialization, and again across effective template expansion.
    pub max_page_objects: usize,
    /// Maximum nesting depth of page blocks or template references on one page.
    pub max_page_block_depth: usize,
    /// Maximum nesting depth of elements in any parsed XML document.
    pub max_xml_depth: usize,
    /// Maximum number of resource catalog files declared by one document.
    pub max_resource_files: usize,
    /// Maximum total number of resources across all document catalogs.
    pub max_resources: usize,
    /// Maximum encoded byte length of one embedded font.
    pub max_font_bytes: u64,
    /// Maximum encoded byte length of one image.
    pub max_encoded_image_bytes: u64,
    /// Maximum decoded pixel count of one image, reserved for image decoding.
    pub max_decoded_image_pixels: u64,
    /// Maximum decoded byte length of one image, reserved for image decoding.
    pub max_decoded_image_bytes: u64,
    /// Maximum text characters expanded on one page, reserved for text rendering.
    pub max_text_characters_per_page: usize,
    /// Maximum glyph count on one page, reserved for text rendering.
    pub max_glyphs_per_page: usize,
    /// Maximum text-run, glyph-map, expanded-delta, and glyph entries on one page.
    pub max_text_expansion_entries: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_entry_size: 64 * 1024 * 1024,
            max_total_size: 512 * 1024 * 1024,
            max_path_commands: 250_000,
            max_page_objects: 100_000,
            max_page_block_depth: 64,
            max_xml_depth: 256,
            max_resource_files: 32,
            max_resources: 100_000,
            max_font_bytes: 64 * 1024 * 1024,
            max_encoded_image_bytes: 64 * 1024 * 1024,
            max_decoded_image_pixels: 100_000_000,
            max_decoded_image_bytes: 400 * 1024 * 1024,
            max_text_characters_per_page: 1_000_000,
            max_glyphs_per_page: 1_000_000,
            max_text_expansion_entries: 2_000_000,
        }
    }
}

/// Options used when opening an OFD package.
#[derive(Clone, Debug)]
pub struct LoadOptions {
    /// Conformance handling mode.
    pub strictness: Strictness,
    /// Package, resource, page, and XML parsing limits.
    pub limits: ResourceLimits,
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self {
            strictness: Strictness::Lenient,
            limits: ResourceLimits::default(),
        }
    }
}
