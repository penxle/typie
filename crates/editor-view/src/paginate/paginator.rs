use editor_common::{EdgeInsets, Rect, Size};

use editor_crdt::Dot;
use editor_state::Position;

use crate::measure::PageBreakPolicy;
use crate::measure::types::{MeasuredBox, MeasuredContent, MeasuredNode, MeasuredTree};
use crate::page::LayoutPage;
use crate::style::*;

use super::types::*;

pub(crate) struct Paginator {
    paginated: bool,
    page_width: f32,
    page_height: f32,
    content_width: f32,
    content_height: f32,
    margins: EdgeInsets,
    accumulated_y: f32,
    current_x: f32,
    page_content_top: f32,
    page_content_bottom: f32,
    pages: Vec<LayoutPage>,
}

impl Paginator {
    pub fn paginated(page_width: f32, page_height: f32, margins: EdgeInsets) -> Self {
        let content_width = page_width - margins.left - margins.right;
        let content_height = page_height - margins.top - margins.bottom;
        Self {
            paginated: true,
            page_width,
            page_height,
            content_width,
            content_height,
            margins,
            accumulated_y: margins.top,
            current_x: margins.left,
            page_content_top: margins.top,
            page_content_bottom: margins.top + content_height,
            pages: vec![],
        }
    }

    pub fn continuous(page_width: f32, max_content_height: f32, margins: EdgeInsets) -> Self {
        let content_width = page_width - margins.left - margins.right;
        Self {
            paginated: false,
            page_width,
            page_height: 0.0,
            content_width,
            content_height: max_content_height,
            margins,
            accumulated_y: margins.top,
            current_x: margins.left,
            page_content_top: margins.top,
            page_content_bottom: margins.top + max_content_height,
            pages: vec![],
        }
    }

    pub fn paginate(mut self, tree: MeasuredTree) -> PaginatedLayout {
        let root_id = match &tree.root.content {
            MeasuredContent::Box(b) => b.node,
            _ => unreachable!("measured document root is always a Box"),
        };
        let root = self.place_node(&tree.root, root_id, 0, 0.0);
        let pages = self.finish();
        let tree = LayoutTree { root };
        PaginatedLayout { tree, pages }
    }

    fn place_node(
        &mut self,
        node: &MeasuredNode,
        parent: Dot,
        child_index: usize,
        terminal_chrome_after: f32,
    ) -> LayoutNode {
        match &node.content {
            MeasuredContent::Box(b) => {
                let mut placed = match b.style.direction {
                    Direction::Vertical => {
                        self.place_vertical(b, node.width, terminal_chrome_after)
                    }
                    Direction::Horizontal => self.place_horizontal(b, node),
                };
                if let LayoutContent::Box(lb) = &mut placed.content {
                    lb.attachment = (lb.node != parent).then_some(ChildAttachment {
                        parent,
                        index: child_index,
                    });
                }
                placed
            }
            MeasuredContent::Line(l) => {
                let y = self.accumulated_y;
                let x = self.current_x;
                self.accumulated_y += node.height;
                LayoutNode {
                    rect: Rect::from_xywh(x, y, node.width, node.height),
                    content: LayoutContent::Line(LayoutLine {
                        node: l.node,
                        baseline: l.baseline,
                        ascent: l.ascent,
                        descent: l.descent,
                        cursor_ascent: l.cursor_ascent,
                        cursor_descent: l.cursor_descent,
                        glyph_runs: l.glyph_runs.clone(),
                        ruby_annotations: l.ruby_annotations.clone(),
                        empty_caret_x: l.empty_caret_x,
                        offset_range: l.offset_range.clone(),
                        tab_gaps: l.tab_gaps.clone(),
                        is_phantom: l.is_phantom,
                        content_edge_x: l.content_edge_x,
                    }),
                }
            }
            MeasuredContent::Atom(a) => {
                let y = self.accumulated_y;
                let x = self.current_x;
                self.accumulated_y += node.height;
                LayoutNode {
                    rect: Rect::from_xywh(x, y, node.width, node.height),
                    content: LayoutContent::Atom(LayoutAtom {
                        node: a.node,
                        attachment: ChildAttachment {
                            parent,
                            index: child_index,
                        },
                    }),
                }
            }
            MeasuredContent::Spacing(h) => {
                let y = self.accumulated_y;
                let x = self.current_x;
                self.accumulated_y += h;
                LayoutNode {
                    rect: Rect::from_xywh(x, y, node.width, *h),
                    content: LayoutContent::Spacing(SpacingKind::Gap {
                        position: Position::new(parent, child_index),
                    }),
                }
            }
            MeasuredContent::PageBreak => LayoutNode {
                rect: Rect::from_xywh(0.0, self.accumulated_y, 0.0, 0.0),
                content: LayoutContent::Spacing(SpacingKind::Gap {
                    position: Position::new(parent, child_index),
                }),
            },
        }
    }

