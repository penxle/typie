editor_macros::preamble!();

pub mod html;
pub mod payload;
pub mod slice;
pub mod text;

#[cfg(test)]
pub(crate) mod test_doc;

pub use payload::{ClipboardAsset, ClipboardPayload};
pub use slice::{PayloadSource, Slice, SlicePreflight};
