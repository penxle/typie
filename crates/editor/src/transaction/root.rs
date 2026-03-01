use crate::model::{DefaultAttrs, DocumentSettings, LayoutMode};
use crate::runtime::Effect;
use crate::transaction::Transaction;
use anyhow::Result;

impl Transaction {
    pub fn set_document_settings(&mut self, settings: DocumentSettings) -> Result<bool> {
        let current = self.doc().settings();
        let settings_changed = current.block_gap != settings.block_gap
            || current.paragraph_indent != settings.paragraph_indent
            || current.layout_mode != settings.layout_mode;

        if !settings_changed {
            return Ok(false);
        }

        let layout_changed = current.layout_mode != settings.layout_mode;
        self.doc().update_settings(|s| {
            s.block_gap = settings.block_gap;
            s.paragraph_indent = settings.paragraph_indent;
            s.layout_mode = settings.layout_mode;
        })?;
        if layout_changed {
            self.push_effect(Effect::LayoutChanged);
        }
        self.push_effect(Effect::SettingsChanged);
        self.push_effect(Effect::DocChanged);
        Ok(true)
    }

    pub fn set_layout_mode(&mut self, mode: LayoutMode) -> Result<bool> {
        let mut settings = self.doc().settings();
        if settings.layout_mode == mode {
            return Ok(false);
        }
        settings.layout_mode = mode;
        self.set_document_settings(settings)
    }

    pub fn set_block_gap(&mut self, gap: u32) -> Result<bool> {
        let mut settings = self.doc().settings();
        if settings.block_gap == gap {
            return Ok(false);
        }
        settings.block_gap = gap;
        self.set_document_settings(settings)
    }

    pub fn set_paragraph_indent(&mut self, indent: u32) -> Result<bool> {
        let mut settings = self.doc().settings();
        if settings.paragraph_indent == indent {
            return Ok(false);
        }
        settings.paragraph_indent = indent;
        self.set_document_settings(settings)
    }

    pub fn set_default_attrs(&mut self, attrs: DefaultAttrs) -> Result<bool> {
        let family = attrs.font_family().to_string();
        let weight = attrs.font_weight();
        self.doc().update_default_attrs(attrs)?;
        self.push_effect(Effect::FontDetected {
            family,
            weight,
            codepoints: vec!['\u{200B}' as u32],
        });
        self.push_effect(Effect::SettingsChanged);
        self.push_effect(Effect::DocChanged);
        Ok(true)
    }
}
