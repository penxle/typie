mod decode;
pub mod encode;
mod svg;
mod utils;

pub use svg::outline_text_to_svg;
pub use utils::FontMetadata;
pub use utils::get_font_metadata;

pub const TPFT_MAGIC: &[u8; 4] = b"TPFT";
pub const TPFT_VERSION: u16 = 1;
pub const TPFT_HEADER_SIZE: usize = 6;

pub use decode::decode_tpft;
