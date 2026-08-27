pub mod diff;
pub mod edit;
pub mod error;
pub mod lcs;
pub mod lexer;
pub mod names;
pub mod reader;
pub mod tree;
pub mod writer;

#[cfg(test)]
mod perf;
#[cfg(test)]
mod props;
#[cfg(test)]
mod test_support;

pub use diff::{ChangeCounts, Diff, DiffOutcome};
pub use edit::{EditOutcome, edit, validate_against};
pub use error::{Pos, XmlError, XmlErrorDetail};
pub use reader::from_xml;
pub use tree::{InlineEntry, InlineLeaf, XmlChild, XmlNode, XmlTree};
pub use writer::{decode_base, encode_base, to_xml};
