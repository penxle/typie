use editor_macros::ffi;
use serde::{Deserialize, Serialize};

/// An IME composition range, expressed in flat-offset coordinates.
///
/// `start` and `end` are **flat offsets** — absolute positions over the
/// entire document, not per-node offsets. Flat offsets are defined by
/// the flat-offset scheme implemented in this crate's `flat` module
/// (see `FlatClass`, `ResolvedPositionFlatExt`).
///
/// A composition can span multiple nodes. The set of nodes covered by
/// a composition is computed on demand by walking the document from
/// the flat range; `Composition` itself stores no node identity and
/// no caching.
#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Composition {
    pub start: usize,
    pub end: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_serde_roundtrip() {
        let c = Composition { start: 3, end: 8 };
        let json = serde_json::to_string(&c).unwrap();
        let back: Composition = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }
}
