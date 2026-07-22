editor_macros::preamble!();

mod dissolve;
mod effect;
mod error;
mod fulfill;
mod materialize;
mod meta;
mod prune;
mod revert;
mod step;
mod steps;
mod transaction;

#[cfg(test)]
mod test_utils;

pub use dissolve::dissolve;
pub use effect::*;
pub use error::*;
pub use fulfill::{first_child_type, fulfill, minimal_subtree};
pub use materialize::{can_materialize_repair_target, materialize_repair_target};
pub use meta::*;
pub use prune::prune;
pub use revert::*;
pub use step::*;
pub use steps::move_node::MovedNode;
pub use steps::move_nodes_into::{MoveDest, MovedItem};
pub use steps::support::{capture_subtree, delete_dots_ops};
pub use transaction::*;
