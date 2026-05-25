use icu_properties::CodePointMapData;
use icu_properties::props::GeneralCategory;

pub struct CharacterCount {
    pub with_whitespace: u32,
    pub without_whitespace: u32,
    pub without_whitespace_and_punctuation: u32,
}

pub fn count_text(
    text: &str,
    general_category: &CodePointMapData<GeneralCategory>,
) -> CharacterCount {
    let gc_map = general_category.as_borrowed();

    let mut with_ws: u32 = 0;
    let mut without_ws: u32 = 0;
    let mut without_ws_punct: u32 = 0;
    let mut prev_whitespace = false;

    for c in text.chars() {
        if c == '\u{200B}' {
            continue;
        }

        if c.is_whitespace() {
            if !prev_whitespace {
                with_ws += 1;
            }
            prev_whitespace = true;
        } else {
            with_ws += 1;
            without_ws += 1;
            prev_whitespace = false;

            let gc = gc_map.get(c);
            if !matches!(
                gc,
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
    }

    let first_non_ws = text
        .chars()
        .find(|&c| c != '\u{200B}' && !c.is_whitespace());
    if first_non_ws.is_none() {
        return CharacterCount {
            with_whitespace: 0,
            without_whitespace: without_ws,
            without_whitespace_and_punctuation: without_ws_punct,
        };
    }

    let starts_with_ws = text
        .chars()
        .find(|&c| c != '\u{200B}')
        .is_some_and(|c| c.is_whitespace());
    let ends_with_ws = text
        .chars()
        .rev()
        .find(|&c| c != '\u{200B}')
        .is_some_and(|c| c.is_whitespace());

    if starts_with_ws && with_ws > 0 {
        with_ws = with_ws.saturating_sub(1);
    }
    if ends_with_ws && with_ws > 0 {
        with_ws = with_ws.saturating_sub(1);
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

    use super::*;

    fn gc() -> CodePointMapData<GeneralCategory> {
        CodePointMapData::<GeneralCategory>::new().static_to_owned()
    }

    fn count(text: &str) -> (u32, u32, u32) {
        let c = count_text(text, &gc());
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
    fn whitespace_only_is_all_zero() {
        assert_eq!(count("   "), (0, 0, 0));
    }

    #[test]
    fn single_word_no_whitespace() {
        assert_eq!(count("hello"), (5, 5, 5));
    }

    #[test]
    fn single_space_between_words() {
        assert_eq!(count("a b"), (3, 2, 2));
    }

    #[test]
    fn consecutive_whitespace_counts_as_one() {
        assert_eq!(count("a  b"), (3, 2, 2));
    }

    #[test]
    fn leading_and_trailing_whitespace_trimmed() {
        assert_eq!(count(" abc "), (3, 3, 3));
    }

    #[test]
    fn leading_only_whitespace_trimmed() {
        assert_eq!(count(" abc"), (3, 3, 3));
    }

    #[test]
    fn trailing_only_whitespace_trimmed() {
        assert_eq!(count("abc "), (3, 3, 3));
    }

    #[test]
    fn zero_width_space_ignored() {
        assert_eq!(count("a\u{200B}b"), (2, 2, 2));
    }

    #[test]
    fn only_zero_width_space_is_all_zero() {
        assert_eq!(count("\u{200B}"), (0, 0, 0));
    }

    #[test]
    fn hangul_word() {
        assert_eq!(count("안녕하세요"), (5, 5, 5));
    }

    #[test]
    fn ascii_punctuation_excluded_from_punct_count() {
        assert_eq!(count("hello, world!"), (13, 12, 10));
    }

    #[test]
    fn newline_treated_as_whitespace() {
        assert_eq!(count("a\nb"), (3, 2, 2));
    }
}
