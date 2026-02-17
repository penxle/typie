mod decode;
pub mod encode;
mod svg;
mod utils;

pub(crate) use svg::outline_text_to_svg;
pub use utils::FontMetadata;
pub(crate) use utils::get_font_metadata;

pub(crate) const TPFT_MAGIC: &[u8; 4] = b"TPFT";
pub(crate) const TPFT_VERSION: u16 = 1;
pub(crate) const TPFT_HEADER_SIZE: usize = 6;

pub(crate) use decode::decode_tpft;
