use editor_model::Modifier;

const PALETTE: &[i32] = &[-10, -5, 0, 5, 10, 20, 40];

pub fn normalize(value: i32) -> Modifier {
    let snapped = PALETTE
        .iter()
        .copied()
        .min_by(|a, b| {
            let da = (a - value).abs();
            let db = (b - value).abs();
            da.cmp(&db).then_with(|| a.abs().cmp(&b.abs()))
        })
        .expect("palette is non-empty");
    Modifier::LetterSpacing { value: snapped }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ls(v: i32) -> i32 {
        match normalize(v) {
            Modifier::LetterSpacing { value } => value,
            _ => panic!("expected LetterSpacing"),
        }
    }

    #[test]
    fn exact_palette_preserved() {
        for v in [-10, -5, 0, 5, 10, 20, 40] {
            assert_eq!(ls(v), v);
        }
    }

    #[test]
    fn snap_to_nearest() {
        assert_eq!(ls(3), 5);
        assert_eq!(ls(7), 5);
        assert_eq!(ls(8), 10);
        assert_eq!(ls(15), 10);
        assert_eq!(ls(16), 20);
        assert_eq!(ls(30), 20);
        assert_eq!(ls(31), 40);
        assert_eq!(ls(-3), -5);
        assert_eq!(ls(-8), -10);
    }

    #[test]
    fn clamp_out_of_range() {
        assert_eq!(ls(100), 40);
        assert_eq!(ls(-100), -10);
    }

    #[test]
    fn tie_prefers_smaller_abs() {
        assert_eq!(ls(15), 10);
        assert_eq!(ls(30), 20);
    }
}
