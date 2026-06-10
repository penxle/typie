use editor_common::StrExt;
use editor_resource::TextSegmenters;
use editor_state::{Position, Selection};
use icu_segmenter::{SentenceSegmenter, WordSegmenter};

use crate::glyph_run::GlyphRun;
use crate::measure::TabGap;
use crate::paginate::*;

use super::layout_index::LayoutIndex;
use super::navigation::{landed_entry, next_navigable_entry, prev_navigable_entry};

pub fn move_word_forward(
    layout_index: &LayoutIndex,
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;
    let line = match entry.content(layout_index)? {
        LayoutContent::Line(l) => l,
        LayoutContent::Atom(_) | LayoutContent::Box(LayoutBox { nav: Some(_), .. }) => {
            let next = next_navigable_entry(layout_index, entry)?;
            return Some(landed_entry(layout_index, next, false, true));
        }
        _ => return None,
    };
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = next_word_boundary(line, char_idx, &segmenters.word) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let next = next_navigable_entry(layout_index, entry)?;
    Some(landed_entry(layout_index, next, false, true))
}

pub fn move_word_backward(
    layout_index: &LayoutIndex,
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;
    let line = match entry.content(layout_index)? {
        LayoutContent::Line(l) => l,
        LayoutContent::Atom(_) | LayoutContent::Box(LayoutBox { nav: Some(_), .. }) => {
            let prev = prev_navigable_entry(layout_index, entry)?;
            return Some(landed_entry(layout_index, prev, true, false));
        }
        _ => return None,
    };
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = prev_word_boundary(line, char_idx, &segmenters.word) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let prev = prev_navigable_entry(layout_index, entry)?;
    Some(landed_entry(layout_index, prev, true, false))
}

pub fn move_sentence_forward(
    layout_index: &LayoutIndex,
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;
    let line = match entry.content(layout_index)? {
        LayoutContent::Line(l) => l,
        LayoutContent::Atom(_) | LayoutContent::Box(LayoutBox { nav: Some(_), .. }) => {
            let next = next_navigable_entry(layout_index, entry)?;
            return Some(landed_entry(layout_index, next, false, true));
        }
        _ => return None,
    };
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = next_sentence_boundary(line, char_idx, &segmenters.sentence) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let next = next_navigable_entry(layout_index, entry)?;
    Some(landed_entry(layout_index, next, false, true))
}

pub fn move_sentence_backward(
    layout_index: &LayoutIndex,
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;
    let line = match entry.content(layout_index)? {
        LayoutContent::Line(l) => l,
        LayoutContent::Atom(_) | LayoutContent::Box(LayoutBox { nav: Some(_), .. }) => {
            let prev = prev_navigable_entry(layout_index, entry)?;
            return Some(landed_entry(layout_index, prev, true, false));
        }
        _ => return None,
    };
    let char_idx = line_char_index(line, pos)?;

    if let Some(boundary) = prev_sentence_boundary(line, char_idx, &segmenters.sentence) {
        let new_pos = position_at_char_index(line, boundary)?;
        return Some(Selection::collapsed(new_pos));
    }

    let prev = prev_navigable_entry(layout_index, entry)?;
    Some(landed_entry(layout_index, prev, true, false))
}

