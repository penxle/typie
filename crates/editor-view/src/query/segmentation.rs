use editor_common::StrExt;
use editor_resource::TextSegmenters;
use editor_state::{Affinity, Position, ResolvedPosition, Selection};
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

pub fn select_word_at(
    tree: &LayoutTree,
    pos: &ResolvedPosition<'_>,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let position = Position::from(pos);
    let line_node = search::find_line_at(tree, &position)?;
    let line = match &line_node.content {
        LayoutContent::Line(l) => l,
        _ => return None,
    };

    if line.glyph_runs.is_empty() {
        let para = pos.doc().node(line.node_id)?;
        let parent_id = para.parent()?.id();
        let index = para.index()?;
        return Some(Selection::new(
            Position {
                node_id: parent_id,
                offset: index,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: parent_id,
                offset: index + 1,
                affinity: Affinity::Upstream,
            },
        ));
    }

    let char_idx = line_char_index(line, &position)?;
    let text = line_text(line);
    let byte_idx = text.nth_char_byte_offset(char_idx);

    let mut prev_start = 0;
    let mut seg_start = 0;
    let mut seg_end = text.len();
    for b in segmenters.word.as_borrowed().segment_str(&text) {
        if b > byte_idx {
            seg_end = b;
            break;
        }
        prev_start = seg_start;
        seg_start = b;
    }

    if seg_start == seg_end {
        seg_start = prev_start;
    }

    let start = text.nth_byte_char_offset(seg_start);
    let end = text.nth_byte_char_offset(seg_end);
    let anchor = position_at_char_index(line, start)?;
    let head = position_at_char_index(line, end)?;
    Some(Selection::new(anchor, head))
}

