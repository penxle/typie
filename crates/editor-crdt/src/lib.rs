editor_macros::preamble!();

pub mod dot;
pub mod error;
pub mod op_graph;
pub mod orset;
pub mod text;

pub use dot::Dot;
pub use error::CrdtError;
pub use op_graph::{Op, OpGraph};
pub use orset::{OrSet, OrSetOp};
pub use text::{TextCrdt, TextOp};

#[cfg(test)]
mod test_utils;
