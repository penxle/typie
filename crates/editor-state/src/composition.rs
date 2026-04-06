/// An IME composition range, expressed in flat-offset coordinates.
///
/// `start` and `end` are **flat offsets** — absolute positions over the
/// entire document, not per-node offsets. Flat offsets are defined by
/// the flat-offset scheme implemented in the `editor-schema::flat`
/// module (see `FlatClass`, `ResolvedPositionFlatExt`).
///
/// A composition can span multiple nodes. The set of nodes covered by
/// a composition is computed on demand by walking the document from
/// the flat range; `Composition` itself stores no node identity and
/// no caching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Composition {
    pub start: usize,
    pub end: usize,
}
