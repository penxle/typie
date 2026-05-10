editor_macros::preamble!();

pub mod changeset;
pub mod dot;
pub mod error;
pub mod lwwreg;
pub mod op_graph;
pub mod ormap;
pub mod orset;
pub mod rga;
pub mod sync;
pub mod text;
pub mod to_plain;
pub mod wire;

pub use changeset::Changeset;
pub use dot::{Dot, Dots};
pub use error::CrdtError;
pub use lwwreg::{LwwReg, LwwRegOp};
pub use op_graph::{Op, OpGraph};
pub use ormap::{OrMap, OrMapOp};
pub use orset::{OrSet, OrSetOp};
pub use rga::{Rga, RgaOp};
pub use sync::SyncMessage;
pub use text::{Text, TextOp};
pub use to_plain::ToPlain;

#[cfg(test)]
mod test_utils;

#[cfg(test)]
mod sync_simulator;
