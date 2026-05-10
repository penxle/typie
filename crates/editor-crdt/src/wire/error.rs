use crate::Dot;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WireError {
    #[error(
        "parent chain rule violated at changeset[{cs_idx}].ops[{op_idx}]: parents = {parents:?}, expected [{expected:?}]"
    )]
    ParentChainViolation {
        cs_idx: usize,
        op_idx: usize,
        parents: Vec<Dot>,
        expected: Dot,
    },

    #[error("Presence op key ({key:#x}) does not match outer node_id ({node_id:#x})")]
    PresenceKeyMismatch { key: u64, node_id: u64 },

    #[error("clock {clock} below baseline {baseline} for actor {actor:#x}")]
    BaselineUnderflow {
        actor: u64,
        clock: u64,
        baseline: u64,
    },

    #[error("truncated input: expected at least {expected} bytes, got {actual}")]
    Truncated { expected: usize, actual: usize },

    #[error("bad magic byte: expected 0xCD, got {got:#04x}")]
    BadMagic { got: u8 },

    #[error("unsupported version: {got}")]
    UnsupportedVersion { got: u8 },

    #[error("required flag bit set but unknown to decoder: flags = {flags:#010b}")]
    RequiredFlagSet { flags: u8 },

    #[error("zstd decompression failed: {0}")]
    Zstd(String),

    #[error("varint overflow / malformed at byte offset {offset}")]
    Varint { offset: usize },

    #[error("actor_idx {idx} out of range (table has {table_len} entries)")]
    ActorIdxOutOfRange { idx: u64, table_len: usize },

    #[error("changeset entry_count = 0 (must be >= 1)")]
    EmptyChangesetEntries,

    #[error("first entry must have node_id_mode = explicit")]
    FirstEntryImplicitNodeId,

    #[error("unknown payload variant tag: {tag}")]
    UnknownPayloadVariant { tag: u8 },

    #[error("run_len = {got} (must be >= 2)")]
    InvalidRunLength { got: u64 },

    #[error("UTF-8 decoding failed in text run: {0}")]
    RunUtf8(String),

    #[error("unknown variant tag {tag} in {ty}")]
    UnknownVariant { ty: &'static str, tag: u8 },

    #[error("trailing bytes after parse complete: {remaining} bytes left")]
    TrailingBytes { remaining: usize },

    #[error("clock arithmetic overflow ({context}): base {base} + delta {delta}")]
    ClockOverflow {
        context: &'static str,
        base: u64,
        delta: u64,
    },

    #[error("changeset has empty ops vec (must contain at least one op)")]
    EmptyChangesetOps,

    #[error("run entry tag bits 5-0 must be zero: tag = {tag:#010b}")]
    RunTagBitsNonZero { tag: u8 },

    #[error(
        "variant subflag bits {bits:#04b} non-zero in slot reserved as zero for variant {variant}"
    )]
    InvalidSubflag { variant: &'static str, bits: u8 },

    #[error("integer value {value} exceeds {ty}::MAX ({max})")]
    IntOverflow {
        ty: &'static str,
        value: u64,
        max: u64,
    },

    #[error("invalid bool tag {tag} (must be 0 or 1)")]
    InvalidBool { tag: u8 },
}

pub type WireResult<T> = Result<T, WireError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_context() {
        let e = WireError::BadMagic { got: 0x42 };
        assert!(format!("{e}").contains("0x42"));
    }
}
