use editor_common::StrExt;
use editor_resource::TextSegmenters;
use editor_state::{Position, Selection};
use icu_segmenter::{SentenceSegmenter, WordSegmenter};

use crate::cursor::navigation::{first_position_in, last_position_in};
use crate::cursor::search;
use crate::fragment::LineFragment;
use crate::page::Page;

pub fn move_word_forward(
    pages: &[Page],
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = next_word_boundary(line, char_idx, &segmenters.word) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let y = line.rect.bottom();
    let (_, next) = search::find_navigable_below(pages, page_idx, y, 0.0)?;
    Some(Selection::collapsed(first_position_in(next)))
}

pub fn move_word_backward(
    pages: &[Page],
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = prev_word_boundary(line, char_idx, &segmenters.word) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let y = line.rect.y;
    let (_, prev) = search::find_navigable_above(pages, page_idx, y, 0.0)?;
    Some(Selection::collapsed(last_position_in(prev)))
}

pub fn move_sentence_forward(
    pages: &[Page],
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = next_sentence_boundary(line, char_idx, &segmenters.sentence) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let y = line.rect.bottom();
    let (_, next) = search::find_navigable_below(pages, page_idx, y, 0.0)?;
    Some(Selection::collapsed(first_position_in(next)))
}

pub fn move_sentence_backward(
    pages: &[Page],
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = prev_sentence_boundary(line, char_idx, &segmenters.sentence) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let y = line.rect.y;
    let (_, prev) = search::find_navigable_above(pages, page_idx, y, 0.0)?;
    Some(Selection::collapsed(last_position_in(prev)))
}

pub fn line_char_index(line: &LineFragment, pos: &Position) -> Option<usize> {
    let mut char_count = 0;
    for run in &line.glyph_runs {
        let run_chars = run.text.char_count();
        if run.node_id == pos.node_id {
            let local = pos.offset.checked_sub(run.offset)?;
            if local <= run_chars {
                return Some(char_count + local);
            }
        }
        char_count += run_chars;
    }
    None
}

pub fn position_at_char_index(line: &LineFragment, char_index: usize) -> Option<Position> {
    let mut remaining = char_index;
    for run in &line.glyph_runs {
        let run_chars = run.text.char_count();
        if remaining <= run_chars {
            return Some(Position::new(run.node_id, run.offset + remaining));
        }
        remaining -= run_chars;
    }
    None
}

fn line_text(line: &LineFragment) -> String {
    line.glyph_runs.iter().map(|r| r.text.as_str()).collect()
}

fn next_word_boundary(
    line: &LineFragment,
    char_index: usize,
    segmenter: &WordSegmenter,
) -> Option<usize> {
    let text = line_text(line);
    let byte_idx = text.nth_char_byte_offset(char_index);
    segmenter
        .as_borrowed()
        .segment_str(&text)
        .find(|&b| b > byte_idx)
        .map(|b| text.nth_byte_char_offset(b))
}

fn prev_word_boundary(
    line: &LineFragment,
    char_index: usize,
    segmenter: &WordSegmenter,
) -> Option<usize> {
    let text = line_text(line);
    let byte_idx = text.nth_char_byte_offset(char_index);
    segmenter
        .as_borrowed()
        .segment_str(&text)
        .filter(|&b| b < byte_idx)
        .last()
        .map(|b| text.nth_byte_char_offset(b))
}

fn next_sentence_boundary(
    line: &LineFragment,
    char_index: usize,
    segmenter: &SentenceSegmenter,
) -> Option<usize> {
    let text = line_text(line);
    let byte_idx = text.nth_char_byte_offset(char_index);
    segmenter
        .as_borrowed()
        .segment_str(&text)
        .find(|&b| b > byte_idx)
        .map(|b| text.nth_byte_char_offset(b))
}

fn prev_sentence_boundary(
    line: &LineFragment,
    char_index: usize,
    segmenter: &SentenceSegmenter,
) -> Option<usize> {
    let text = line_text(line);
    let byte_idx = text.nth_char_byte_offset(char_index);
    segmenter
        .as_borrowed()
        .segment_str(&text)
        .filter(|&b| b < byte_idx)
        .last()
        .map(|b| text.nth_byte_char_offset(b))
}

#[cfg(test)]
mod tests {
    use editor_common::Rect;
    use editor_model::NodeId;

    use super::*;
    use crate::fragment::GlyphRun;

    fn make_run(
        node_id: NodeId,
        offset: usize,
        text: &str,
        x: f32,
        advances: Vec<f32>,
    ) -> GlyphRun {
        GlyphRun::make_test_run(node_id, offset, text, x, advances)
    }

    fn make_line(id: NodeId, text: &str) -> LineFragment {
        let n = text.chars().count();
        let advances = vec![10.0; n];
        LineFragment {
            node_id: id,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 20.0,
            },
            baseline: 16.0,
            glyph_runs: vec![make_run(id, 0, text, 0.0, advances)],
        }
    }

    fn make_multi_segment_line() -> (LineFragment, NodeId, NodeId) {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let line = LineFragment {
            node_id: id1,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 20.0,
            },
            baseline: 16.0,
            glyph_runs: vec![
                make_run(id1, 0, "hello ", 0.0, vec![10.0; 6]),
                make_run(id2, 0, "world", 60.0, vec![10.0; 5]),
            ],
        };
        (line, id1, id2)
    }

    #[test]
    fn char_index_at_start() {
        let id = NodeId::new();
        let line = make_line(id, "hello");
        assert_eq!(line_char_index(&line, &Position::new(id, 0)), Some(0));
    }

    #[test]
    fn char_index_in_second_segment() {
        let (line, _, id2) = make_multi_segment_line();
        assert_eq!(line_char_index(&line, &Position::new(id2, 2)), Some(8));
    }

    #[test]
    fn position_at_start() {
        let id = NodeId::new();
        let line = make_line(id, "hello");
        let pos = position_at_char_index(&line, 0).unwrap();
        assert_eq!(pos.node_id, id);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn position_in_second_segment() {
        let (line, _, id2) = make_multi_segment_line();
        let pos = position_at_char_index(&line, 8).unwrap();
        assert_eq!(pos.node_id, id2);
        assert_eq!(pos.offset, 2);
    }

    #[test]
    fn word_forward() {
        let id = NodeId::new();
        let line = make_line(id, "hello world");
        let segmenters = TextSegmenters::new_test();
        let boundary = next_word_boundary(&line, 0, &segmenters.word).unwrap();
        assert!(boundary > 0 && boundary <= 6);
    }

    #[test]
    fn word_backward() {
        let id = NodeId::new();
        let line = make_line(id, "hello world");
        let segmenters = TextSegmenters::new_test();
        let boundary = prev_word_boundary(&line, 11, &segmenters.word).unwrap();
        assert!(boundary >= 5 && boundary <= 6);
    }

    #[test]
    fn sentence_forward() {
        let id = NodeId::new();
        let line = make_line(id, "Hello world. Goodbye world.");
        let segmenters = TextSegmenters::new_test();
        let boundary = next_sentence_boundary(&line, 0, &segmenters.sentence).unwrap();
        assert!(boundary > 0 && boundary <= 13);
    }

    #[test]
    fn sentence_backward() {
        let id = NodeId::new();
        let line = make_line(id, "Hello world. Goodbye world.");
        let segmenters = TextSegmenters::new_test();
        let boundary = prev_sentence_boundary(&line, 27, &segmenters.sentence).unwrap();
        assert!(boundary >= 12 && boundary <= 13);
    }
}
