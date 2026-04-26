use proptest::prelude::*;

pub fn arb_unicode_text(min: usize, max: usize) -> impl Strategy<Value = String> {
    let char_strategy = prop_oneof![
        50 => proptest::char::range('\u{0021}', '\u{D7FF}'),
        30 => proptest::char::range('\u{E000}', '\u{FFFD}'),
        20 => proptest::char::range('\u{1F300}', '\u{1F9FF}'),
    ];
    proptest::collection::vec(char_strategy, min..=max)
        .prop_map(|chars| chars.into_iter().collect())
}

#[cfg(test)]
mod sanity {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn respects_max_char_count(s in arb_unicode_text(0, 12)) {
            prop_assert!(s.chars().count() <= 12, "char count {} > 12", s.chars().count());
        }
    }
}
