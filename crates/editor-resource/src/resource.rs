use parley::{FontContext, LayoutContext};
use std::sync::Arc;

use icu_properties::CodePointMapData;
use icu_properties::props::GeneralCategory;

use crate::brush::TextBrush;
use crate::error::ResourceError;
use crate::font::{FontFamily, FontRegistry, PLACEHOLDER_FAMILY_NAME, PLACEHOLDER_WEIGHT};
use crate::segmentation::{IcuResources, TextSegmenters};
use crate::text_replacement::{RawTextReplacementRule, TextReplacementRule, compile_rules};
use crate::theme::Theme;
use crate::theme_data::ThemeVariant;

const PLACEHOLDER_TTF: &[u8] = include_bytes!("../assets/placeholder.ttf");

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

    pub fn add_font_base(
        &mut self,
        family: &str,
        weight: u16,
        data: &[u8],
    ) -> Result<(), ResourceError> {
        let id = self.font_registry.intern(family);
        self.font_registry.add_font_base(id, weight, data)?;

        if let Some(font_bytes) = self.font_registry.font_data(id, weight) {
            self.font_context.collection.register_fonts(
                fontique::Blob::new(Arc::new(font_bytes.to_vec())),
                Some(fontique::FontInfoOverride {
                    family_name: Some(family),
                    weight: Some(fontique::FontWeight::new(weight as f32)),
                    ..Default::default()
                }),
            );
        }

        Ok(())
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
}
