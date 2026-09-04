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
    /// Each `Clip`, `Area`, and clip `Path` consumes one unit even though
    /// these structures do not have object IDs. The limit is enforced during
    /// XML preflight, before recursive deserialization.
    pub max_page_objects: usize,
    /// Maximum nesting depth of page blocks on one page.
    pub max_page_block_depth: usize,
    /// Maximum nesting depth of elements in any parsed XML document.
    pub max_xml_depth: usize,
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
        }
    }
}

/// Options used when opening an OFD package.
#[derive(Clone, Debug)]
pub struct LoadOptions {
    /// Conformance handling mode.
    pub strictness: Strictness,
    /// ZIP resource limits.
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
