use super::Size;

#[derive(Debug, Clone, Copy)]
pub struct BoxConstraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl BoxConstraints {
    pub fn new(min_width: f32, max_width: f32, min_height: f32, max_height: f32) -> Self {
        debug_assert!(min_width <= max_width, "min_width must be <= max_width");
        debug_assert!(min_height <= max_height, "min_height must be <= max_height");
        debug_assert!(min_width >= 0.0, "min_width must be >= 0");
        debug_assert!(min_height >= 0.0, "min_height must be >= 0");

        Self {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }

    pub fn loose(size: Size) -> Self {
        Self {
            min_width: 0.0,
            max_width: size.width,
            min_height: 0.0,
            max_height: size.height,
        }
    }
}

impl std::hash::Hash for BoxConstraints {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.min_width.to_bits().hash(state);
        self.max_width.to_bits().hash(state);
        self.min_height.to_bits().hash(state);
        self.max_height.to_bits().hash(state);
    }
}
