editor_macros::preamble!();

pub mod backend;
pub mod error;
pub(crate) mod glyph;
pub mod icons;
pub(crate) mod nodes;
pub mod renderer;
pub mod sink;
pub mod theme;
pub mod theme_data;
pub mod types;

pub use backend::RenderBackend;
pub use backend::gpu::GpuDevice;
pub use backend::kind::BackendKind;
pub use error::RendererError;
pub use renderer::Renderer;
pub use sink::RenderSink;
pub use theme::Theme;
pub use theme_data::ThemeVariant;
pub use types::*;
