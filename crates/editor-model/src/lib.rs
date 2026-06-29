editor_macros::preamble!();

mod alignment;
mod attr;
mod canonical;
mod edit_op;
mod error;
mod fragment;
mod marker;
mod modifier;
mod node_attr;
mod node_lww;
mod nodes;
mod plain;
mod projection;
mod schema;
mod seq;
mod span;
mod style;
mod style_log;
mod subtree;
mod view;

#[cfg(any(test, feature = "test-utils"))]
mod test_utils;

pub use alignment::*;
pub use attr::*;
pub use canonical::*;
pub use edit_op::*;
pub use error::*;
pub use fragment::*;
pub use imbl;
pub use marker::Marker;
pub use modifier::*;
pub use node_attr::*;
pub use node_lww::*;
pub use nodes::*;
pub use plain::*;
pub use projection::*;
pub use schema::*;
pub use seq::*;
pub use span::*;
pub use style::*;
pub use style_log::*;
pub use subtree::*;
pub use view::{ChildView, DocView, InlineItem, InlineKind, LeafView, NodeView};

#[cfg(any(test, feature = "test-utils"))]
pub use test_utils::*;
