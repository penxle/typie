editor_macros::preamble!();

mod alias;
mod alignment;
mod attr;
mod canonical;
mod edit_op;
mod error;
mod fragment;
mod modifier;
mod node_attr;
mod nodes;
mod plain;
mod projection;
mod schema;
mod seq;
mod span;
mod subtree;
mod view;

#[cfg(any(test, feature = "test-utils"))]
mod test_utils;

pub use alias::{AliasClasses, AliasLog, AliasOp, AliasRun, alias_op_is_valid};
pub use alignment::*;
pub use attr::*;
pub use canonical::*;
pub use edit_op::*;
pub use error::*;
pub use fragment::*;
pub use imbl;
pub use modifier::*;
pub use node_attr::*;
pub use nodes::*;
pub use plain::*;
pub use projection::*;
pub use schema::*;
pub use seq::*;
pub use span::*;
pub use subtree::*;
pub use view::{ChildView, DocView, InlineItem, InlineKind, LeafStateRef, LeafView, NodeView};

#[cfg(any(test, feature = "test-utils"))]
pub use test_utils::*;
