#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CodecError {
    #[error(transparent)]
    Corruption(#[from] Corruption),
    #[error(transparent)]
    Fenced(#[from] Fenced),
    #[error(transparent)]
    Encode(#[from] EncodeInvariant),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Corruption {
    #[error("truncated input: expected at least {expected} bytes, got {actual}")]
    Truncated { expected: usize, actual: usize },

    #[error("bad magic byte: got {got:#04x}")]
    BadMagic { got: u8 },

    #[error("checksum mismatch")]
    ChecksumMismatch,

    #[error("unknown tag {tag} for closed type {ty}")]
    UnknownClosedTag { ty: &'static str, tag: u64 },

    #[error("invalid utf-8")]
    InvalidUtf8,

    #[error("invalid char scalar value: {got:#x}")]
    InvalidChar { got: u32 },

    #[error("invalid bool tag: {got} (must be 0 or 1)")]
    InvalidBool { got: u8 },

    #[error("declared length {declared} exceeds remaining {remaining} bytes")]
    LengthOverflow { declared: u64, remaining: usize },

    #[error("trailing bytes after parse complete: {remaining} bytes left")]
    TrailingBytes { remaining: usize },

    #[error("varint overflow")]
    VarintOverflow,

    #[error("non-canonical varint encoding")]
    NonCanonicalVarint,

    #[error("actor table is not strictly sorted/deduplicated")]
    NonCanonicalActorTable,

    #[error("clock arithmetic overflow")]
    ClockOverflow,

    #[error("actor index {idx} out of range (table has {table_len} entries)")]
    ActorIndexOutOfRange { idx: u64, table_len: usize },

    #[error("reserved flag bits set: {got:#010b}")]
    ReservedFlagBits { got: u8 },

    #[error("body too large: declared {declared}, max {max}")]
    BodyTooLarge { declared: u64, max: u64 },

    #[error("decompressed length mismatch: declared {declared}, actual {actual}")]
    RawLenMismatch { declared: u64, actual: u64 },

    #[error("decoder made no progress")]
    NoProgress,

    #[error("missing required field {field} on {ty}")]
    MissingRequiredField {
        ty: &'static str,
        field: &'static str,
    },

    #[error("zstd decompression failed: {0}")]
    Zstd(String),

    #[error("changeset has empty ops (must contain at least one record)")]
    EmptyChangesetOps,

    #[error("invalid parents marker: {marker} (must be 0, 1, or 2)")]
    InvalidParentsMarker { marker: u8 },

    #[error("explicit parents list must not be empty")]
    EmptyExplicitParents,

    #[error("implicit parents marker used without a preceding changeset")]
    ImplicitPrevWithoutPredecessor,

    #[error("parents marker is not in canonical form for its value")]
    NonCanonicalParentsMarker,

    #[error("envelope payload kind {kind} is not valid for this decode path")]
    UnexpectedPayloadKind { kind: u8 },

    #[error("missing required record field: {field}")]
    MissingRecordField { field: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Fenced {
    #[error("unsupported format version: got {got}, supported {supported}")]
    FormatVersion { got: u8, supported: u8 },

    #[error("unknown required features: {unknown_bits:#x}")]
    RequiredFeatures { unknown_bits: u64 },

    #[error("unsupported epoch: {got}")]
    Epoch { got: u64 },

    #[error("unknown payload kind: {got}")]
    PayloadKind { got: u8 },

    #[error("data is newer than this reader can losslessly re-encode")]
    LossyForReencode,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncodeInvariant {
    #[error("clock {clock} below baseline {baseline} for actor {actor:#x}")]
    BaselineUnderflow {
        actor: u64,
        clock: u64,
        baseline: u64,
    },

    #[error("actor {actor:#x} not present in the collected actor table")]
    ActorNotCollected { actor: u64 },

    #[error("unsupported required features: {bits:#x}")]
    UnsupportedRequiredFeatures { bits: u64 },

    #[error("unsupported epoch: {got}")]
    UnsupportedEpoch { got: u64 },

    #[error("body too large: {len}, max {max}")]
    BodyTooLarge { len: u64, max: u64 },

    #[error("actor table length mismatch: {actors} actors, {baselines} baselines")]
    MismatchedActorTable { actors: usize, baselines: usize },

    #[error("actor table is not strictly sorted/deduplicated")]
    NonCanonicalActorTable,

    #[error("vec element encoded to zero bytes")]
    ZeroWidthVecElement,

    #[error("cannot encode a record with an unknown/preserved payload")]
    UnknownPayloadEncode,

    #[error("parents list is not in canonical (sorted, deduplicated) form")]
    NonCanonicalParents,

    #[error("changeset must contain at least one record")]
    EmptyChangeset,
}

pub type CodecResult<T> = Result<T, CodecError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corruption_and_fenced_are_distinct_arms() {
        let c: CodecError = Corruption::ChecksumMismatch.into();
        let f: CodecError = Fenced::Epoch { got: 3 }.into();
        assert!(matches!(c, CodecError::Corruption(_)));
        assert!(matches!(f, CodecError::Fenced(_)));
        assert_ne!(c, f);
    }

    #[test]
    fn fenced_carries_structured_info() {
        let f = Fenced::RequiredFeatures {
            unknown_bits: 0b1010,
        };
        match f {
            Fenced::RequiredFeatures { unknown_bits } => assert_eq!(unknown_bits, 0b1010),
            _ => unreachable!(),
        }
    }

    #[test]
    fn display_includes_context() {
        let e = Corruption::BadMagic { got: 0x42 };
        assert!(format!("{e}").contains("0x42"));
        let e: CodecError = Corruption::ActorIndexOutOfRange {
            idx: 5,
            table_len: 1,
        }
        .into();
        assert!(format!("{e}").contains('5'));
    }
}
