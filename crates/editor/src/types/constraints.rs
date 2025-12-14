use super::Size;

#[derive(Debug, Clone, Copy)]
pub struct BoxConstraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

#[allow(dead_code)]
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

    pub fn tight(size: Size) -> Self {
        Self {
            min_width: size.width,
            max_width: size.width,
            min_height: size.height,
            max_height: size.height,
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

    pub fn tighten(&self, width: Option<f32>, height: Option<f32>) -> Self {
        Self {
            min_width: width.unwrap_or(self.min_width),
            max_width: width.unwrap_or(self.max_width),
            min_height: height.unwrap_or(self.min_height),
            max_height: height.unwrap_or(self.max_height),
        }
    }

    pub fn loosen(&self) -> Self {
        Self {
            min_width: 0.0,
            max_width: self.max_width,
            min_height: 0.0,
            max_height: self.max_height,
        }
    }

    pub fn constrain(&self, size: Size) -> Size {
        Size::new(
            size.width.clamp(self.min_width, self.max_width),
            size.height.clamp(self.min_height, self.max_height),
        )
    }

    pub fn constrain_width(&self, width: f32) -> f32 {
        width.clamp(self.min_width, self.max_width)
    }

    pub fn constrain_height(&self, height: f32) -> f32 {
        height.clamp(self.min_height, self.max_height)
    }

    pub fn is_tight(&self) -> bool {
        self.min_width == self.max_width && self.min_height == self.max_height
    }

    pub fn biggest(&self) -> Size {
        Size::new(self.max_width, self.max_height)
    }

    pub fn smallest(&self) -> Size {
        Size::new(self.min_width, self.min_height)
    }

    pub fn deflate(&self, edge_insets: super::EdgeInsets) -> Self {
        let horizontal = edge_insets.left + edge_insets.right;
        let vertical = edge_insets.top + edge_insets.bottom;

        Self {
            min_width: (self.min_width - horizontal).max(0.0),
            max_width: (self.max_width - horizontal).max(0.0),
            min_height: (self.min_height - vertical).max(0.0),
            max_height: (self.max_height - vertical).max(0.0),
        }
    }

    pub fn has_tight_width(&self) -> bool {
        self.min_width == self.max_width
    }

    pub fn has_tight_height(&self) -> bool {
        self.min_height == self.max_height
    }

    pub fn has_bounded_width(&self) -> bool {
        self.max_width < f32::INFINITY
    }

    pub fn has_bounded_height(&self) -> bool {
        self.max_height < f32::INFINITY
    }

    pub fn has_infinite_width(&self) -> bool {
        self.max_width == f32::INFINITY
    }

    pub fn has_infinite_height(&self) -> bool {
        self.max_height == f32::INFINITY
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_constraints_tight() {
        let c = BoxConstraints::tight(Size::new(100.0, 200.0));
        assert_eq!(c.min_width, 100.0);
        assert_eq!(c.max_width, 100.0);
        assert_eq!(c.min_height, 200.0);
        assert_eq!(c.max_height, 200.0);
        assert!(c.is_tight());
    }

    #[test]
    fn test_box_constraints_loose() {
        let c = BoxConstraints::loose(Size::new(100.0, 200.0));
        assert_eq!(c.min_width, 0.0);
        assert_eq!(c.max_width, 100.0);
        assert_eq!(c.min_height, 0.0);
        assert_eq!(c.max_height, 200.0);
        assert!(!c.is_tight());
    }

    #[test]
    fn test_box_constraints_constrain() {
        let c = BoxConstraints::new(50.0, 150.0, 50.0, 150.0);

        let s1 = c.constrain(Size::new(200.0, 30.0));
        assert_eq!(s1.width, 150.0);
        assert_eq!(s1.height, 50.0);

        let s2 = c.constrain(Size::new(30.0, 200.0));
        assert_eq!(s2.width, 50.0);
        assert_eq!(s2.height, 150.0);

        let s3 = c.constrain(Size::new(100.0, 100.0));
        assert_eq!(s3.width, 100.0);
        assert_eq!(s3.height, 100.0);
    }

    #[test]
    fn test_box_constraints_loosen() {
        let c = BoxConstraints::new(50.0, 150.0, 50.0, 150.0);
        let loosened = c.loosen();
        assert_eq!(loosened.min_width, 0.0);
        assert_eq!(loosened.max_width, 150.0);
        assert_eq!(loosened.min_height, 0.0);
        assert_eq!(loosened.max_height, 150.0);
    }

    #[test]
    fn test_box_constraints_tighten() {
        let c = BoxConstraints::new(50.0, 150.0, 50.0, 150.0);
        let tightened = c.tighten(Some(100.0), Some(80.0));
        assert_eq!(tightened.min_width, 100.0);
        assert_eq!(tightened.max_width, 100.0);
        assert_eq!(tightened.min_height, 80.0);
        assert_eq!(tightened.max_height, 80.0);
    }

    #[test]
    fn test_box_constraints_biggest_smallest() {
        let c = BoxConstraints::new(50.0, 150.0, 50.0, 150.0);

        let biggest = c.biggest();
        assert_eq!(biggest.width, 150.0);
        assert_eq!(biggest.height, 150.0);

        let smallest = c.smallest();
        assert_eq!(smallest.width, 50.0);
        assert_eq!(smallest.height, 50.0);
    }
}