    fn place_vertical(
        &mut self,
        measured: &MeasuredBox,
        width: f32,
        terminal_chrome_after: f32,
    ) -> LayoutNode {
        let box_x = self.compute_box_x(measured, width);
        let box_y = self.accumulated_y;

        let collapse = measured.style.border_mode == BorderMode::Collapse;
        let lead_border_top = if collapse {
            0.0
        } else {
            measured.style.border.top
        };
        let lead_border_left = if collapse {
            0.0
        } else {
            measured.style.border.left
        };

        self.accumulated_y += lead_border_top + measured.style.padding.top;

        let old_x = self.current_x;
        self.current_x = box_x + lead_border_left + measured.style.padding.left;

        let mut children = Vec::new();
        let mut prev_border_bottom: Option<f32> = None;

        let mut child_index: usize = 0;
        let terminal_child_index = terminal_child_index(measured);

        for (raw_child_index, child) in measured.children.iter().enumerate() {
            let is_doc_child = !matches!(child.content, MeasuredContent::Spacing(_));

            // 1. Gap absorption at page start (paginated only)
            if self.is_paginated()
                && self.is_at_page_start()
                && matches!(child.content, MeasuredContent::Spacing(_))
            {
                continue;
            }

            // 2. Border collapse
            if measured.style.border_mode == BorderMode::Collapse
                && let Some(prev_bb) = prev_border_bottom
            {
                let child_bt = child_border_top(child);
                let overlap = prev_bb.min(child_bt);
                self.accumulated_y -= overlap;
            }

            // 3. PageBreak -> forced break
            if matches!(child.content, MeasuredContent::PageBreak) {
                if self.is_paginated() && !self.is_at_page_start() {
                    self.break_page(&mut children);
                }
                prev_border_bottom = None;
                child_index += 1;
                continue; // PageBreak consumed, not added to output
            }

            let child_terminal_chrome_after = if Some(raw_child_index) == terminal_child_index {
                terminal_chrome_after + trailing_chrome_height(measured)
            } else {
                0.0
            };

            // 4. Break check
            if self.should_break_before_child(child, child_terminal_chrome_after) {
                self.break_page(&mut children);
                // Absorb gap immediately after a forced page break
                if matches!(child.content, MeasuredContent::Spacing(_)) {
                    continue;
                }
            }

            // 5. Place child
            let layout_child = self.place_node(
                child,
                measured.node,
                child_index,
                child_terminal_chrome_after,
            );
            if is_doc_child {
                child_index += 1;
            }
            children.push(layout_child);

            prev_border_bottom = child_border_bottom(child);

            // 6. Oversized child: advance pages until the child fits
            let child_bottom = self.accumulated_y;
            if self.is_paginated() {
                while child_bottom > self.page_content_bottom() {
                    self.start_new_page();
                }
                if self.accumulated_y < child_bottom {
                    self.accumulated_y = child_bottom;
                }
            }
            if !self.is_paginated() && self.accumulated_y > self.page_content_bottom() {
                self.start_new_page();
            }
        }

        self.current_x = old_x;
        let trail_border_bottom = if collapse {
            0.0
        } else {
            measured.style.border.bottom
        };
        self.accumulated_y += measured.style.padding.bottom + trail_border_bottom;
        let box_height = self.accumulated_y - box_y;

        LayoutNode {
            rect: Rect::from_xywh(box_x, box_y, width, box_height),
            content: LayoutContent::Box(LayoutBox {
                node: measured.node,
                style: measured.style.clone(),
                children,
                attachment: None,
            }),
        }
    }

