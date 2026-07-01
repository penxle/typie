use editor_common::StrExt;
use editor_resource::TextSegmenters;
use editor_state::{Position, Selection};
use icu_segmenter::{SentenceSegmenter, WordSegmenter};

use crate::glyph_run::GlyphRun;
use crate::measure::text::measure::TabGap;
use crate::paginate::types::{LayoutContent, LayoutLine};

use super::layout_index::LayoutIndex;
use super::navigation::{
    landed_entry, move_box_boundary, next_navigable_entry, prev_navigable_entry,
};

pub(crate) fn move_word_forward(
    layout_index: &LayoutIndex,
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;
    let line = match entry.content(layout_index)? {
        LayoutContent::Line(l) => l,
        LayoutContent::Atom(_) => {
            let next = next_navigable_entry(layout_index, entry)?;
            return Some(landed_entry(layout_index, next, false, true));
        }
        LayoutContent::Box(b) => {
            return move_box_boundary(layout_index, entry, b, pos, true);
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

pub(crate) fn move_word_backward(
    layout_index: &LayoutIndex,
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;
    let line = match entry.content(layout_index)? {
        LayoutContent::Line(l) => l,
        LayoutContent::Atom(_) => {
            let prev = prev_navigable_entry(layout_index, entry)?;
            return Some(landed_entry(layout_index, prev, true, false));
        }
        LayoutContent::Box(b) => {
            return move_box_boundary(layout_index, entry, b, pos, false);
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

pub(crate) fn move_sentence_forward(
    layout_index: &LayoutIndex,
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;
    let line = match entry.content(layout_index)? {
        LayoutContent::Line(l) => l,
        LayoutContent::Atom(_) => {
            let next = next_navigable_entry(layout_index, entry)?;
            return Some(landed_entry(layout_index, next, false, true));
        }
        LayoutContent::Box(b) => {
            return move_box_boundary(layout_index, entry, b, pos, true);
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

pub(crate) fn move_sentence_backward(
    layout_index: &LayoutIndex,
    pos: &Position,
    segmenters: &TextSegmenters,
) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;
    let line = match entry.content(layout_index)? {
        LayoutContent::Line(l) => l,
        LayoutContent::Atom(_) => {
            let prev = prev_navigable_entry(layout_index, entry)?;
            return Some(landed_entry(layout_index, prev, true, false));
        }
        LayoutContent::Box(b) => {
            return move_box_boundary(layout_index, entry, b, pos, false);
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

pub(crate) fn line_char_index(line: &LayoutLine, pos: &Position) -> Option<usize> {
    if line_items(line).is_empty()
        && pos.node == line.node
        && line
            .offset_range
            .as_ref()
            .is_none_or(|range| pos.offset >= range.start && pos.offset <= range.end)
    {
        return Some(0);
    }

    let mut char_count = 0;
    for item in line_items(line) {
        match item {
            LineItem::Run(run) => {
                let run_chars = run.text.char_count();
                if pos.node == line.node
                    && let Some(local) = pos.offset.checked_sub(run.offset_range.start)
                    && local <= run_chars
                {
                    return Some(char_count + local);
                }
                char_count += run_chars;
            }
            LineItem::Tab(gap) => {
                if pos.node == line.node && pos.offset == gap.offset_index {
                    return Some(char_count);
                }
                char_count += 1;
            }
        }
    }
    None
}

pub(crate) fn position_at_char_index(line: &LayoutLine, char_index: usize) -> Option<Position> {
    let mut remaining = char_index;
    for item in line_items(line) {
        match item {
            LineItem::Run(run) => {
                let run_chars = run.text.char_count();
                if remaining <= run_chars {
                    return Some(Position::new(line.node, run.offset_range.start + remaining));
                }
                remaining -= run_chars;
            }
            LineItem::Tab(gap) => {
                if remaining == 0 {
                    return Some(Position::new(line.node, gap.offset_index));
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
    use editor_common::EdgeInsets;
    use editor_common::Size;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeMarkerLog, NodeStyleLog, NodeType,
        SeqItem, SpanLog, StyleLog, project_document,
    };
    use editor_resource::Resource;
    use editor_state::Position;

    use crate::glyph_run::GlyphRun;
    use crate::glyph_run::{GraphemeSpan, Synthesis, TextDecoration};
    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;
    use crate::page::LayoutPage;
    use crate::paginate::paginator::Paginator;
    use crate::paginate::types::{LayoutBox, LayoutContent, LayoutLine, LayoutNode, LayoutTree};
    use crate::style::BoxStyle;

    use super::super::layout_index::LayoutIndex;
    use super::*;

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_styles: NodeStyleLog::new(),
            node_markers: NodeMarkerLog::new(),
            styles: StyleLog::new(),
        }
    }

    fn build_index(doc: &DocLogs, width: f32) -> (Dot, LayoutIndex) {
        let pd = project_document(doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let root_id = root_node.id();
        let mut res = Resource::new_test();
        let measured = measure_node(
            &mut crate::measure::Measurer::new(),
            &root_node,
            width,
            &MeasureContext::default(),
            &mut res,
        );
        let layout = Paginator::continuous(width, 100_000.0, EdgeInsets::all(0.0))
            .paginate(MeasuredTree { root: measured });
        let index = LayoutIndex::new(layout.tree, &layout.pages);
        (root_id, index)
    }

    fn para_doc(text: &str, width: f32) -> (Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
            },
        )];
        for (i, ch) in text.chars().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), SeqItem::Char(ch)));
        }
        let doc = logs(&items);
        let para_id = para;
        let (_, index) = build_index(&doc, width);
        (para_id, index)
    }

    fn gs(advance: f32, codepoints: u8) -> GraphemeSpan {
        GraphemeSpan {
            advance,
            codepoints,
        }
    }

    fn vrun(
        offset_range: std::ops::Range<usize>,
        x: f32,
        text: &str,
        graphemes: Vec<GraphemeSpan>,
    ) -> GlyphRun {
        let width = graphemes.iter().map(|g| g.advance).sum();
        GlyphRun {
            family_id: 0,
            weight: 400,
            font_size: 16.0,
            synthesis: Synthesis::default(),
            color: String::new(),
            background_color: None,
            glyphs: vec![],
            decoration: TextDecoration::default(),
            offset_range,
            link: None,
            text: text.to_string(),
            x,
            width,
            graphemes,
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        }
    }

    fn vline(
        node: Dot,
        offset_range: Option<std::ops::Range<usize>>,
        runs: Vec<GlyphRun>,
    ) -> LayoutLine {
        LayoutLine {
            node,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: runs,
            ruby_annotations: vec![],
            empty_caret_x: 0.0,
            offset_range,
            tab_gaps: vec![],
            is_phantom: false,
            content_edge_x: None,
        }
    }

    fn make_single_line_index(para_id: Dot, line: LayoutLine, line_height: f32) -> LayoutIndex {
        let root_id = Dot::ROOT;
        let line_rect = editor_common::Rect::from_xywh(0.0, 0.0, 110.0, line_height);
        let para_rect = editor_common::Rect::from_xywh(0.0, 0.0, 110.0, line_height);
        let root_rect = editor_common::Rect::from_xywh(0.0, 0.0, 110.0, line_height);

        let tree = LayoutTree {
            root: LayoutNode {
                rect: root_rect,
                content: LayoutContent::Box(LayoutBox {
                    node: root_id,
                    style: BoxStyle::default(),
                    children: vec![LayoutNode {
                        rect: para_rect,
                        content: LayoutContent::Box(LayoutBox {
                            node: para_id,
                            style: BoxStyle::default(),
                            children: vec![LayoutNode {
                                rect: line_rect,
                                content: LayoutContent::Line(line),
                            }],
                            attachment: None,
                        }),
                    }],
                    attachment: None,
                }),
            },
        };

        let page = LayoutPage::new(
            0.0,
            line_height + 1.0,
            Size {
                width: 110.0,
                height: line_height + 1.0,
            },
        );
        LayoutIndex::new(tree, &[page])
    }

    #[test]
    fn word_forward_backward() {
        let (para_id, index) = para_doc("hello world", 400.0);
        let res = Resource::new_test();

        let pos0 = Position::new(para_id, 0);
        let sel_fwd = move_word_forward(&index, &pos0, &res.segmenters)
            .expect("word_forward from 0 must resolve");
        assert_eq!(sel_fwd.head.node, para_id);
        assert!(
            sel_fwd.head.offset > 0 && sel_fwd.head.offset <= 11,
            "word_forward offset must be in (0, 11], got {}",
            sel_fwd.head.offset
        );

        let sel_bwd = move_word_backward(&index, &sel_fwd.head, &res.segmenters)
            .expect("word_backward must resolve");
        assert_eq!(sel_bwd.head.node, para_id);
        assert_eq!(
            sel_bwd.head.offset, 0,
            "word_backward from word boundary must return to 0"
        );
    }

    #[test]
    fn sentence_forward_backward() {
        let text = "Hello world. Goodbye world.";
        let (para_id, index) = para_doc(text, 800.0);
        let res = Resource::new_test();

        let pos0 = Position::new(para_id, 0);
        let sel_fwd = move_sentence_forward(&index, &pos0, &res.segmenters)
            .expect("sentence_forward from 0 must resolve");
        assert_eq!(sel_fwd.head.node, para_id);
        assert!(
            sel_fwd.head.offset > 0 && sel_fwd.head.offset <= text.len(),
            "sentence_forward offset must be positive, got {}",
            sel_fwd.head.offset
        );

        let sel_bwd = move_sentence_backward(&index, &sel_fwd.head, &res.segmenters)
            .expect("sentence_backward must resolve");
        assert_eq!(sel_bwd.head.node, para_id);
        assert_eq!(
            sel_bwd.head.offset, 0,
            "sentence_backward from boundary must return to 0"
        );
    }

    #[test]
    fn word_forward_multi_run() {
        let para_id = Dot::new(1, 1);

        let run0 = vrun(0..6, 0.0, "hello ", vec![gs(10.0, 1); 6]);
        let run1 = vrun(6..11, 60.0, "world", vec![gs(10.0, 1); 5]);
        let line = vline(para_id, Some(0..11), vec![run0, run1]);

        let index = make_single_line_index(para_id, line, 20.0);
        let res = Resource::new_test();

        // From offset 5 (end of "hello" in run0), word-forward skips the space
        // and must land at offset 11 (end of "world" in run1), crossing the
        // run boundary identified solely by offset_range (not per-run node identity).
        let pos = Position::new(para_id, 5);
        let sel = move_word_forward(&index, &pos, &res.segmenters)
            .expect("word_forward from offset 5 must resolve");
        assert_eq!(sel.head.node, para_id);
        assert_eq!(
            sel.head.offset, 11,
            "word_forward from offset 5 must cross run boundary to offset 11 (end of 'world' in run1)"
        );
    }
}
