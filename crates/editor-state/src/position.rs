use editor_macros::ffi;
use editor_model::{Doc, NodeId};
use serde::{Deserialize, Serialize};

use crate::affinity::Affinity;
use crate::resolved_position::ResolvedPosition;

/// A document position: the triple `(node_id, offset, affinity)`.
///
/// `Position` is a plain value type (POD) with no automatic validation.
/// Its invariants are documented below; violating positions either
/// resolve to `None` via [`Position::resolve`] (value-level invariants)
/// or produce incorrect behavior in downstream code (structural
/// invariants).
///
/// # Invariants
///
/// - `node_id` must refer to a **text node** or a **container node**
///   (a node whose schema allows children). Non-text leaf nodes
///   (e.g. `hard_break`, `horizontal_rule`, `image`, `page_break`,
///   `embed`, `file`) must **never** appear as `node_id`; such
///   locations are represented by the parent container's boundary
///   (the offset between the siblings of the leaf).
///   *Not currently enforced.*
///
/// - `offset` must lie within the node's valid range:
///   - Text node: `0..=char_count` (unicode codepoint units, **not** bytes).
///   - Container node: `0..=children.len()`.
///   *Not currently enforced.*
///
/// # Semantics of `offset`
///
/// `offset` names the **boundary between** elements, not an element itself.
///
/// - In a **text node**, `offset` is a unicode codepoint index between
///   chars. For `"hello"`, offset `0` is before `'h'`, offset `5` is
///   after `'o'`.
/// - In a **container node**, `offset` is an index between children.
///   For `blockquote { p1, p2, p3 }`, `offset: 1` names the boundary
///   **between `p1` and `p2`** — it does NOT point at `p2` itself.
///   - Empty container cursor: `offset = 0`.
///   - End of container: `offset = children.len()` (e.g. 3 in the
///     example above — the position after `p3`).
///
/// # Semantics of `affinity`
///
/// See [`Affinity`].
#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Position {
    pub node_id: NodeId,
    pub offset: usize,
    #[serde(default)]
    pub affinity: Affinity,
}

impl Position {
    pub fn new(node_id: NodeId, offset: usize) -> Self {
        Self {
            node_id,
            offset,
            affinity: Affinity::default(),
        }
    }

    pub fn resolve<'a>(&self, doc: &'a Doc) -> Option<ResolvedPosition<'a>> {
        ResolvedPosition::resolve(doc, *self)
    }
}
