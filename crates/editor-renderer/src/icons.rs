use crate::types::Path;
use editor_common::Rect;

pub struct IconRegistry;

pub static ICONS: IconRegistry = IconRegistry;

impl IconRegistry {
    pub fn resolve(&self, _name: &str, rect: Rect) -> Path {
        Path::rect(rect)
    }
}