    fn place_horizontal(&mut self, measured: &MeasuredBox, node: &MeasuredNode) -> LayoutNode {
        let box_x = self.compute_box_x(measured, node.width);
        let box_y = self.accumulated_y;

        let collapse = measured.style.border_mode == BorderMode::Collapse;
        let lead_border_left = if collapse {
            0.0
        } else {
            measured.style.border.left
        };
        let lead_border_top = if collapse {
            0.0
        } else {
            measured.style.border.top
        };

        let mut child_x = box_x + lead_border_left + measured.style.padding.left;
        let child_y = box_y + lead_border_top + measured.style.padding.top;

        let mut child_index: usize = 0;
        let children: Vec<LayoutNode> = measured
            .children
            .iter()
            .map(|child| {
                let is_doc_child = !matches!(child.content, MeasuredContent::Spacing(_));
                let layout_child =
                    place_node_at(child, child_x, child_y, measured.node, child_index);
                if is_doc_child {
                    child_index += 1;
                }
                child_x += child.width;
                if measured.style.border_mode == BorderMode::Collapse
                    && let MeasuredContent::Box(child_box) = &child.content
                {
                    child_x -= child_box.style.border.right;
                }
                layout_child
            })
            .collect();

        self.accumulated_y += node.height;

        LayoutNode {
            rect: Rect::from_xywh(box_x, box_y, node.width, node.height),
            content: LayoutContent::Box(LayoutBox {
                node: measured.node,
                style: measured.style.clone(),
                children,
                attachment: None,
            }),
        }
    }

    fn break_page(&mut self, children: &mut Vec<LayoutNode>) {
        let fill_height = self.remaining();
        if fill_height > 0.0 {
            children.push(LayoutNode {
                rect: Rect::from_xywh(0.0, self.accumulated_y, 0.0, fill_height),
                content: LayoutContent::Spacing(SpacingKind::Fill),
            });
            self.accumulated_y += fill_height;
        }
        self.start_new_page();
    }

    fn compute_box_x(&self, measured: &MeasuredBox, width: f32) -> f32 {
        let base_x = self.current_x;
        match measured.style.alignment {
            Alignment::Start => base_x,
            Alignment::Center => base_x + (self.content_width - width) / 2.0,
            Alignment::End => base_x + self.content_width - width,
        }
    }

    fn remaining(&self) -> f32 {
        (self.page_content_bottom - self.accumulated_y).max(0.0)
    }

    fn is_at_page_start(&self) -> bool {
        (self.accumulated_y - self.page_content_top).abs() < f32::EPSILON
    }

    fn is_paginated(&self) -> bool {
        self.paginated
    }

    fn should_break_before_child(&self, child: &MeasuredNode, terminal_chrome_after: f32) -> bool {
        if !self.is_paginated() || self.is_at_page_start() {
            return false;
        }

        let remaining = self.remaining();

        if matches!(child.content, MeasuredContent::Spacing(_)) {
            return child.height > remaining;
        }

        initial_keep_height(child, terminal_chrome_after).is_some_and(|keep| keep > remaining)
    }

    fn page_content_bottom(&self) -> f32 {
        self.page_content_bottom
    }

    fn page_width(&self) -> f32 {
        self.page_width
    }

    fn start_new_page(&mut self) {
        if self.paginated {
            let page_start = self.page_content_top - self.margins.top;
            let page_end = self.page_content_bottom + self.margins.bottom;
            self.pages.push(LayoutPage::with_content(
                page_start,
                page_end,
                self.page_content_top,
                self.page_content_bottom,
                Size::new(self.page_width(), self.page_height),
            ));
            self.page_content_top = page_end + self.margins.top;
            self.page_content_bottom = self.page_content_top + self.content_height;
            self.accumulated_y = self.page_content_top;
        } else {
            let is_first_page = self.pages.is_empty();
            let page_start = if is_first_page {
                self.page_content_top - self.margins.top
            } else {
                self.page_content_top
            };
            let page_end = self.accumulated_y;
            self.pages.push(LayoutPage::new(
                page_start,
                page_end,
                Size::new(self.page_width(), page_end - page_start),
            ));
            self.page_content_top = self.accumulated_y;
            self.page_content_bottom = self.page_content_top + self.content_height;
        }
    }

