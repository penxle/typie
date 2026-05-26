use editor_model::Modifier;

pub fn normalize(value: u16) -> Modifier {
    let clamped = value.clamp(100, 900);
    let snapped = ((clamped as u32 + 50) / 100 * 100).min(900) as u16;
    Modifier::FontWeight { value: snapped }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weight(v: u16) -> u16 {
        match normalize(v) {
            Modifier::FontWeight { value } => value,
            _ => panic!("expected FontWeight"),
        }
    }

    #[test]
    fn snap_to_nearest_hundred() {
        assert_eq!(weight(350), 400);
        assert_eq!(weight(449), 400);
        assert_eq!(weight(450), 500);
        assert_eq!(weight(550), 600);
        assert_eq!(weight(700), 700);
    }

    #[test]
    fn clamp_below_100() {
        assert_eq!(weight(0), 100);
        assert_eq!(weight(50), 100);
        assert_eq!(weight(99), 100);
    }

    #[test]
    fn clamp_above_900() {
        assert_eq!(weight(901), 900);
        assert_eq!(weight(1000), 900);
        assert_eq!(weight(2000), 900);
    }

    #[test]
    fn exact_palette_values_preserved() {
        for w in [100u16, 200, 300, 400, 500, 600, 700, 800, 900] {
            assert_eq!(weight(w), w);
        }
    }
}
