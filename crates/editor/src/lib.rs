#![cfg_attr(feature = "native", allow(unused))]

#[macro_use]
mod utils;

#[macro_use]
mod test_utils;

mod ffi;
mod font;
mod global;
mod icu_data;
mod inspect;
mod layout;
mod model;
mod render;
mod runtime;
mod schema;
mod state;
mod transaction;
mod types;

#[cfg(any(test, feature = "bench"))]
pub use model::{Doc, Node, NodeId, Text, TextNode};
#[cfg(any(test, feature = "bench"))]
pub use runtime::{Direction, Effect, Message, Runtime, State};
#[cfg(any(test, feature = "bench"))]
pub use state::{Position, Selection, compute_selection_attrs};
#[cfg(any(test, feature = "bench"))]
pub use test_utils::init_test_env;
#[cfg(any(test, feature = "bench"))]
pub use transaction::Transaction;
#[cfg(any(test, feature = "bench"))]
pub use types::{Affinity, Theme};