    fn finish(mut self) -> Vec<LayoutPage> {
        if self.accumulated_y > self.page_content_top {
            if self.paginated {
                let page_start = self.page_content_top - self.margins.top;
                let page_end = self.page_content_top + self.content_height + self.margins.bottom;
                self.pages.push(LayoutPage::with_content(
                    page_start,
                    page_end,
                    self.page_content_top,
                    self.page_content_top + self.content_height,
                    Size::new(self.page_width(), self.page_height),
                ));
            } else {
                let is_first_page = self.pages.is_empty();
                let page_start = if is_first_page {
                    self.page_content_top - self.margins.top
                } else {
                    self.page_content_top
                };
                let page_end = self.accumulated_y + self.margins.bottom;
                self.pages.push(LayoutPage::new(
                    page_start,
                    page_end,
                    Size::new(self.page_width(), page_end - page_start),
                ));
            }
        } else if self.pages.is_empty() {
            if self.paginated {
                self.pages.push(LayoutPage::with_content(
                    0.0,
                    self.page_height,
                    self.margins.top,
                    self.page_height - self.margins.bottom,
                    Size::new(self.page_width(), self.page_height),
                ));
            } else {
                self.pages.push(LayoutPage::new(
                    0.0,
                    self.margins.top + self.margins.bottom,
                    Size::new(self.page_width(), self.margins.top + self.margins.bottom),
                ));
            }
        } else if !self.paginated
            && let Some(page) = self.pages.last_mut()
        {
            page.y_end += self.margins.bottom;
            page.content_y_end += self.margins.bottom;
            page.size.height += self.margins.bottom;
        }
        self.pages
    }
}

fn child_border_top(node: &MeasuredNode) -> f32 {
    match &node.content {
        MeasuredContent::Box(b) => b.style.border.top,
        _ => 0.0,
    }
}

fn child_border_bottom(node: &MeasuredNode) -> Option<f32> {
    match &node.content {
        MeasuredContent::Box(b) => Some(b.style.border.bottom),
        _ => None,
    }
}

fn leading_chrome_height(b: &MeasuredBox) -> f32 {
    let border_top = if b.style.border_mode == BorderMode::Collapse {
        0.0
    } else {
        b.style.border.top
    };
    border_top + b.style.padding.top
}

fn trailing_chrome_height(b: &MeasuredBox) -> f32 {
    let border_bottom = if b.style.border_mode == BorderMode::Collapse {
        0.0
    } else {
        b.style.border.bottom
    };
    b.style.padding.bottom + border_bottom
}

fn terminal_child_index(b: &MeasuredBox) -> Option<usize> {
    let mut last = None;
    for (i, child) in b.children.iter().enumerate() {
        if !matches!(
            child.content,
            MeasuredContent::Spacing(_) | MeasuredContent::PageBreak
        ) {
            last = Some(i);
        }
    }
    last
}

fn initial_child_index(b: &MeasuredBox) -> Option<usize> {
    b.children
        .iter()
        .position(|child| !matches!(child.content, MeasuredContent::Spacing(_)))
}

fn initial_keep_height(node: &MeasuredNode, terminal_chrome_after: f32) -> Option<f32> {
    if node.page_break_policy() == PageBreakPolicy::Avoid {
        return Some(node.height + terminal_chrome_after);
    }

    let MeasuredContent::Box(b) = &node.content else {
        return None;
    };
    if b.style.direction != Direction::Vertical {
        return None;
    }

    let first_child_index = initial_child_index(b)?;
    let child_terminal_chrome_after = if Some(first_child_index) == terminal_child_index(b) {
        terminal_chrome_after + trailing_chrome_height(b)
    } else {
        0.0
    };

    Some(
        leading_chrome_height(b)
            + initial_keep_height(&b.children[first_child_index], child_terminal_chrome_after)?,
    )
}

