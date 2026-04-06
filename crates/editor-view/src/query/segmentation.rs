use editor_common::StrExt;
use editor_resource::TextSegmenters;
use editor_state::{Position, Selection};
use icu_segmenter::{SentenceSegmenter, WordSegmenter};

use crate::paginate::*;

use super::navigation::{first_position_in, last_position_in};
use super::search;

pub fn move_word_forward(
    tree: &LayoutTree,
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;
    let line = match &line_node.content {
        LayoutContent::Line(l) => l,
        _ => return None,
    };
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = next_word_boundary(line, char_idx, &segmenters.word) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let y = line_node.rect.bottom();
    let next = search::find_navigable_below(&tree.root, y)?;
    Some(Selection::collapsed(first_position_in(next)))
}

pub fn move_word_backward(
    tree: &LayoutTree,
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;
    let line = match &line_node.content {
        LayoutContent::Line(l) => l,
        _ => return None,
    };
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = prev_word_boundary(line, char_idx, &segmenters.word) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let y = line_node.rect.y;
    let prev = search::find_navigable_above(&tree.root, y)?;
    Some(Selection::collapsed(last_position_in(prev)))
}

pub fn move_sentence_forward(
    tree: &LayoutTree,
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;
    let line = match &line_node.content {
        LayoutContent::Line(l) => l,
        _ => return None,
    };
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = next_sentence_boundary(line, char_idx, &segmenters.sentence) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let y = line_node.rect.bottom();
    let next = search::find_navigable_below(&tree.root, y)?;
    Some(Selection::collapsed(first_position_in(next)))
}

pub fn move_sentence_backward(
    tree: &LayoutTree,
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;
    let line = match &line_node.content {
        LayoutContent::Line(l) => l,
        _ => return None,
    };
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = prev_sentence_boundary(line, char_idx, &segmenters.sentence) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let y = line_node.rect.y;
    let prev = search::find_navigable_above(&tree.root, y)?;
    Some(Selection::collapsed(last_position_in(prev)))
}

pub fn line_char_index(line: &LayoutLine, pos: &Position) -> Option<usize> {
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

pub fn position_at_char_index(line: &LayoutLine, char_index: usize) -> Option<Position> {
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

fn line_text(line: &LayoutLine) -> String {
    line.glyph_runs.iter().map(|r| r.text.as_str()).collect()
}

fn next_word_boundary(
    line: &LayoutLine,
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
    line: &LayoutLine,
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
    line: &LayoutLine,
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
    line: &LayoutLine,
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
    use editor_model::NodeId;

    use super::*;
    use crate::glyph_run::{GlyphRun, GraphemeSpan};

    fn gs(n: usize) -> Vec<GraphemeSpan> {
        vec![
            GraphemeSpan {
                advance: 10.0,
                codepoints: 1
            };
            n
        ]
    }

    fn make_line(id: NodeId, text: &str) -> LayoutLine {
        let n = text.chars().count();
        LayoutLine {
            node_id: id,
            baseline: 16.0,
            glyph_runs: vec![GlyphRun::make_test_run(id, 0, text, 0.0, gs(n))],
        }
    }

    fn make_multi_segment_line() -> (LayoutLine, NodeId, NodeId) {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let line = LayoutLine {
            node_id: id1,
            baseline: 16.0,
            glyph_runs: vec![
                GlyphRun::make_test_run(id1, 0, "hello ", 0.0, gs(6)),
                GlyphRun::make_test_run(id2, 0, "world", 60.0, gs(5)),
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
        assert!((5..=6).contains(&boundary));
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
        assert!((12..=13).contains(&boundary));
    }
}
