use bitcode::{Decode, Encode};

use super::manifest::FontManifest;

#[derive(Clone, Debug, Encode, Decode)]
pub struct FallbackFont {
    pub weight: u16,
    pub manifest: FontManifest,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct FallbackFontEntry {
    pub family_name: String,
    pub fonts: Vec<FallbackFont>,
}
