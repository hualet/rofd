/// Handling mode for recoverable conformance problems.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Strictness {
    /// Reject every conformance problem encountered by the implemented parser.
    Strict,
    /// Continue past recoverable problems and expose warnings.
    Lenient,
}

/// Limits applied before decompressed package data is accepted.
#[derive(Clone, Debug)]
pub struct ResourceLimits {
    /// Maximum number of ZIP entries.
    pub max_entries: usize,
    /// Maximum uncompressed size of one entry.
    pub max_entry_size: u64,
    /// Maximum sum of declared uncompressed entry sizes.
    pub max_total_size: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_entry_size: 64 * 1024 * 1024,
            max_total_size: 512 * 1024 * 1024,
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