enum LineItem<'a> {
    Run(&'a GlyphRun),
    Tab(&'a TabGap),
}

fn line_items(line: &LayoutLine) -> Vec<LineItem<'_>> {
    let mut items: Vec<(f32, LineItem<'_>)> =
        Vec::with_capacity(line.glyph_runs.len() + line.tab_gaps.len());
    for run in &line.glyph_runs {
        items.push((run.x, LineItem::Run(run)));
    }
    for gap in &line.tab_gaps {
        items.push((gap.x, LineItem::Tab(gap)));
    }
    items.sort_by(|a, b| a.0.total_cmp(&b.0));
    items.into_iter().map(|(_, it)| it).collect()
}

pub fn line_char_index(line: &LayoutLine, pos: &Position) -> Option<usize> {
    let mut char_count = 0;
    for item in line_items(line) {
        match item {
            LineItem::Run(run) => {
                let run_chars = run.text.char_count();
                if run.node_id == pos.node_id
                    && let Some(local) = pos.offset.checked_sub(run.offset)
                    && local <= run_chars
                {
                    return Some(char_count + local);
                }
                char_count += run_chars;
            }
            LineItem::Tab(gap) => {
                if pos.node_id == line.node_id && pos.offset == gap.child_index {
                    return Some(char_count);
                }
                char_count += 1;
            }
        }
    }
    None
}

pub fn position_at_char_index(line: &LayoutLine, char_index: usize) -> Option<Position> {
    let mut remaining = char_index;
    for item in line_items(line) {
        match item {
            LineItem::Run(run) => {
                let run_chars = run.text.char_count();
                if remaining <= run_chars {
                    return Some(Position::new(run.node_id, run.offset + remaining));
                }
                remaining -= run_chars;
            }
            LineItem::Tab(gap) => {
                if remaining == 0 {
                    return Some(Position::new(line.node_id, gap.child_index));
                }
                remaining -= 1;
            }
        }
    }
    None
}

fn line_text(line: &LayoutLine) -> String {
    let mut text = String::new();
    for item in line_items(line) {
        match item {
            LineItem::Run(run) => text.push_str(&run.text),
            LineItem::Tab(_) => text.push('\t'),
        }
    }
    text
}

fn next_word_boundary(
    line: &LayoutLine,
    char_index: usize,
    segmenter: &WordSegmenter,
) -> Option<usize> {
    let text = line_text(line);
    let boundaries: Vec<_> = segmenter
        .as_borrowed()
        .segment_str(&text)
        .map(|b| text.nth_byte_char_offset(b))
        .collect();
    if boundaries.len() < 2 {
        return None;
    }

    let idx = boundaries.partition_point(|&b| b <= char_index).max(1);
    (idx..boundaries.len())
        .find(|&i| {
            let start = boundaries[i - 1];
            let end = boundaries[i];
            !is_whitespace_segment(&text, start, end)
        })
        .map(|i| boundaries[i])
}

fn prev_word_boundary(
    line: &LayoutLine,
    char_index: usize,
    segmenter: &WordSegmenter,
) -> Option<usize> {
    let text = line_text(line);
    if char_index == 0 {
        return None;
    }

    let boundaries: Vec<_> = segmenter
        .as_borrowed()
        .segment_str(&text)
        .map(|b| text.nth_byte_char_offset(b))
        .collect();
    if boundaries.len() < 2 {
        return None;
    }

    let idx = boundaries.partition_point(|&b| b < char_index);
    if idx == 0 {
        return None;
    }

    (0..idx)
        .rev()
        .find(|&i| {
            let start = boundaries[i];
            let end = *boundaries.get(i + 1).unwrap_or(&text.char_count());
            !is_whitespace_segment(&text, start, end)
        })
        .map(|i| boundaries[i])
        .or(Some(boundaries[0]))
}

fn is_whitespace_segment(text: &str, start: usize, end: usize) -> bool {
    let byte_start = text.nth_char_byte_offset(start);
    let byte_end = text.nth_char_byte_offset(end);
    text[byte_start..byte_end].chars().all(char::is_whitespace)
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
    use crate::page::LayoutPage;
    use crate::query::layout_index::LayoutIndex;
    use crate::style::Alignment;
    use editor_common::{EdgeInsets, Rect, Size};
    use editor_model::NodeId;
    use editor_state::Affinity;

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
            ruby_annotations: vec![],
            empty_caret_x: 0.0,
            child_range: None,
            tab_gaps: vec![],
            is_phantom: false,
            content_edge_x: None,
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
            ruby_annotations: vec![],
            empty_caret_x: 0.0,
            child_range: None,
            tab_gaps: vec![],
            is_phantom: false,
            content_edge_x: None,
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
    fn word_forward_skips_whitespace_between_words() {
        let id = NodeId::new();
        let tree = make_single_line_tree(make_line(id, "hello  world"));
        let layout_index = layout_index(&tree);
        let segmenters = TextSegmenters::new_test();
        let selection =
            move_word_forward(&layout_index, &Position::new(id, 5), &segmenters).unwrap();
        assert_eq!(selection.head.node_id, id);
        assert_eq!(selection.head.offset, 12);
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
    fn word_backward_skips_whitespace_between_words() {
        let id = NodeId::new();
        let tree = make_single_line_tree(make_line(id, "hello  world"));
        let layout_index = layout_index(&tree);
        let segmenters = TextSegmenters::new_test();
        let selection =
            move_word_backward(&layout_index, &Position::new(id, 7), &segmenters).unwrap();
        assert_eq!(selection.head.node_id, id);
        assert_eq!(selection.head.offset, 0);
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
            decorations: vec![],
            monolithic: false,
        }
    }

    fn make_single_line_tree(line: LayoutLine) -> LayoutTree {
        let width = line.glyph_runs.iter().map(|run| run.width).sum::<f32>();
        LayoutTree {
            root: LayoutNode {
                rect: Rect::from_xywh(0.0, 0.0, width, 20.0),
                content: LayoutContent::Box(LayoutBox {
                    node_id: NodeId::new(),
                    style: make_box_style(),
                    children: vec![LayoutNode {
                        rect: Rect::from_xywh(0.0, 0.0, width, 20.0),
                        content: LayoutContent::Line(line),
                    }],
                    nav: None,
                }),
            },
        }
    }

    fn layout_index(tree: &LayoutTree) -> LayoutIndex {
        let pages = [LayoutPage::new(0.0, 10_000.0, Size::new(1_000.0, 10_000.0))];
        LayoutIndex::new(tree.clone(), &pages)
    }

    #[test]
    fn word_forward_onto_atom_selects_atom() {
        use crate::paginate::{LayoutAtom, LayoutContent, LayoutNode, LayoutTree};
        let para = NodeId::new();
        let atom_parent = NodeId::new();
        let atom_id = NodeId::new();
        let line = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 50.0, 20.0),
            content: LayoutContent::Line(make_line(para, "hi")),
        };
        let atom = LayoutNode {
            rect: Rect::from_xywh(0.0, 20.0, 200.0, 20.0),
            content: LayoutContent::Atom(LayoutAtom {
                node_id: atom_id,
                parent_id: atom_parent,
                index: 0,
            }),
        };
        let tree = LayoutTree {
            root: LayoutNode {
                rect: Rect::from_xywh(0.0, 0.0, 200.0, 40.0),
                content: LayoutContent::Box(LayoutBox {
                    node_id: NodeId::new(),
                    style: make_box_style(),
                    children: vec![line, atom],
                    nav: None,
                }),
            },
        };
        let segmenters = TextSegmenters::new_test();
        // Moving word-forward from the end of "hi" has no in-line boundary; the next navigable is the atom.
        let layout_index = layout_index(&tree);
        let sel = move_word_forward(&layout_index, &Position::new(para, 2), &segmenters).unwrap();
        assert!(
            !sel.is_collapsed(),
            "word-forward onto atom must node-select, got {:?}",
            sel
        );
        assert_eq!(
            sel.anchor,
            Position {
                node_id: atom_parent,
                offset: 0,
                affinity: Affinity::Downstream
            }
        );
        assert_eq!(
            sel.head,
            Position {
                node_id: atom_parent,
                offset: 1,
                affinity: Affinity::Upstream
            }
        );
    }

    #[test]
    fn word_forward_from_selected_atom_passes_to_next_text() {
        use crate::paginate::{LayoutAtom, LayoutContent, LayoutNode, LayoutTree};
        let para = NodeId::new();
        let atom_parent = NodeId::new();
        let atom_id = NodeId::new();
        let atom = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
            content: LayoutContent::Atom(LayoutAtom {
                node_id: atom_id,
                parent_id: atom_parent,
                index: 0,
            }),
        };
        let line = LayoutNode {
            rect: Rect::from_xywh(0.0, 20.0, 50.0, 20.0),
            content: LayoutContent::Line(make_line(para, "hi")),
        };
        let tree = LayoutTree {
            root: LayoutNode {
                rect: Rect::from_xywh(0.0, 0.0, 200.0, 40.0),
                content: LayoutContent::Box(LayoutBox {
                    node_id: NodeId::new(),
                    style: make_box_style(),
                    children: vec![atom, line],
                    nav: None,
                }),
            },
        };
        let segmenters = TextSegmenters::new_test();
        // Forward head of the atom node-selection is (atom_parent, 1, Upstream).
        // Position ownership returns Atom, so word-forward passes through to the next navigable Line.
        let pos = Position {
            node_id: atom_parent,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let layout_index = layout_index(&tree);
        let sel = move_word_forward(&layout_index, &pos, &segmenters).unwrap();
        assert!(
            sel.is_collapsed(),
            "passing atom must yield text caret, got {:?}",
            sel
        );
        assert_eq!(sel.head.node_id, para);
        assert_eq!(sel.head.offset, 0);
    }
}
