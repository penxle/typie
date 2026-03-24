#![cfg_attr(any(feature = "native", feature = "uniffi"), allow(unused))]

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

#[macro_use]
mod utils;

#[macro_use]
mod test_utils;

mod diagnostics;
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
mod tracing;
mod transaction;
mod types;

#[cfg(any(test, feature = "bench"))]
pub use model::{Doc, Node, NodeId, Text, TextNode};
#[cfg(any(test, feature = "bench"))]
pub use runtime::{Direction, Effect, Message, Runtime, State};
#[cfg(any(test, feature = "bench"))]
pub use state::{Position, Selection, compute_selection_attrs};
#[cfg(any(test, feature = "bench"))]
pub use test_utils::init_bench_env;
#[cfg(any(test, feature = "bench"))]
pub use test_utils::init_test_env;
#[cfg(any(test, feature = "bench"))]
pub use transaction::Transaction;
#[cfg(any(test, feature = "bench"))]
pub use types::{Affinity, Theme};
