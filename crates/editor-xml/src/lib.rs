pub mod address;
pub mod diff;
pub mod edit;
pub mod error;
pub mod lcs;
pub mod lexer;
pub mod names;
pub mod ops;
pub mod outline;
pub mod reader;
pub mod tree;
pub mod write_tree;
pub mod writer;

#[cfg(test)]
mod perf;
#[cfg(test)]
mod props;
#[cfg(test)]
mod test_support;

pub use address::{Address, NodePath};
pub use diff::{ChangeCounts, Diff, DiffOutcome};
pub use edit::{EditOutcome, edit, validate_against};
pub use error::{Pos, XmlError, XmlErrorDetail};
pub use ops::{Applied, At, Edited, Op, OpError, SetAttr, apply_ops, edit_file};
pub use outline::{OutlineResult, OutlineRow, OutlineScope, outline, outline_at};
pub use reader::{from_xml, from_xml_fragment};
pub use tree::{InlineEntry, InlineLeaf, XmlChild, XmlNode, XmlTree};
pub use write_tree::{BlockSpan, write_fragment, write_tree};
pub use writer::{decode_base, encode_base, to_xml};
