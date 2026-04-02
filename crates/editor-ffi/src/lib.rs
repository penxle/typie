editor_macros::preamble!();

#[cfg(all(feature = "uniffi", feature = "wasm"))]
compile_error!("features \"uniffi\" and \"wasm\" are mutually exclusive");

#[cfg(all(feature = "wasm-browser", feature = "wasm-server"))]
compile_error!("features \"wasm-browser\" and \"wasm-server\" are mutually exclusive");

#[cfg(not(feature = "wasm-server"))]
mod backend;
mod convert;
pub mod editor;
mod error;
pub mod host;
#[cfg(not(feature = "wasm-server"))]
mod platform;
mod prelude;
#[cfg(feature = "wasm-server")]
mod server;
