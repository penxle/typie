//! Browser surface adapter for the shared Prism `wgpu` renderer.

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_arch = "wasm32")]
pub use web::*;
