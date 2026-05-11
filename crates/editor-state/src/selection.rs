use editor_macros::ffi;
use editor_model::Doc;
use serde::{Deserialize, Serialize};

use crate::position::Position;
use crate::resolved_selection::ResolvedSelection;

/// A document selection: an ordered pair of positions with directional intent.
///
/// `Selection` is a plain value type (POD) with no automatic validation.
/// Structural invariants (subtree constraint, affinity mutual
/// exclusion, affinity agreement) are the responsibility of
/// command/transaction implementations; constructors do **not**
/// enforce them.
///
/// # `anchor` vs `head`
///
/// - `anchor`: the fixed endpoint of the selection. It stays in place
///   under range-extension operations (shift+arrow, shift+click, etc.).
/// - `head`: the moving endpoint — the caret.
///
/// Direction is **preserved, never normalized**. A selection where
/// `anchor` sorts after `head` (a backward selection) is a distinct,
/// valid state from its forward counterpart. The two differ in which
/// endpoint future range extensions will move, so normalizing would
/// lose user intent.
///
/// # Invariants
///
/// - **Subtree constraint**: `anchor` and `head` must not lie in each
///   other's subtrees. A selection that starts outside a nested node
///   and ends inside it (or vice versa) is not representable.
///   *Upheld by command/transaction implementations; constructors do
///   not enforce this.*
///
/// - **Affinity mutual exclusion (non-collapsed)**: when
///   `anchor != head`, `anchor.affinity` points toward `head` and
///   `head.affinity` points toward `anchor`.
///   *Upheld by command/transaction implementations.*
///
/// - **Affinity agreement (collapsed)**: when `anchor == head` (all
///   three fields of `Position` match), the two affinities are equal.
///   A caret has a single direction; the specific value (Up/Down) is
///   free.
///   *Upheld by command/transaction implementations.*
///
/// # Node selection
///
/// Selecting a non-text node (e.g. clicking an image) is represented
/// the same way as selecting a range of text: by two positions that
/// bracket the target. For `root { paragraph, image, paragraph }`,
/// selecting the image forward produces
///
/// ```text
/// Selection {
///     anchor: Position { node: root, offset: 1, affinity: Downstream },
///     head:   Position { node: root, offset: 2, affinity: Upstream },
/// }
/// ```
///
/// The backward form — `anchor` at offset 2 `Upstream`, `head` at
/// offset 1 `Downstream` — is a distinct valid state representing the
/// same visual selection with the opposite user intent.
#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Selection {
    pub anchor: Position,
    pub head: Position,
}

impl Selection {
    pub fn collapsed(pos: Position) -> Self {
        Self {
            anchor: pos,
            head: pos,
        }
    }

    pub fn new(anchor: Position, head: Position) -> Self {
        Self { anchor, head }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }

    pub fn resolve<'a>(&self, doc: &'a Doc) -> Option<ResolvedSelection<'a>> {
        ResolvedSelection::resolve(doc, *self)
    }

    pub fn freeze(&self, doc: &Doc) -> crate::StableSelection {
        crate::StableSelection::freeze(self, doc)
    }
}
