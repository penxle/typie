#[cfg(all(feature = "uniffi", feature = "wasm"))]
compile_error!("features \"uniffi\" and \"wasm\" are mutually exclusive");

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

mod backend;
mod convert;
pub mod editor;
mod error;
pub mod host;
mod platform;
mod prelude;
mod types;
