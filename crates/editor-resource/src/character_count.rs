use icu_properties::CodePointMapData;
use icu_properties::props::GeneralCategory;
use icu_segmenter::GraphemeClusterSegmenter;

pub struct CharacterCount {
    pub with_whitespace: u32,
    pub without_whitespace: u32,
    pub without_whitespace_and_punctuation: u32,
}

pub fn count_text(
    text: &str,
    grapheme: &GraphemeClusterSegmenter,
    general_category: &CodePointMapData<GeneralCategory>,
) -> CharacterCount {
    let stripped;
    let text = if text.contains('\u{200B}') {
        stripped = text.replace('\u{200B}', "");
        stripped.as_str()
    } else {
        text
    };

    if text.is_empty() {
        return CharacterCount {
            with_whitespace: 0,
            without_whitespace: 0,
            without_whitespace_and_punctuation: 0,
        };
    }

    let mut with_ws: u32 = 0;
    let mut without_ws: u32 = 0;
    let mut without_ws_punct: u32 = 0;

    let gc_map = general_category.as_borrowed();
    let mut prev = 0usize;
    for end in grapheme.as_borrowed().segment_str(text).skip(1) {
        let base = text[prev..end].chars().next().unwrap();
        prev = end;
        with_ws += 1;
        if base.is_whitespace() {
            continue;
        }
        without_ws += 1;
        if !matches!(
            gc_map.get(base),
            GeneralCategory::ConnectorPunctuation
                | GeneralCategory::DashPunctuation
                | GeneralCategory::ClosePunctuation
                | GeneralCategory::FinalPunctuation
                | GeneralCategory::InitialPunctuation
                | GeneralCategory::OtherPunctuation
                | GeneralCategory::OpenPunctuation
        ) {
            without_ws_punct += 1;
        }
    }

    CharacterCount {
        with_whitespace: with_ws,
        without_whitespace: without_ws,
        without_whitespace_and_punctuation: without_ws_punct,
    }
}

#[cfg(test)]
mod tests {
    use icu_properties::CodePointMapData;
    use icu_properties::props::GeneralCategory;
    use icu_segmenter::GraphemeClusterSegmenter;

    use super::*;

    fn count(text: &str) -> (u32, u32, u32) {
        let seg = GraphemeClusterSegmenter::new().static_to_owned();
        let gc = CodePointMapData::<GeneralCategory>::new().static_to_owned();
        let c = count_text(text, &seg, &gc);
        (
            c.with_whitespace,
            c.without_whitespace,
            c.without_whitespace_and_punctuation,
        )
    }

    #[test]
    fn empty_string_is_all_zero() {
        assert_eq!(count(""), (0, 0, 0));
    }

    #[test]
    fn whitespace_is_counted_literally() {
        assert_eq!(count("   "), (3, 0, 0));
        assert_eq!(count("a b"), (3, 2, 2));
        assert_eq!(count("a  b"), (4, 2, 2));
        assert_eq!(count(" abc "), (5, 3, 3));
        assert_eq!(count("a\tb"), (3, 2, 2));
        assert_eq!(count("a\nb"), (3, 2, 2));
        assert_eq!(count("a\n\nb"), (4, 2, 2));
    }

    #[test]
    fn non_ascii_and_multi_scalar_whitespace_is_counted_literally() {
        assert_eq!(count("a\u{00A0}b"), (3, 2, 2));
        assert_eq!(count("a\u{3000}b"), (3, 2, 2));
        assert_eq!(count("a\r\nb"), (3, 2, 2));
        assert_eq!(count("\r\n"), (1, 0, 0));
    }

    #[test]
    fn single_word_no_whitespace() {
        assert_eq!(count("hello"), (5, 5, 5));
        assert_eq!(count("안녕하세요"), (5, 5, 5));
    }

    #[test]
    fn punctuation_excluded_from_third_tier_only() {
        assert_eq!(count("hello, world!"), (13, 12, 10));
    }

    #[test]
    fn zero_width_space_stripped() {
        assert_eq!(count("a\u{200B}b"), (2, 2, 2));
        assert_eq!(count("\u{200B}"), (0, 0, 0));
    }

    #[test]
    fn nfd_hangul_syllable_is_one_grapheme() {
        assert_eq!(count("\u{1112}\u{1161}\u{11AB}"), (1, 1, 1));
    }

    #[test]
    fn emoji_clusters_count_as_one() {
        assert_eq!(count("👨\u{200D}👩\u{200D}👧\u{200D}👦"), (1, 1, 1));
        assert_eq!(count("👍🏽"), (1, 1, 1));
        assert_eq!(count("🇰🇷"), (1, 1, 1));
        assert_eq!(count("1\u{FE0F}\u{20E3}"), (1, 1, 1));
    }

    #[test]
    fn punctuation_base_emoji_excluded_from_third_tier() {
        assert_eq!(count("‼\u{FE0F}"), (1, 1, 0));
    }
}
