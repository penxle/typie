//! Semantic text in document order with geometry from the existing layout.
//! This is independent of rendered pages, tiles, and the editor selection.
use crate::PageRect;
use editor_common::Rect;
use editor_crdt::Dot;
use editor_macros::ffi;
use editor_model::{ChildView, DocView, NodeType};
use editor_state::{Position, Selection};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::paginate::types::{LayoutContent, LayoutLine};
use crate::query::layout_index::LayoutIndex;

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionFont {
    pub family: u16,
    pub weight: u16,
    pub key: String,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionTextRun {
    pub font: Option<SelectionFont>,
    pub offset: usize,
    pub text: String,
    pub page_idx: usize,
    pub rect: Rect,
    pub line: Rect,
    pub letter_spacing: f32,
    pub font_size: f32,
    pub weight: u16,
    pub italic: bool,
    pub rtl: bool,
    pub link: Option<String>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionLayoutBlock {
    pub node: Dot,
    pub kind: NodeType,
    pub before: Position,
    pub after: Position,
    pub runs: Vec<SelectionTextRun>,
    /// The final newline represents this structural range, not a hard-break leaf.
    pub trailing_break: Option<Selection>,
}

pub(crate) fn selection_layout(
    index: &LayoutIndex,
    doc: &DocView,
    fonts: &editor_resource::FontRegistry,
) -> Vec<SelectionLayoutBlock> {
    let mut text_blocks: HashMap<Dot, Vec<SelectionTextRun>> = HashMap::new();
    let mut atoms = HashMap::new();
    for entry in index.entries() {
        let Some(node) = entry.node(index) else {
            continue;
        };
        match &node.content {
            LayoutContent::Line(line) if !line.is_phantom => {
                let Some(page) = index.page_rect(node.rect) else {
                    continue;
                };
                let runs = text_blocks.entry(line.node).or_default();
                for run in &line.glyph_runs {
                    runs.push(SelectionTextRun {
                        font: fonts
                            .metrics_font_key(run.family_id, run.weight)
                            .map(|key| SelectionFont {
                                family: run.family_id,
                                weight: run.weight,
                                key,
                            }),
                        offset: run.offset_range.start,
                        text: run.text.clone(),
                        rtl: run.rtl,
                        page_idx: page.page_idx,
                        line: page.rect,
                        rect: Rect::from_xywh(
                            node.rect.x + run.x,
                            page.rect.y + line.baseline - run.cursor_ascent,
                            run.width,
                            run.cursor_ascent + run.cursor_descent,
                        ),
                        letter_spacing: run.letter_spacing,
                        font_size: run.font_size,
                        weight: run.weight,
                        italic: run.synthesis.skew.is_some(),
                        link: run.link.clone(),
                    });
                }
                for tab in &line.tab_gaps {
                    runs.push(SelectionTextRun {
                        font: None,
                        offset: tab.offset_index,
                        text: "\t".into(),
                        page_idx: page.page_idx,
                        line: page.rect,
                        rect: Rect::from_xywh(
                            node.rect.x + tab.x,
                            page.rect.y + line.baseline - line.cursor_ascent,
                            tab.width,
                            line.cursor_ascent + line.cursor_descent,
                        ),
                        letter_spacing: 0.0,
                        font_size: line.cursor_ascent + line.cursor_descent,
                        weight: 400,
                        italic: false,
                        rtl: false,
                        link: tab.link.clone(),
                    });
                }
                // Empty paragraphs and hard breaks still have a selectable boundary.
                if let Some(range) = &line.offset_range {
                    let end = range.end;
                    let hard_break = doc.node(line.node).and_then(|node| node.child_at(end))
                        .is_some_and(|child| matches!(child, ChildView::Leaf(leaf) if leaf.node_type() == NodeType::HardBreak));
                    if hard_break || line.glyph_runs.is_empty() && line.tab_gaps.is_empty() {
                        runs.push(boundary_run(
                            line,
                            page,
                            end,
                            if hard_break { "\n" } else { "" },
                        ));
                    }
                }
            }
            LayoutContent::Atom(atom) => {
                atoms.insert(atom.node, atom.attachment.clone());
            }
            _ => {}
        }
    }

    let mut result = Vec::new();
    let Some(root) = doc.root() else {
        return result;
    };
    let mut stack: Vec<_> = root.children().collect();
    stack.reverse();
    while let Some(child) = stack.pop() {
        match child {
            ChildView::Block(node) => {
                if let Some(mut runs) = text_blocks.remove(&node.id()) {
                    // The source document owns logical text; keep only direction
                    // in GlyphRun rather than duplicating every RTL string there.
                    if runs.iter().any(|run| run.rtl) {
                        let characters: Vec<_> = node
                            .children()
                            .map(|child| match child {
                                ChildView::Leaf(leaf) => leaf.as_char(),
                                ChildView::Block(_) => None,
                            })
                            .collect();
                        for run in runs.iter_mut().filter(|run| run.rtl) {
                            let end = run.offset + run.text.chars().count();
                            run.text = characters[run.offset..end]
                                .iter()
                                .filter_map(|ch| *ch)
                                .collect();
                        }
                    }
                    let trailing_break =
                        crate::query::trailing_break::trailing_break_occurrence_for_node(
                            index,
                            doc,
                            node.id(),
                        )
                        .and_then(|boundary| {
                            let entry = index.entry_for_position(&boundary.range.anchor)?;
                            let LayoutContent::Line(line) = entry.content(index)? else {
                                return None;
                            };
                            let page = index.page_rect(entry.rect)?;
                            // An empty paragraph needs one boundary, not both a
                            // placeholder <br> and a structural newline.
                            runs.retain(|run| {
                                !run.text.is_empty()
                                    || run.page_idx != page.page_idx
                                    || run.line != page.rect
                            });
                            runs.push(boundary_run(line, page, boundary.range.anchor.offset, "\n"));
                            Some(boundary.range)
                        });
                    runs.sort_by_key(|run| run.offset);
                    result.push(SelectionLayoutBlock {
                        node: node.id(),
                        kind: node.node_type(),
                        before: Position::new(node.id(), 0),
                        after: Position::new(node.id(), node.child_count()),
                        runs,
                        trailing_break,
                    });
                }
                // Inline children are already represented by runs. Hidden text
                // blocks have no runs, but need no character traversal either.
                if !node.node_type().spec().is_textblock() {
                    let children: Vec<_> = node.children().collect();
                    stack.extend(children.into_iter().rev());
                }
            }
            ChildView::Leaf(leaf) => {
                if let Some(attachment) = atoms.remove(&leaf.dot()) {
                    result.push(SelectionLayoutBlock {
                        node: leaf.dot(),
                        kind: leaf.node_type(),
                        before: Position::new(attachment.parent, attachment.index),
                        after: Position::new(attachment.parent, attachment.index + 1),
                        runs: Vec::new(),
                        trailing_break: None,
                    });
                }
            }
        }
    }
    result
}

fn boundary_run(line: &LayoutLine, page: PageRect, offset: usize, text: &str) -> SelectionTextRun {
    SelectionTextRun {
        font: None,
        offset,
        text: text.into(),
        page_idx: page.page_idx,
        line: page.rect,
        rect: Rect::from_xywh(
            page.rect.x
                + crate::query::grapheme::x_at_offset(
                    line,
                    &Position {
                        node: line.node,
                        offset,
                        affinity: editor_state::Affinity::Upstream,
                    },
                ),
            page.rect.y + line.baseline - line.cursor_ascent,
            1.0,
            line.cursor_ascent + line.cursor_descent,
        ),
        letter_spacing: 0.0,
        font_size: line.cursor_ascent + line.cursor_descent,
        weight: 400,
        italic: false,
        rtl: false,
        link: None,
    }
}
