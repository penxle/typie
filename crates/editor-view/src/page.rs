use crate::fragment::Fragment;

#[derive(Debug, Clone)]
pub struct Page {
    pub fragments: Vec<Fragment>,
    pub height: f32,
}

impl Page {
    pub fn new(fragments: Vec<Fragment>, height: f32) -> Self {
        Self { fragments, height }
    }
}
