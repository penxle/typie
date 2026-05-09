use editor_macros::ffi;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::Op;

/// Atomic application unit. Wire / db row / FFI boundary.
///
/// `ops` must be in parents-before-children topological order — sender's
/// responsibility. The receiver (`OpGraph::receive_changeset` Phase A)
/// rejects out-of-order ops via the per-op parents-known check. Standard
/// sender APIs (`OpGraph::topo_sort`, `OpGraph::missing_changesets_for`,
/// sequential `OpGraph::add` followed by `OpGraph::commit`) satisfy this
/// naturally.
#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct Changeset<P> {
    #[n(0)]
    pub ops: Vec<Op<P>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
#[cbor(transparent)]
pub struct Changesets<P>(#[n(0)] pub Vec<Changeset<P>>);