pub fn select_paragraph_at(tree: &LayoutTree, pos: &Position) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;
    let para_id = match &line_node.content {
        LayoutContent::Line(l) => l.node_id,
        LayoutContent::Atom(a) => a.parent_id,
        _ => return None,
    };
    let container = search::find_box_by_node_id(&tree.root, para_id)?;
    let first = search::find_first_navigable(container)?;
    let last = search::find_last_navigable(container)?;
    Some(Selection::new(
        first_position_in(first),
        last_position_in(last),
    ))
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
    use crate::style::Alignment;
    use editor_common::{EdgeInsets, Rect};
    use editor_macros::doc;
    use editor_model::NodeId;

    use super::*;
    use crate::glyph_run::{GlyphRun, GraphemeSpan};
    use crate::style::{BorderMode, BoxStyle, Direction as LayoutDirection};

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
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![GlyphRun::make_test_run(id, 0, text, 0.0, gs(n))],
            text_indent: 0.0,
        }
    }

    fn make_multi_segment_line() -> (LayoutLine, NodeId, NodeId) {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let line = LayoutLine {
            node_id: id1,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![
                GlyphRun::make_test_run(id1, 0, "hello ", 0.0, gs(6)),
                GlyphRun::make_test_run(id2, 0, "world", 60.0, gs(5)),
            ],
            text_indent: 0.0,
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

    fn make_box_style() -> BoxStyle {
        BoxStyle {
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::ZERO,
            border_mode: BorderMode::Separate,
            alignment: Alignment::Start,
            scope: false,
            decorations: vec![],
        }
    }

    #[test]
    fn select_paragraph_at_selects_full_paragraph() {
        let para_id = NodeId::new();
        let text_id = NodeId::new();
        let n = "hello world".chars().count();
        let line_node = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 110.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: para_id,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(
                    text_id,
                    0,
                    "hello world",
                    0.0,
                    gs(n),
                )],
                text_indent: 0.0,
            }),
        };
        let tree = LayoutTree {
            root: LayoutNode {
                rect: Rect::from_xywh(0.0, 0.0, 110.0, 20.0),
                content: LayoutContent::Box(LayoutBox {
                    node_id: para_id,
                    style: make_box_style(),
                    children: vec![line_node],
                }),
            },
        };
        let pos = Position::new(text_id, 3);

        let sel = select_paragraph_at(&tree, &pos).unwrap();
        assert_eq!(sel.anchor, Position::new(text_id, 0));
        assert_eq!(sel.head.node_id, text_id);
        assert_eq!(sel.head.offset, 11);
    }

    #[test]
    fn select_word_at_middle_of_word() {
        let (doc, id) = doc! { root { paragraph { id: text("hello world") } } };
        let line_node = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 110.0, 20.0),
            content: LayoutContent::Line(make_line(id, "hello world")),
        };
        let tree = LayoutTree {
            root: LayoutNode {
                rect: Rect::from_xywh(0.0, 0.0, 110.0, 20.0),
                content: LayoutContent::Box(LayoutBox {
                    node_id: NodeId::new(),
                    style: make_box_style(),
                    children: vec![line_node],
                }),
            },
        };
        let segmenters = TextSegmenters::new_test();
        let pos = Position::new(id, 2).resolve(&doc).unwrap(); // "he|llo world"

        let sel = select_word_at(&tree, &pos, &segmenters).unwrap();
        assert_eq!(sel.anchor, Position::new(id, 0));
        assert_eq!(sel.head.node_id, id);
        assert!(sel.head.offset > 0 && sel.head.offset <= 5);
    }

    #[test]
    fn select_word_at_word_boundary() {
        let (doc, id) = doc! { root { paragraph { id: text("hello world") } } };
        let line_node = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 110.0, 20.0),
            content: LayoutContent::Line(make_line(id, "hello world")),
        };
        let tree = LayoutTree {
            root: LayoutNode {
                rect: Rect::from_xywh(0.0, 0.0, 110.0, 20.0),
                content: LayoutContent::Box(LayoutBox {
                    node_id: NodeId::new(),
                    style: make_box_style(),
                    children: vec![line_node],
                }),
            },
        };
        let segmenters = TextSegmenters::new_test();
        let pos = Position::new(id, 5).resolve(&doc).unwrap(); // "hello| world"

        let sel = select_word_at(&tree, &pos, &segmenters).unwrap();
        assert_ne!(sel.anchor, sel.head);
    }

    #[test]
    fn select_word_at_end_of_word() {
        let (doc, id) = doc! { root { paragraph { id: text("hello world") } } };
        let line_node = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 110.0, 20.0),
            content: LayoutContent::Line(make_line(id, "hello world")),
        };
        let tree = LayoutTree {
            root: LayoutNode {
                rect: Rect::from_xywh(0.0, 0.0, 110.0, 20.0),
                content: LayoutContent::Box(LayoutBox {
                    node_id: NodeId::new(),
                    style: make_box_style(),
                    children: vec![line_node],
                }),
            },
        };
        let segmenters = TextSegmenters::new_test();
        let pos = Position::new(id, 11).resolve(&doc).unwrap(); // "hello world|"

        let sel = select_word_at(&tree, &pos, &segmenters).unwrap();
        assert_eq!(sel.anchor.node_id, id);
        assert!(sel.anchor.offset >= 6);
        assert_eq!(sel.head.node_id, id);
        assert_eq!(sel.head.offset, 11);
        assert_ne!(sel.anchor, sel.head);
    }

    #[test]
    fn select_word_at_empty_line() {
        let (doc, p1, ..) = doc! {
            root {
                p1: paragraph {}
            }
        };
        let line_node = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 100.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: p1,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![],
                text_indent: 0.0,
            }),
        };
        let tree = LayoutTree {
            root: LayoutNode {
                rect: Rect::from_xywh(0.0, 0.0, 100.0, 20.0),
                content: LayoutContent::Box(LayoutBox {
                    node_id: NodeId::ROOT,
                    style: make_box_style(),
                    children: vec![line_node],
                }),
            },
        };
        let segmenters = TextSegmenters::new_test();
        let pos = Position::new(p1, 0).resolve(&doc).unwrap();

        let sel = select_word_at(&tree, &pos, &segmenters).unwrap();
        assert_eq!(sel.anchor.node_id, NodeId::ROOT);
        assert_eq!(sel.anchor.offset, 0);
        assert_eq!(sel.head.node_id, NodeId::ROOT);
        assert_eq!(sel.head.offset, 1);
    }
}
