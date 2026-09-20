use std::ops::Range;

use icu_properties::CodePointMapDataBorrowed;
use icu_properties::props::Script;
use parley::style::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShapingLanguage {
    Korean,
    Japanese,
}

impl ShapingLanguage {
    fn as_language(self) -> Language {
        Language::parse(match self {
            Self::Korean => "ko",
            Self::Japanese => "ja",
        })
        .expect("valid shaping language tag")
    }
}

pub(super) struct ShapingLocaleRun {
    pub byte_range: Range<usize>,
    pub locale: Option<Language>,
}

#[derive(Clone, Copy)]
enum CodepointClass {
    Strong(ShapingLanguage),
    Contextual,
    Barrier,
}

pub(super) fn resolve_shaping_locale_runs(
    text: &str,
    scripts: CodePointMapDataBorrowed<'_, Script>,
) -> Vec<ShapingLocaleRun> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut runs = Vec::new();
    let mut previous_strong = None;
    let mut pending_contextual = None;
    let mut codepoints = text.char_indices().peekable();

    while let Some((byte_start, ch)) = codepoints.next() {
        let byte_end = codepoints
            .peek()
            .map_or(text.len(), |(next_start, _)| *next_start);
        match classify_codepoint(ch, scripts) {
            CodepointClass::Strong(locale) => {
                if let Some(start) = pending_contextual.take() {
                    push_contextual_runs(
                        text,
                        &mut runs,
                        start..byte_start,
                        previous_strong,
                        Some(locale),
                    );
                }
                push_locale_run(&mut runs, byte_start..byte_end, Some(locale));
                previous_strong = Some(locale);
            }
            CodepointClass::Contextual => {
                pending_contextual.get_or_insert(byte_start);
            }
            CodepointClass::Barrier => {
                if let Some(start) = pending_contextual.take() {
                    push_contextual_runs(text, &mut runs, start..byte_start, previous_strong, None);
                }
                push_locale_run(&mut runs, byte_start..byte_end, None);
                previous_strong = None;
            }
        }
    }

    if let Some(start) = pending_contextual {
        push_contextual_runs(text, &mut runs, start..text.len(), previous_strong, None);
    }
    runs
}

pub(super) fn locale_at_byte(
    runs: &[ShapingLocaleRun],
    cursor: &mut usize,
    byte_start: usize,
) -> Option<Language> {
    while runs
        .get(*cursor)
        .is_some_and(|run| run.byte_range.end <= byte_start)
    {
        *cursor += 1;
    }
    runs.get(*cursor)
        .filter(|run| run.byte_range.start <= byte_start)
        .and_then(|run| run.locale)
}

fn classify_codepoint(ch: char, scripts: CodePointMapDataBorrowed<'_, Script>) -> CodepointClass {
    match scripts.get(ch) {
        Script::Hangul => CodepointClass::Strong(ShapingLanguage::Korean),
        Script::Hiragana | Script::Katakana | Script::KatakanaOrHiragana => {
            CodepointClass::Strong(ShapingLanguage::Japanese)
        }
        Script::Common | Script::Inherited | Script::Han => CodepointClass::Contextual,
        _ => CodepointClass::Barrier,
    }
}

fn push_contextual_runs(
    text: &str,
    runs: &mut Vec<ShapingLocaleRun>,
    range: Range<usize>,
    left: Option<ShapingLanguage>,
    right: Option<ShapingLanguage>,
) {
    match (left, right) {
        (Some(left), Some(right)) if left != right => {
            let contextual = &text[range.clone()];
            let left_count = contextual.chars().count().div_ceil(2);
            let split = contextual
                .char_indices()
                .nth(left_count)
                .map_or(range.end, |(offset, _)| range.start + offset);
            push_locale_run(runs, range.start..split, Some(left));
            push_locale_run(runs, split..range.end, Some(right));
        }
        (Some(locale), _) | (None, Some(locale)) => {
            push_locale_run(runs, range, Some(locale));
        }
        (None, None) => push_locale_run(runs, range, None),
    }
}

fn push_locale_run(
    runs: &mut Vec<ShapingLocaleRun>,
    byte_range: Range<usize>,
    language: Option<ShapingLanguage>,
) {
    let locale = language.map(ShapingLanguage::as_language);
    if byte_range.is_empty() {
        return;
    }
    if let Some(last) = runs.last_mut()
        && last.locale == locale
        && last.byte_range.end == byte_range.start
    {
        last.byte_range.end = byte_range.end;
    } else {
        runs.push(ShapingLocaleRun { byte_range, locale });
    }
}

#[cfg(test)]
mod tests {
    use icu_properties::CodePointMapData;

    use super::*;

    fn resolved(text: &str) -> Vec<(Range<usize>, Option<String>)> {
        let scripts = CodePointMapData::<Script>::new();
        resolve_shaping_locale_runs(text, scripts)
            .into_iter()
            .map(|run| {
                (
                    run.byte_range,
                    run.locale.map(|locale| locale.as_str().to_owned()),
                )
            })
            .collect()
    }

    #[test]
    fn common_characters_inherit_korean_context() {
        assert_eq!(resolved("한…글"), vec![(0..9, Some("ko".to_owned()))]);
    }

    #[test]
    fn leading_common_characters_inherit_japanese_context() {
        assert_eq!(resolved("…かな"), vec![(0..9, Some("ja".to_owned()))]);
    }

    #[test]
    fn shared_cjk_punctuation_uses_surrounding_japanese_context() {
        assert_eq!(resolved("あ、い"), vec![(0..9, Some("ja".to_owned()))]);
    }

    #[test]
    fn bopomofo_remains_unspecified_without_traditional_chinese_shaping_support() {
        assert_eq!(resolved("ㄅ…ㄆ"), vec![(0..9, None)]);
    }

    #[test]
    fn han_only_text_remains_unspecified() {
        assert_eq!(resolved("漢字…"), vec![(0..9, None)]);
    }

    #[test]
    fn common_characters_use_the_nearest_cjk_context() {
        assert_eq!(
            resolved("한…あ"),
            vec![(0..6, Some("ko".to_owned())), (6..9, Some("ja".to_owned())),]
        );
    }

    #[test]
    fn other_scripts_separate_cjk_contexts() {
        assert_eq!(
            resolved("한…A…あ"),
            vec![
                (0..6, Some("ko".to_owned())),
                (6..7, None),
                (7..13, Some("ja".to_owned())),
            ]
        );
    }
}