fn place_node_at(
    node: &MeasuredNode,
    x: f32,
    y: f32,
    parent: Dot,
    child_index: usize,
) -> LayoutNode {
    match &node.content {
        MeasuredContent::Box(b) => {
            let mut offset_y = b.style.border.top + b.style.padding.top;
            let mut offset_x = b.style.border.left + b.style.padding.left;
            let children: Vec<LayoutNode> = match b.style.direction {
                Direction::Vertical => {
                    let mut idx: usize = 0;
                    b.children
                        .iter()
                        .map(|child| {
                            let is_doc_child =
                                !matches!(child.content, MeasuredContent::Spacing(_));
                            let c = place_node_at(child, x + offset_x, y + offset_y, b.node, idx);
                            if is_doc_child {
                                idx += 1;
                            }
                            offset_y += child.height;
                            c
                        })
                        .collect()
                }
                Direction::Horizontal => {
                    let mut idx: usize = 0;
                    b.children
                        .iter()
                        .map(|child| {
                            let is_doc_child =
                                !matches!(child.content, MeasuredContent::Spacing(_));
                            let c = place_node_at(child, x + offset_x, y + offset_y, b.node, idx);
                            if is_doc_child {
                                idx += 1;
                            }
                            offset_x += child.width;
                            c
                        })
                        .collect()
                }
            };
            let attachment = Some(ChildAttachment {
                parent,
                index: child_index,
            });
            LayoutNode {
                rect: Rect::from_xywh(x, y, node.width, node.height),
                content: LayoutContent::Box(LayoutBox {
                    node: b.node,
                    style: b.style.clone(),
                    children,
                    attachment,
                }),
            }
        }
        MeasuredContent::Line(l) => LayoutNode {
            rect: Rect::from_xywh(x, y, node.width, node.height),
            content: LayoutContent::Line(LayoutLine {
                node: l.node,
                baseline: l.baseline,
                ascent: l.ascent,
                descent: l.descent,
                cursor_ascent: l.cursor_ascent,
                cursor_descent: l.cursor_descent,
                glyph_runs: l.glyph_runs.clone(),
                ruby_annotations: l.ruby_annotations.clone(),
                empty_caret_x: l.empty_caret_x,
                offset_range: l.offset_range.clone(),
                tab_gaps: l.tab_gaps.clone(),
                is_phantom: l.is_phantom,
                content_edge_x: l.content_edge_x,
            }),
        },
        MeasuredContent::Atom(a) => LayoutNode {
            rect: Rect::from_xywh(x, y, node.width, node.height),
            content: LayoutContent::Atom(LayoutAtom {
                node: a.node,
                attachment: ChildAttachment {
                    parent,
                    index: child_index,
                },
            }),
        },
        MeasuredContent::Spacing(h) => LayoutNode {
            rect: Rect::from_xywh(x, y, node.width, *h),
            content: LayoutContent::Spacing(SpacingKind::Gap {
                position: Position::new(parent, child_index),
            }),
        },
        MeasuredContent::PageBreak => LayoutNode {
            rect: Rect::from_xywh(x, y, 0.0, 0.0),
            content: LayoutContent::Spacing(SpacingKind::Gap {
                position: Position::new(parent, child_index),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AtomLeaf, DocLogs, DocView, HorizontalRuleVariant, Modifier, ModifierAttrLog,
        ModifierAttrOp::SetModifier, NodeAttrLog, NodeMarkerLog, NodeStyleLog, NodeType, SeqItem,
        SpanLog, StyleLog, project_document,
    };
    use editor_resource::Resource;

    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;

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

    fn measure_doc(doc: &DocLogs, width: f32) -> (editor_crdt::Dot, MeasuredTree) {
        let pd = project_document(doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let root_id = root_node.id();
        let mut res = Resource::new_test();
        let measured = measure_node(&root_node, width, &MeasureContext::default(), &mut res);
        (root_id, MeasuredTree { root: measured })
    }

    fn paginate_continuous(doc: &DocLogs, width: f32) -> (editor_crdt::Dot, PaginatedLayout) {
        let (root_id, tree) = measure_doc(doc, width);
        let layout = Paginator::continuous(width, 100_000.0, EdgeInsets::all(0.0)).paginate(tree);
        (root_id, layout)
    }

    fn has_fill(node: &LayoutNode) -> bool {
        match &node.content {
            LayoutContent::Spacing(SpacingKind::Fill) => true,
            LayoutContent::Box(b) => b.children.iter().any(has_fill),
            _ => false,
        }
    }

    fn build_root_two_paragraphs_gap(block_gap: Option<Modifier>) -> DocLogs {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let p2 = Dot::new(1, 2);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        let mut doc = logs(&items);
        if let Some(modifier) = block_gap {
            doc.block_modifiers = ModifierAttrLog::new()
                .apply(
                    Dot::ROOT,
                    SetModifier {
                        target: root,
                        modifier,
                    },
                )
                .unwrap();
        }
        doc
    }

    #[test]
    fn root_paragraph_line() {
        let root = Dot::ROOT;
        let p = Dot::new(1, 1);
        let items = vec![
            (
                p,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('H')),
            (Dot::new(1, 3), SeqItem::Char('e')),
            (Dot::new(1, 4), SeqItem::Char('l')),
            (Dot::new(1, 5), SeqItem::Char('l')),
            (Dot::new(1, 6), SeqItem::Char('o')),
        ];
        let doc = logs(&items);
        let (root_id, layout) = paginate_continuous(&doc, 400.0);

        let LayoutContent::Box(ref root_box) = layout.tree.root.content else {
            panic!("expected root Box");
        };
        assert_eq!(root_box.node, root_id);

        let para_node = root_box
            .children
            .iter()
            .find(|n| matches!(n.content, LayoutContent::Box(_)))
            .unwrap();
        let LayoutContent::Box(ref para_box) = para_node.content else {
            panic!()
        };

        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_view = view.root().unwrap();
        let para_id = root_view
            .children()
            .find_map(|c| {
                if let editor_model::ChildView::Block(nv) = c {
                    Some(nv.id())
                } else {
                    None
                }
            })
            .unwrap();

        let line_node = para_box
            .children
            .iter()
            .find(|n| matches!(n.content, LayoutContent::Line(_)))
            .unwrap();
        let LayoutContent::Line(ref line) = line_node.content else {
            panic!()
        };
        assert_eq!(line.node, para_id);
        assert!(line_node.rect.height > 0.0);
        assert!(line.offset_range.is_some());
    }

    #[test]
    fn vertical_stacking() {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let p2 = Dot::new(1, 2);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('H')),
            (Dot::new(1, 4), SeqItem::Char('i')),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 5), SeqItem::Char('B')),
            (Dot::new(1, 6), SeqItem::Char('y')),
            (Dot::new(1, 7), SeqItem::Char('e')),
        ];
        let doc = logs(&items);
        let (root_id, layout) = paginate_continuous(&doc, 400.0);

        let LayoutContent::Box(ref root_box) = layout.tree.root.content else {
            panic!("expected root Box");
        };
        assert_eq!(root_box.node, root_id);

        let para_boxes: Vec<_> = root_box
            .children
            .iter()
            .filter(|n| matches!(n.content, LayoutContent::Box(_)))
            .collect();
        assert!(para_boxes.len() >= 2);
        assert!(para_boxes[1].rect.y > para_boxes[0].rect.y);
        assert!(
            layout.tree.root.rect.height >= para_boxes[0].rect.height + para_boxes[1].rect.height
        );
    }

    #[test]
    fn atom_attachment() {
        let root = Dot::ROOT;
        let hr = Dot::new(1, 1);
        let p = Dot::new(1, 2);
        let items = vec![
            (
                hr,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HorizontalRule {
                        variant: HorizontalRuleVariant::default(),
                    },
                    parents: vec![root],
                },
            ),
            (
                p,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let doc = logs(&items);
        let (root_id, layout) = paginate_continuous(&doc, 400.0);

        let hr_id = hr;

        let LayoutContent::Box(ref root_box) = layout.tree.root.content else {
            panic!("expected root Box");
        };

        let atom_node = root_box
            .children
            .iter()
            .find(|n| matches!(n.content, LayoutContent::Atom(_)))
            .unwrap();
        let LayoutContent::Atom(ref atom) = atom_node.content else {
            panic!()
        };
        assert_eq!(atom.node, hr_id);
        assert_eq!(
            atom.attachment,
            ChildAttachment {
                parent: root_id,
                index: 0
            }
        );
        assert_eq!(atom_node.rect.height, 24.0);
    }

    #[test]
    fn spacing_gap() {
        let doc = build_root_two_paragraphs_gap(Some(Modifier::BlockGap { value: 100 }));
        let (root_id, layout) = paginate_continuous(&doc, 300.0);

        let LayoutContent::Box(ref root_box) = layout.tree.root.content else {
            panic!("expected root Box");
        };

        let spacing = root_box
            .children
            .iter()
            .find(|n| matches!(n.content, LayoutContent::Spacing(_)))
            .unwrap();
        let LayoutContent::Spacing(ref kind) = spacing.content else {
            panic!()
        };
        let SpacingKind::Gap { position } = kind else {
            panic!("expected Gap spacing");
        };
        assert_eq!(position.node, root_id);
    }

    #[test]
    fn horizontal_row() {
        let root = Dot::ROOT;
        let table = Dot::new(1, 1);
        let row = Dot::new(1, 2);
        let cell_a = Dot::new(1, 3);
        let para_a = Dot::new(1, 4);
        let cell_b = Dot::new(1, 10);
        let para_b = Dot::new(1, 11);
        let p_root = Dot::new(1, 20);
        let items = vec![
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                },
            ),
            (
                row,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                },
            ),
            (
                cell_a,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row],
                },
            ),
            (
                para_a,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row, cell_a],
                },
            ),
            (Dot::new(1, 5), SeqItem::Char('A')),
            (
                cell_b,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row],
                },
            ),
            (
                para_b,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row, cell_b],
                },
            ),
            (Dot::new(1, 12), SeqItem::Char('B')),
            (
                p_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        let doc = logs(&items);
        let (_root_id, layout) = paginate_continuous(&doc, 400.0);

        let LayoutContent::Box(ref root_box) = layout.tree.root.content else {
            panic!("expected root Box");
        };

        let table_node = root_box
            .children
            .iter()
            .find(|n| matches!(n.content, LayoutContent::Box(_)))
            .unwrap();
        let LayoutContent::Box(ref table_box) = table_node.content else {
            panic!()
        };

        let row_node = table_box
            .children
            .iter()
            .find(|n| matches!(n.content, LayoutContent::Box(_)))
            .unwrap();
        let LayoutContent::Box(ref row_box) = row_node.content else {
            panic!()
        };
        assert_eq!(row_box.style.direction, Direction::Horizontal);

        let cell_nodes: Vec<_> = row_box
            .children
            .iter()
            .filter(|n| matches!(n.content, LayoutContent::Box(_)))
            .collect();
        assert!(cell_nodes.len() >= 2);
        assert!(cell_nodes[1].rect.x > cell_nodes[0].rect.x);
    }

    #[test]
    fn root_attachment_none() {
        let root = Dot::ROOT;
        let p = Dot::new(1, 1);
        let items = vec![
            (
                p,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('x')),
        ];
        let doc = logs(&items);
        let (_root_id, layout) = paginate_continuous(&doc, 400.0);

        let LayoutContent::Box(ref root_box) = layout.tree.root.content else {
            panic!("expected root Box");
        };
        assert_eq!(root_box.attachment, None);
    }

    #[test]
    fn forced_page_break() {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let p2 = Dot::new(1, 2);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('A')),
            (Dot::new(1, 4), SeqItem::Atom(AtomLeaf::PageBreak)),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 5), SeqItem::Char('B')),
        ];
        let doc = logs(&items);

        let (_, tree_p) = measure_doc(&doc, 400.0);
        let paginated = Paginator::paginated(400.0, 1000.0, EdgeInsets::all(10.0)).paginate(tree_p);
        assert_eq!(paginated.pages.len(), 2);
        assert!(has_fill(&paginated.tree.root));

        let (_, tree_c) = measure_doc(&doc, 400.0);
        let continuous =
            Paginator::continuous(400.0, 100_000.0, EdgeInsets::all(10.0)).paginate(tree_c);
        assert_eq!(continuous.pages.len(), 1);
        assert!(!has_fill(&continuous.tree.root));
    }
}
