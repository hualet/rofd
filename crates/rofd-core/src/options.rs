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
    /// Maximum number of commands accepted in one abbreviated path.
    ///
    /// The default of 250,000 accommodates complex pages while bounding the
    /// memory occupied by the parsed command vector.
    pub max_path_commands: usize,
    /// Maximum number of layer, group, and leaf object IDs on one page.
    pub max_page_objects: usize,
    /// Maximum nesting depth of page blocks on one page.
    pub max_page_block_depth: usize,
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
