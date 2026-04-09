use crate::icon_data;
use crate::types::IconData;

pub struct IconRegistry;

pub static ICONS: IconRegistry = IconRegistry;

impl IconRegistry {
    pub fn resolve(&self, name: &str) -> Option<&'static IconData> {
        icon_data::ICONS.get(name)
    }
}
