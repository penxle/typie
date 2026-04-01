use crate::error::ResourceError;
use editor_common::TextSegmenters;
use fontique::ScriptExt;
use parley::{FontContext, LayoutContext};
use std::sync::Arc;

use crate::brush::TextBrush;
use crate::font::FontRegistry;

pub struct Resource {
    pub font_registry: FontRegistry,
    pub font_context: FontContext,
    pub layout_context: LayoutContext<TextBrush>,
    pub segmenters: Option<TextSegmenters>,
}

impl Resource {
    pub fn new() -> Self {
        Self {
            font_registry: FontRegistry::new(),
            font_context: FontContext::new(),
            layout_context: LayoutContext::new(),
            segmenters: None,
        }
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
        data: &[u8],
    ) -> Result<(), ResourceError> {
        let id = self.font_registry.intern(family);
        self.font_registry.add_font_chunk(id, weight, data)?;
        Ok(())
    }

    pub fn set_fallback_font_families(
        &mut self,
        families: Vec<String>,
    ) -> Result<(), ResourceError> {
        let families: Vec<fontique::FamilyId> = families
            .iter()
            .filter_map(|name| {
                self.font_context
                    .collection
                    .family_by_name(name)
                    .map(|f| f.id())
            })
            .collect();

        for script in fontique::Script::all_samples()
            .iter()
            .map(|(script, _)| script)
            .chain(&[
                fontique::Script::COMMON,
                fontique::Script::INHERITED,
                fontique::Script::UNKNOWN,
            ])
        {
            self.font_context.collection.set_fallbacks(
                fontique::FallbackKey::new(*script, None),
                families.iter().copied(),
            );
        }

        Ok(())
    }
}

impl Default for Resource {
    fn default() -> Self {
        Self::new()
    }
}
