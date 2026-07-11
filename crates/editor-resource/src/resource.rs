use parley::{FontContext, LayoutContext};
use skrifa::Tag;
use skrifa::raw::FontRef;
use std::sync::Arc;

use icu_properties::CodePointMapData;
use icu_properties::props::GeneralCategory;

use crate::brush::TextBrush;
use crate::error::ResourceError;
use crate::font::{
    FontData, FontFamily, FontManifest, FontRegistry, PLACEHOLDER_FAMILY_NAME, PLACEHOLDER_WEIGHT,
};
use crate::segmentation::{IcuResources, TextSegmenters};
use crate::text_replacement::{RawTextReplacementRule, TextReplacementRule, compile_rules};
use crate::theme::Theme;
use crate::theme_data::ThemeVariant;
use crate::zstd::decompress_zstd_capped;

const PLACEHOLDER_TTF: &[u8] = include_bytes!("../assets/placeholder.ttf");
const BASE_MAX_BYTES: usize = 64 * 1024 * 1024;

/// Output of [`prepare_font_base`] — the expensive TTF parsing and blob copy
/// done up front so `Resource::insert_font_base` only needs a short,
/// infallible apply step while holding the `Resource` mutex.
pub struct PreparedFontBase {
    font_data: Arc<FontData>,
    split_offset: usize,
    blob: fontique::Blob<u8>,
}

/// Decompress and parse a base font ahead of taking the `Resource` lock.
pub fn prepare_font_base(data: &[u8]) -> Result<PreparedFontBase, ResourceError> {
    let raw_ttf = decompress_zstd_capped(data, BASE_MAX_BYTES)?;

    let font = FontRef::new(&raw_ttf)
        .map_err(|e| ResourceError::InvalidFont(format!("failed to parse TTF: {e:?}")))?;

    let glyf_tag = Tag::new(b"glyf");
    let cbdt_tag = Tag::new(b"CBDT");
    let record = font
        .table_directory()
        .table_records()
        .iter()
        .find(|r| r.tag() == cbdt_tag)
        .or_else(|| {
            font.table_directory()
                .table_records()
                .iter()
                .find(|r| r.tag() == glyf_tag)
        })
        .ok_or_else(|| ResourceError::InvalidFont("glyf/CBDT table missing".into()))?;

    let split_offset = record.offset() as usize;
    let font_data = Arc::new(FontData::new(raw_ttf));
    let blob = fontique::Blob::new(font_data.clone());

    Ok(PreparedFontBase {
        font_data,
        split_offset,
        blob,
    })
}

pub struct Resource {
    pub theme: Theme,
    pub font_registry: FontRegistry,
    pub font_context: FontContext,
    pub layout_context: LayoutContext<TextBrush>,
    pub segmenters: Arc<TextSegmenters>,
    pub general_category: Arc<CodePointMapData<GeneralCategory>>,
    pub text_replacement_rules: Vec<TextReplacementRule>,
    pub auto_surround_enabled: bool,
}

impl Resource {
    pub fn new(icu: IcuResources) -> Self {
        let mut resource = Self {
            theme: Theme::new(ThemeVariant::LightWhite),
            font_registry: FontRegistry::new(),
            font_context: FontContext::new(),
            layout_context: LayoutContext::new(),
            segmenters: icu.segmenters,
            general_category: icu.general_category,
            text_replacement_rules: Vec::new(),
            auto_surround_enabled: true,
        };
        resource.register_placeholder();
        resource
    }

    pub fn set_text_replacement_rules(&mut self, raw_rules: Vec<RawTextReplacementRule>) {
        self.text_replacement_rules = compile_rules(raw_rules);
    }

    pub fn set_auto_surround_enabled(&mut self, enabled: bool) {
        self.auto_surround_enabled = enabled;
    }

    pub fn clear_text_replacement_rules(&mut self) {
        self.text_replacement_rules.clear();
    }

