use crate::model::DefaultAttrs;
use crate::runtime::Effect;
use crate::transaction::Transaction;
use anyhow::Result;

impl Transaction {
    pub fn set_block_gap(&mut self, gap: f32) -> Result<bool> {
        let _ = self.doc().update_settings(|s| s.block_gap = gap);
        self.push_effect(Effect::SettingsChanged);
        self.push_effect(Effect::DocChanged);
        Ok(true)
    }

    pub fn set_paragraph_indent(&mut self, indent: f32) -> Result<bool> {
        let _ = self.doc().update_settings(|s| s.paragraph_indent = indent);
        self.push_effect(Effect::SettingsChanged);
        self.push_effect(Effect::DocChanged);
        Ok(true)
    }

    pub fn set_default_attrs(&mut self, attrs: DefaultAttrs) -> Result<bool> {
        let _ = self.doc().update_default_attrs(attrs);
        self.push_effect(Effect::SettingsChanged);
        self.push_effect(Effect::DocChanged);
        Ok(true)
    }
}
