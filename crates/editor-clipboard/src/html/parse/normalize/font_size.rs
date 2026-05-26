use editor_model::Modifier;

const MIN_FONT_SIZE: u32 = 100;
const MAX_FONT_SIZE: u32 = 20_000;

pub fn normalize(value: u32) -> Modifier {
    Modifier::FontSize {
        value: value.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(v: u32) -> u32 {
        match normalize(v) {
            Modifier::FontSize { value } => value,
            _ => panic!("expected FontSize"),
        }
    }

    #[test]
    fn clamp_below_min() {
        assert_eq!(size(0), 100);
        assert_eq!(size(50), 100);
        assert_eq!(size(99), 100);
    }

    #[test]
    fn clamp_above_max() {
        assert_eq!(size(20_001), 20_000);
        assert_eq!(size(50_000), 20_000);
    }

    #[test]
    fn preserves_arbitrary_value_in_range() {
        assert_eq!(size(1234), 1234);
        assert_eq!(size(1600), 1600);
        assert_eq!(size(100), 100);
        assert_eq!(size(20_000), 20_000);
    }
}
