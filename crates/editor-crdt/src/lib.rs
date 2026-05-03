editor_macros::preamble!();

pub mod dot;
pub mod error;
pub mod lwwreg;
pub mod op_graph;
pub mod orset;
pub mod rga;
pub mod text;

pub use dot::Dot;
pub use error::CrdtError;
pub use lwwreg::{LwwReg, LwwRegOp};
pub use op_graph::{Op, OpGraph};
pub use orset::{OrSet, OrSetOp};
pub use rga::{Rga, RgaOp};
pub use text::{Text, TextOp};

#[cfg(test)]
mod test_utils;