    fn register_placeholder(&mut self) {
        self.font_registry.register_placeholder(PLACEHOLDER_TTF);
        self.font_context.collection.register_fonts(
            fontique::Blob::new(Arc::new(PLACEHOLDER_TTF.to_vec())),
            Some(fontique::FontInfoOverride {
                family_name: Some(PLACEHOLDER_FAMILY_NAME),
                weight: Some(fontique::FontWeight::new(PLACEHOLDER_WEIGHT as f32)),
                ..Default::default()
            }),
        );
    }

    pub fn set_fonts(&mut self, families: Vec<FontFamily>) {
        self.font_registry.set_fonts(families);
    }

    /// Apply a base font prepared by [`prepare_font_base`]: registers it in
    /// `font_registry` and with the `fontique` collection used for shaping.
    pub fn insert_font_base(
        &mut self,
        family: &str,
        weight: u16,
        prepared: PreparedFontBase,
    ) -> Result<(), ResourceError> {
        let id = self.font_registry.intern(family);
        self.font_registry
            .insert_base(id, weight, prepared.font_data, prepared.split_offset);

        self.font_context.collection.register_fonts(
            prepared.blob,
            Some(fontique::FontInfoOverride {
                family_name: Some(family),
                weight: Some(fontique::FontWeight::new(weight as f32)),
                ..Default::default()
            }),
        );

        Ok(())
    }

    pub fn add_font_base(
        &mut self,
        family: &str,
        weight: u16,
        data: &[u8],
    ) -> Result<(), ResourceError> {
        let prepared = prepare_font_base(data)?;
        self.insert_font_base(family, weight, prepared)
    }

    pub fn add_font_chunk(
        &mut self,
        family: &str,
        weight: u16,
        chunk_id: u16,
        data: &[u8],
    ) -> Result<(), ResourceError> {
        let id = self.font_registry.intern(family);
        self.font_registry
            .add_font_chunk(id, weight, chunk_id, data)?;
        Ok(())
    }

    pub fn add_font_manifest(
        &mut self,
        family: &str,
        weight: u16,
        manifest: FontManifest,
    ) -> Result<(), ResourceError> {
        let family_id = self.font_registry.intern(family);
        self.font_registry.set_manifest(family_id, weight, manifest);
        Ok(())
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Resource {
    pub fn new_test() -> Self {
        let segmenters = Arc::new(TextSegmenters::new_test());
        let general_category =
            Arc::new(CodePointMapData::<GeneralCategory>::new().static_to_owned());
        Self::new(IcuResources {
            segmenters,
            general_category,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_fonts_accepts_empty() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![]);
    }

    #[test]
    fn new_initializes_with_light_white_theme() {
        let resource = Resource::new_test();
        assert_eq!(resource.theme.variant(), ThemeVariant::LightWhite);
    }

    #[test]
    fn theme_set_variant_mutates() {
        let mut resource = Resource::new_test();
        assert!(resource.theme.set_variant(ThemeVariant::DarkBlack));
        assert_eq!(resource.theme.variant(), ThemeVariant::DarkBlack);
    }

    #[test]
    fn placeholder_registered_on_new() {
        use crate::font::{PLACEHOLDER_FAMILY_NAME, PLACEHOLDER_WEIGHT};

        let mut resource = Resource::new_test();

        let id = resource
            .font_registry
            .placeholder_family_id()
            .expect("placeholder family id must be set on Resource::new");
        assert_eq!(
            resource.font_registry.family_name_opt(id),
            Some(PLACEHOLDER_FAMILY_NAME)
        );
        let bytes = resource
            .font_registry
            .font_data(id, PLACEHOLDER_WEIGHT)
            .expect("placeholder bytes present");
        assert!(!bytes.is_empty());

        let family = resource
            .font_context
            .collection
            .family_by_name(PLACEHOLDER_FAMILY_NAME);
        assert!(
            family.is_some(),
            "placeholder must be registered with fontique"
        );
    }

    #[test]
    fn prepare_font_base_shares_single_copy_via_arc() {
        let compressed = crate::zstd::compress_zstd(PLACEHOLDER_TTF);
        let prepared = prepare_font_base(&compressed).unwrap();

        assert_eq!(
            prepared.blob.strong_count(),
            2,
            "font_data and blob must share one allocation, not hold separate copies"
        );
    }
}
