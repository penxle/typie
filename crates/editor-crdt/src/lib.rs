editor_macros::preamble!();

pub mod changeset;
pub mod dot;
mod dot_map;
pub use dot_map::DotMap;
pub mod error;
pub mod lwwreg;
pub mod op_graph;
pub mod oplog;
pub mod ormap;
pub mod orset;
pub mod sequence;
pub mod sync;
pub mod to_plain;

pub use changeset::Changeset;
pub use dot::{Dot, Dots, OpDot};
pub use error::CrdtError;
pub use lwwreg::{LwwReg, LwwRegOp};
pub use op_graph::{ChangesetRef, Op, OpGraph};
pub use oplog::{InputEvent, ListOp, OpLog, build_oplog};
pub use ormap::{OrMap, OrMapOp};
pub use orset::{OrSet, OrSetOp};
pub use sync::SyncMessage;
pub use to_plain::ToPlain;

/// `imbl` map/set with a fast non-cryptographic hasher — the default
/// `RandomState` (SipHash) shows up in profiles when every dot lookup hashes
/// through it. Keys here are u64 actors / 16-byte dots, not attacker-chosen
/// hash-flood surfaces.
pub type FastMap<K, V> =
    imbl::GenericHashMap<K, V, hashbrown::DefaultHashBuilder, imbl::shared_ptr::DefaultSharedPtr>;
pub type FastSet<A> =
    imbl::GenericHashSet<A, hashbrown::DefaultHashBuilder, imbl::shared_ptr::DefaultSharedPtr>;

#[cfg(test)]
mod test_utils;

#[cfg(test)]
mod sync_simulator;
