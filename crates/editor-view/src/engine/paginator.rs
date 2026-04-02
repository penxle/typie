use editor_common::{Alignment, EdgeInsets, Rect, Size};
use editor_model::NodeId;

use crate::fragment::*;
use crate::measure::*;
use crate::page::Page;

type Placeholders = Vec<MeasuredPlaceholder>;

enum Mode {
    Paginated { page_height: f32 },
    Continuous,
}

struct OpenContainer {
    node_id: NodeId,
    scope: bool,
    start_y: f32,
    width: f32,
    children: Vec<Fragment>,
    pending_gap: f32,
    split_top: bool,
    padding: EdgeInsets,
    border: EdgeInsets,
    border_mode: BorderMode,
    last_child_border_end: f32,
    is_first_child: bool,
    placeholders: Placeholders,
}

pub struct Paginator {
    mode: Mode,
    width: f32,
    content_height: f32,
    margin_top: f32,
    margin_bottom: f32,
    margin_left: f32,
    current_y: f32,
    container_stack: Vec<OpenContainer>,
    page_fragments: Vec<Fragment>,
    pages: Vec<Page>,
}

fn child_border_top(measurement: &Measurement) -> f32 {
    match &measurement.content {
        MeasuredContent::Container(c) => c.border.top,
        _ => 0.0,
    }
}

fn child_border_bottom(measurement: &Measurement) -> f32 {
    match &measurement.content {
        MeasuredContent::Container(c) => c.border.bottom,
        _ => 0.0,
    }
}

impl Paginator {
    pub fn new_paginated(
        width: f32,
        page_height: f32,
        margin_top: f32,
        margin_bottom: f32,
        margin_left: f32,
    ) -> Self {
        Self {
            mode: Mode::Paginated { page_height },
            width,
            content_height: page_height - margin_top - margin_bottom,
            margin_top,
            margin_bottom,
            margin_left,
            current_y: margin_top,
            container_stack: vec![],
            page_fragments: vec![],
            pages: vec![],
        }
    }

    pub fn new_continuous(
        width: f32,
        max_content_height: f32,
        margin_top: f32,
        margin_bottom: f32,
        margin_left: f32,
    ) -> Self {
        Self {
            mode: Mode::Continuous,
            width,
            content_height: max_content_height,
            margin_top,
            margin_bottom,
            margin_left,
            current_y: margin_top,
            container_stack: vec![],
            page_fragments: vec![],
            pages: vec![],
        }
    }

    fn is_paginated(&self) -> bool {
        matches!(self.mode, Mode::Paginated { .. })
    }

    fn is_first_page(&self) -> bool {
        self.pages.is_empty()
    }

    fn content_top(&self) -> f32 {
        if self.is_paginated() {
            self.margin_top
        } else if self.is_first_page() {
            self.margin_top
        } else {
            0.0
        }
    }

    fn content_bottom(&self) -> f32 {
        self.content_top() + self.content_height
    }

    fn current_x(&self) -> f32 {
        self.margin_left
            + self
                .container_stack
                .iter()
                .map(|c| c.border.left + c.padding.left)
                .sum::<f32>()
    }

    fn child_x(&self, child_measurement: &Measurement) -> f32 {
        let base_x = self.current_x();
        let container_content_width = self.container_stack.last().map_or(self.width, |c| {
            c.width - c.border.left - c.padding.left - c.padding.right - c.border.right
        });

        match child_measurement.alignment {
            Alignment::Start => base_x,
            Alignment::Center => {
                base_x + (container_content_width - child_measurement.size.width) / 2.0
            }
            Alignment::End => base_x + container_content_width - child_measurement.size.width,
        }
    }

    fn remaining(&self) -> f32 {
        (self.content_bottom() - self.current_y).max(0.0)
    }

    fn is_page_empty(&self) -> bool {
        self.page_fragments.is_empty() && self.container_stack.iter().all(|c| c.children.is_empty())
    }

    fn container_height_on_break(&self, start_y: f32) -> f32 {
        if self.is_paginated() {
            self.content_bottom() - start_y
        } else {
            self.current_y - start_y
        }
    }

    fn container_breaks_on_break(&self, split_top: bool) -> Breaks {
        if self.is_paginated() {
            Breaks {
                top: split_top,
                bottom: true,
            }
        } else {
            Breaks::default()
        }
    }

    fn page_height_on_break(&self) -> f32 {
        match self.mode {
            Mode::Paginated { page_height } => page_height,
            Mode::Continuous => self.current_y,
        }
    }

    fn final_page_height(&self) -> f32 {
        match self.mode {
            Mode::Paginated { page_height } => page_height,
            Mode::Continuous => self.current_y + self.margin_bottom,
        }
    }

    fn empty_page_height(&self) -> f32 {
        match self.mode {
            Mode::Paginated { page_height } => page_height,
            Mode::Continuous => self.margin_top + self.margin_bottom,
        }
    }

    fn gap_after_break(&self, gap: f32) -> f32 {
        if self.is_paginated() {
            let margin_spacing = self.margin_top + self.margin_bottom;
            if gap > margin_spacing {
                gap - margin_spacing
            } else {
                0.0
            }
        } else {
            gap
        }
    }

    pub fn place(&mut self, node_id: NodeId, measurement: &Measurement) {
        match &measurement.content {
            MeasuredContent::Container(ContainerContent {
                children,
                scope,
                direction,
                padding,
                border,
                border_mode,
                placeholders,
            }) => match direction {
                LayoutDirection::Vertical if children.is_empty() => {
                    if measurement.size.height > self.remaining() && !self.is_page_empty() {
                        self.break_page();
                    }

                    let fragment = Fragment::Container(ContainerFragment {
                        node_id,
                        rect: Rect {
                            x: self.child_x(measurement),
                            y: self.current_y,
                            width: self.width,
                            height: measurement.size.height,
                        },
                        children: vec![],
                        scope: *scope,
                        breaks: Breaks::default(),
                        border: EdgeInsets::ZERO,
                    });

                    self.add_to_current(fragment);
                    self.current_y += measurement.size.height;
                }
                LayoutDirection::Vertical => {
                    self.open_container(
                        node_id,
                        *scope,
                        *padding,
                        *border,
                        *border_mode,
                        placeholders.clone(),
                    );

                    for child in children {
                        if let Some(top) = self.container_stack.last_mut() {
                            if top.border_mode == BorderMode::Collapse {
                                let child_bt = child_border_top(&child.measurement);
                                if top.is_first_child {
                                    let overlap = top.border.top.min(child_bt);
                                    self.current_y -= overlap;
                                    top.is_first_child = false;
                                } else {
                                    let overlap = top.last_child_border_end.min(child_bt);
                                    self.current_y -= overlap;
                                }
                            }
                        }

                        self.apply_gap_or_break(&child.measurement);
                        self.place(child.node_id, &child.measurement);

                        if let Some(top) = self.container_stack.last_mut() {
                            top.pending_gap = child.measurement.gap_after;
                            if top.border_mode == BorderMode::Collapse {
                                top.last_child_border_end = child_border_bottom(&child.measurement);
                            }
                        }
                    }

                    self.close_container();
                }
                LayoutDirection::Horizontal => {
                    self.place_horizontal(
                        node_id,
                        measurement,
                        children,
                        *scope,
                        *border,
                        *border_mode,
                    );
                }
            },
            MeasuredContent::TextBlock { lines } => {
                self.open_container(
                    node_id,
                    false,
                    EdgeInsets::ZERO,
                    EdgeInsets::ZERO,
                    BorderMode::Separate,
                    vec![],
                );

                for line in lines {
                    self.place_line(node_id, line);
                }

                self.close_container();
            }
            MeasuredContent::Atom { parent_id, index } => {
                if measurement.size.height > self.remaining() && !self.is_page_empty() {
                    self.break_page();
                }

                let fragment = Fragment::Atom(AtomFragment {
                    node_id,
                    parent_id: *parent_id,
                    index: *index,
                    rect: Rect {
                        x: self.child_x(measurement),
                        y: self.current_y,
                        width: measurement.size.width,
                        height: measurement.size.height,
                    },
                });

                self.add_to_current(fragment);
                self.current_y += measurement.size.height;
            }
            MeasuredContent::PageBreak => {
                if !self.is_page_empty() {
                    self.break_page();
                }
            }
        }
    }

    fn place_horizontal(
        &mut self,
        node_id: NodeId,
        measurement: &Measurement,
        children: &[ChildMeasurement],
        scope: bool,
        border: EdgeInsets,
        border_mode: BorderMode,
    ) {
        if measurement.size.height > self.remaining() && !self.is_page_empty() {
            self.break_page();
        }

        let mut child_x = match border_mode {
            BorderMode::Collapse => border.left,
            BorderMode::Separate => 0.0,
        };

        let child_frags: Vec<Fragment> = children
            .iter()
            .enumerate()
            .map(|(i, child)| {
                if border_mode == BorderMode::Collapse {
                    let child_bl = match &child.measurement.content {
                        MeasuredContent::Container(c) => c.border.left,
                        _ => 0.0,
                    };
                    if i == 0 {
                        child_x -= border.left.min(child_bl);
                    } else {
                        let prev_br = match &children[i - 1].measurement.content {
                            MeasuredContent::Container(c) => c.border.right,
                            _ => 0.0,
                        };
                        child_x -= prev_br.min(child_bl);
                    }
                }

                let frag = self.position_subtree(
                    &child.measurement,
                    child.node_id,
                    child_x,
                    self.current_y,
                );
                child_x += child.measurement.size.width;
                frag
            })
            .collect();

        let fragment = Fragment::Container(ContainerFragment {
            node_id,
            rect: Rect {
                x: self.margin_left,
                y: self.current_y,
                width: measurement.size.width,
                height: measurement.size.height,
            },
            children: child_frags,
            scope,
            breaks: Breaks::default(),
            border: EdgeInsets::ZERO,
        });

        self.add_to_current(fragment);
        self.current_y += measurement.size.height;
    }

    fn place_line(&mut self, node_id: NodeId, line: &MeasuredLine) {
        if line.height > self.remaining() && !self.is_page_empty() {
            self.break_page();
        }

        let fragment = Fragment::Line(LineFragment {
            node_id,
            rect: Rect {
                x: self.current_x(),
                y: self.current_y,
                width: self.width,
                height: line.height,
            },
            baseline: line.baseline,
            glyph_runs: line.glyph_runs.clone(),
        });

        self.add_to_current(fragment);
        self.current_y += line.height;
    }

    fn apply_gap_or_break(&mut self, child_measurement: &Measurement) {
        let gap = self.container_stack.last().map_or(0.0, |c| c.pending_gap);
        if gap + child_measurement.size.height > self.remaining() && !self.is_page_empty() {
            let carry = self.gap_after_break(gap);
            self.break_page();
            self.current_y += carry;
        }

        let gap = self.container_stack.last().map_or(0.0, |c| c.pending_gap);
        self.current_y += gap;
    }

    fn position_subtree(
        &self,
        measurement: &Measurement,
        node_id: NodeId,
        x: f32,
        y: f32,
    ) -> Fragment {
        match &measurement.content {
            MeasuredContent::Container(content) => {
                let mut offset = 0.0;

                let mut child_frags: Vec<Fragment> = content
                    .children
                    .iter()
                    .map(|child| {
                        let (cx, cy) = match content.direction {
                            LayoutDirection::Vertical => (x, y + offset),
                            LayoutDirection::Horizontal => (x + offset, y),
                        };

                        offset += match content.direction {
                            LayoutDirection::Vertical => child.measurement.size.height,
                            LayoutDirection::Horizontal => child.measurement.size.width,
                        };

                        self.position_subtree(&child.measurement, child.node_id, cx, cy)
                    })
                    .collect();

                for ph in &content.placeholders {
                    child_frags.push(Fragment::Placeholder(PlaceholderFragment {
                        id: ph.id,
                        rect: Rect {
                            x: x + ph.rect.x,
                            y: y + ph.rect.y,
                            width: ph.rect.width,
                            height: ph.rect.height,
                        },
                        data: ph.data.clone(),
                    }));
                }

                Fragment::Container(ContainerFragment {
                    node_id,
                    rect: Rect {
                        x,
                        y,
                        width: measurement.size.width,
                        height: measurement.size.height,
                    },
                    children: child_frags,
                    scope: content.scope,
                    breaks: Breaks::default(),
                    border: content.border,
                })
            }
            MeasuredContent::TextBlock { lines } => {
                let mut line_y = y;

                let line_frags: Vec<Fragment> = lines
                    .iter()
                    .map(|line| {
                        let frag = Fragment::Line(LineFragment {
                            node_id,
                            rect: Rect {
                                x,
                                y: line_y,
                                width: measurement.size.width,
                                height: line.height,
                            },
                            baseline: line.baseline,
                            glyph_runs: line.glyph_runs.clone(),
                        });
                        line_y += line.height;
                        frag
                    })
                    .collect();

                Fragment::Container(ContainerFragment {
                    node_id,
                    rect: Rect {
                        x,
                        y,
                        width: measurement.size.width,
                        height: measurement.size.height,
                    },
                    children: line_frags,
                    scope: false,
                    breaks: Breaks::default(),
                    border: EdgeInsets::ZERO,
                })
            }
            MeasuredContent::Atom { parent_id, index } => Fragment::Atom(AtomFragment {
                node_id,
                parent_id: *parent_id,
                index: *index,
                rect: Rect {
                    x,
                    y,
                    width: measurement.size.width,
                    height: measurement.size.height,
                },
            }),
            MeasuredContent::PageBreak => Fragment::Container(ContainerFragment {
                node_id,
                rect: Rect {
                    x,
                    y,
                    width: 0.0,
                    height: 0.0,
                },
                children: vec![],
                scope: false,
                breaks: Breaks::default(),
                border: EdgeInsets::ZERO,
            }),
        }
    }

    fn open_container(
        &mut self,
        node_id: NodeId,
        scope: bool,
        padding: EdgeInsets,
        border: EdgeInsets,
        border_mode: BorderMode,
        placeholders: Placeholders,
    ) {
        match border_mode {
            BorderMode::Separate => {
                self.current_y += border.top + padding.top;
            }
            BorderMode::Collapse => {
                self.current_y += border.top;
            }
        }
        let start_y = match border_mode {
            BorderMode::Separate => self.current_y - padding.top - border.top,
            BorderMode::Collapse => self.current_y - border.top,
        };
        self.container_stack.push(OpenContainer {
            node_id,
            scope,
            start_y,
            width: self.width,
            children: vec![],
            pending_gap: 0.0,
            split_top: false,
            padding,
            border,
            border_mode,
            last_child_border_end: 0.0,
            is_first_child: true,
            placeholders,
        });
    }

    fn close_container(&mut self) {
        let mut c = self.container_stack.pop().expect("close without open");
        match c.border_mode {
            BorderMode::Separate => {
                self.current_y += c.padding.bottom + c.border.bottom;
            }
            BorderMode::Collapse => {
                let extra = (c.border.bottom - c.last_child_border_end).max(0.0);
                self.current_y += extra;
            }
        }
        let container_x = self.current_x();
        let container_y = c.start_y;
        for ph in &c.placeholders {
            c.children.push(Fragment::Placeholder(PlaceholderFragment {
                id: ph.id,
                rect: Rect {
                    x: container_x + ph.rect.x,
                    y: container_y + ph.rect.y,
                    width: ph.rect.width,
                    height: ph.rect.height,
                },
                data: ph.data.clone(),
            }));
        }
        let fragment = Fragment::Container(ContainerFragment {
            node_id: c.node_id,
            rect: Rect {
                x: container_x,
                y: container_y,
                width: c.width,
                height: self.current_y - container_y,
            },
            children: c.children,
            scope: c.scope,
            breaks: Breaks {
                top: c.split_top,
                bottom: false,
            },
            border: c.border,
        });
        self.add_to_current(fragment);
    }

    fn add_to_current(&mut self, fragment: Fragment) {
        if let Some(top) = self.container_stack.last_mut() {
            top.children.push(fragment);
        } else {
            self.page_fragments.push(fragment);
        }
    }

    fn break_page(&mut self) {
        let mut reopens: Vec<(NodeId, bool, f32, EdgeInsets, EdgeInsets, BorderMode)> = vec![];

        while let Some(c) = self.container_stack.pop() {
            reopens.push((
                c.node_id,
                c.scope,
                c.width,
                c.padding,
                c.border,
                c.border_mode,
            ));
            if c.children.is_empty() {
                continue;
            }

            match c.border_mode {
                BorderMode::Separate => {
                    self.current_y += c.border.bottom;
                }
                BorderMode::Collapse => {
                    let extra = (c.border.bottom - c.last_child_border_end).max(0.0);
                    self.current_y += extra;
                }
            }

            let container_x = self.current_x();
            let container_y = c.start_y;
            let mut children = c.children;
            for ph in &c.placeholders {
                children.push(Fragment::Placeholder(PlaceholderFragment {
                    id: ph.id,
                    rect: Rect {
                        x: container_x + ph.rect.x,
                        y: container_y + ph.rect.y,
                        width: ph.rect.width,
                        height: ph.rect.height,
                    },
                    data: ph.data.clone(),
                }));
            }

            let fragment = Fragment::Container(ContainerFragment {
                node_id: c.node_id,
                rect: Rect {
                    x: container_x,
                    y: container_y,
                    width: c.width,
                    height: self.container_height_on_break(c.start_y),
                },
                children,
                scope: c.scope,
                breaks: self.container_breaks_on_break(c.split_top),
                border: c.border,
            });

            self.add_to_current(fragment);
        }

        let frags = std::mem::take(&mut self.page_fragments);
        if !frags.is_empty() {
            self.pages.push(Page::new(
                Size::new(self.width, self.page_height_on_break()),
                frags,
            ));
        }

        self.current_y = self.content_top();

        for (node_id, scope, width, padding, border, border_mode) in reopens.into_iter().rev() {
            let start_offset = match border_mode {
                BorderMode::Separate => border.top + padding.top,
                BorderMode::Collapse => border.top,
            };
            self.current_y += start_offset;
            self.container_stack.push(OpenContainer {
                node_id,
                scope,
                start_y: self.current_y - start_offset,
                width,
                children: vec![],
                pending_gap: 0.0,
                split_top: true,
                padding,
                border,
                border_mode,
                last_child_border_end: 0.0,
                is_first_child: true,
                placeholders: vec![],
            });
        }
    }

    pub fn finish(mut self) -> Vec<Page> {
        while !self.container_stack.is_empty() {
            self.close_container();
        }

        if !self.page_fragments.is_empty() {
            let frags = std::mem::take(&mut self.page_fragments);
            self.pages.push(Page::new(
                Size::new(self.width, self.final_page_height()),
                frags,
            ));
        }

        if self.pages.is_empty() {
            self.pages.push(Page::new(
                Size::new(self.width, self.empty_page_height()),
                vec![],
            ));
        }

        self.pages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::Size;
    use std::sync::Arc;

    fn container_m(height: f32, children: Vec<ChildMeasurement>) -> Arc<Measurement> {
        Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children,
                ..Default::default()
            }),
        })
    }

    fn leaf_container_m(height: f32) -> Arc<Measurement> {
        container_m(height, vec![])
    }

    fn child(height: f32) -> ChildMeasurement {
        ChildMeasurement {
            node_id: NodeId::new(),
            measurement: leaf_container_m(height),
        }
    }

    fn page_break_m() -> Arc<Measurement> {
        Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 0.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::PageBreak,
        })
    }

    fn text_block_m(line_heights: &[f32]) -> Arc<Measurement> {
        let lines: Vec<MeasuredLine> = line_heights
            .iter()
            .map(|&h| MeasuredLine {
                height: h,
                baseline: h * 0.8,
                glyph_runs: vec![],
            })
            .collect();

        let height: f32 = line_heights.iter().sum();

        Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::TextBlock { lines },
        })
    }

    #[test]
    fn continuous_single_page() {
        let mut p = Paginator::new_continuous(200.0, 1024.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &leaf_container_m(100.0));
        p.place(NodeId::new(), &leaf_container_m(100.0));
        let pages = p.finish();

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].fragments.len(), 2);
        assert_eq!(pages[0].size.height, 200.0);
        assert_eq!(pages[0].fragments[0].rect().y, 0.0);
        assert_eq!(pages[0].fragments[1].rect().y, 100.0);
    }

    #[test]
    fn paginated_splits_text_block_at_line_boundary() {
        // 4 lines of 20px, page_height=50: 2 lines per page
        let mut p = Paginator::new_paginated(200.0, 50.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &text_block_m(&[20.0, 20.0, 20.0, 20.0]));
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].size.height, 50.0);
        assert_eq!(pages[1].size.height, 50.0);

        let p1 = match &pages[0].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(p1.children.len(), 2);
        assert_eq!(p1.rect.height, 50.0);
        assert!(matches!(&p1.children[0], Fragment::Line(_)));
        assert!(matches!(&p1.children[1], Fragment::Line(_)));

        let p2 = match &pages[1].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(p2.children.len(), 2);
        assert_eq!(p2.children[0].rect().y, 0.0);
        assert_eq!(p2.children[1].rect().y, 20.0);
    }

    #[test]
    fn nested_container_preserves_hierarchy_across_pages() {
        // Fold > FoldContent > 3 children of 100px, page_height=250: first two on page 1, last on page 2
        let fold_content = container_m(300.0, vec![child(100.0), child(100.0), child(100.0)]);
        let fold = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 300.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![ChildMeasurement {
                    node_id: NodeId::new(),
                    measurement: fold_content,
                }],
                ..Default::default()
            }),
        });

        let mut p = Paginator::new_paginated(200.0, 250.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &fold);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].size.height, 250.0);
        assert_eq!(pages[1].size.height, 250.0);

        let page1_fold = match &pages[0].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(page1_fold.rect.height, 250.0);
        let page1_content = match &page1_fold.children[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(page1_content.children.len(), 2);
        assert_eq!(page1_content.rect.height, 250.0);

        let page2_fold = match &pages[1].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        let page2_content = match &page2_fold.children[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(page2_content.children.len(), 1);
        assert_eq!(page2_content.children[0].rect().y, 0.0);
        assert_eq!(page2_content.children[0].rect().height, 100.0);
        assert_eq!(page2_content.rect.height, 100.0);
    }

    #[test]
    fn atom_not_split() {
        // Atom larger than page_height is not split; it is placed as-is on the page
        let atom = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 100.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Atom {
                parent_id: NodeId::ROOT,
                index: 0,
            },
        });
        let mut p = Paginator::new_paginated(200.0, 80.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &atom);
        let pages = p.finish();

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].size.height, 80.0);
        let a = match &pages[0].fragments[0] {
            Fragment::Atom(a) => a,
            _ => panic!("expected Atom"),
        };
        assert_eq!(a.rect.y, 0.0);
        assert_eq!(a.rect.height, 100.0);
    }

    #[test]
    fn page_break_forces_new_page() {
        let mut p = Paginator::new_paginated(200.0, 200.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &leaf_container_m(50.0));
        p.place(NodeId::new(), &page_break_m());
        p.place(NodeId::new(), &leaf_container_m(50.0));
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].fragments.len(), 1);
        assert_eq!(pages[0].size.height, 200.0);
        assert_eq!(pages[1].fragments.len(), 1);
        assert_eq!(pages[1].size.height, 200.0);
        assert_eq!(pages[1].fragments[0].rect().y, 0.0);
    }

    #[test]
    fn page_break_on_empty_page_is_noop() {
        let mut p = Paginator::new_paginated(200.0, 200.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &page_break_m());
        p.place(NodeId::new(), &leaf_container_m(50.0));
        let pages = p.finish();

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].size.height, 200.0);
        assert_eq!(pages[0].fragments[0].rect().y, 0.0);
    }

    #[test]
    fn margins_reduce_content_area() {
        // Two 50px children, page_height=100, margins=10/10: content area is 80px so they split across pages
        let wrapper = container_m(100.0, vec![child(50.0), child(50.0)]);
        let mut p = Paginator::new_paginated(200.0, 100.0, 10.0, 10.0, 0.0);
        p.place(NodeId::new(), &wrapper);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        let outer1 = match &pages[0].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(outer1.rect.y, 10.0);
        let inner1 = match &outer1.children[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(inner1.rect.y, 10.0);

        let outer2 = match &pages[1].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(outer2.rect.y, 10.0);
    }

    #[test]
    fn breaks_set_on_page_break() {
        // 3 children of 100px, page_height=250: split across 2 pages; break flags indicate continuation
        let fold_content = container_m(300.0, vec![child(100.0), child(100.0), child(100.0)]);
        let fold = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 300.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![ChildMeasurement {
                    node_id: NodeId::new(),
                    measurement: fold_content,
                }],
                ..Default::default()
            }),
        });

        let mut p = Paginator::new_paginated(200.0, 250.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &fold);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);

        let page1_outer = match &pages[0].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert!(!page1_outer.breaks.top);
        assert!(page1_outer.breaks.bottom);

        let page2_outer = match &pages[1].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert!(page2_outer.breaks.top);
        assert!(!page2_outer.breaks.bottom);
    }

    #[test]
    fn wrapper_extension_fills_page_bottom() {
        // 3 children of 100px, page_height=250: split container on page 1 extends to fill the page
        let fold_content = container_m(300.0, vec![child(100.0), child(100.0), child(100.0)]);
        let fold = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 300.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![ChildMeasurement {
                    node_id: NodeId::new(),
                    measurement: fold_content,
                }],
                ..Default::default()
            }),
        });

        let mut p = Paginator::new_paginated(200.0, 250.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &fold);
        let pages = p.finish();

        let page1_outer = match &pages[0].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(page1_outer.rect.y, 0.0);
        assert_eq!(page1_outer.rect.height, 250.0);

        let page1_inner = match &page1_outer.children[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(page1_inner.rect.y, 0.0);
        assert_eq!(page1_inner.rect.height, 250.0);
    }

    #[test]
    fn margins_with_wrapper_extension() {
        // Two 50px children, page_height=100, margins=10/10: wrapper on page 1 extends to fill the content area
        let content = container_m(100.0, vec![child(50.0), child(50.0)]);
        let wrapper = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 100.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![ChildMeasurement {
                    node_id: NodeId::new(),
                    measurement: content,
                }],
                ..Default::default()
            }),
        });

        let mut p = Paginator::new_paginated(200.0, 100.0, 10.0, 10.0, 0.0);
        p.place(NodeId::new(), &wrapper);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);

        let page1_outer = match &pages[0].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(page1_outer.rect.y, 10.0);
        assert_eq!(page1_outer.rect.height, 80.0);

        let page2_outer = match &pages[1].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(page2_outer.rect.y, 10.0);
        assert_eq!(pages[1].size.height, 100.0);
        let page2_inner = match &page2_outer.children[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        let page2_child = match &page2_inner.children[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(page2_child.rect.y, 10.0);
        assert_eq!(page2_child.rect.height, 50.0);
    }

    fn leaf_with_gap(height: f32, gap: f32) -> Arc<Measurement> {
        Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height,
            },
            gap_after: gap,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![],
                ..Default::default()
            }),
        })
    }

    fn child_with_gap(height: f32, gap: f32) -> ChildMeasurement {
        ChildMeasurement {
            node_id: NodeId::new(),
            measurement: leaf_with_gap(height, gap),
        }
    }

    #[test]
    fn gap_collapsing_discards_small_gap() {
        // Gap smaller than margin spacing is discarded on page break; the next child starts at margin_top
        let parent = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 95.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![child_with_gap(40.0, 5.0), child_with_gap(50.0, 0.0)],
                ..Default::default()
            }),
        });
        let mut p = Paginator::new_paginated(200.0, 100.0, 10.0, 10.0, 0.0);
        p.place(NodeId::new(), &parent);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        let page2_outer = match &pages[1].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        let page2_child = match &page2_outer.children[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(page2_child.rect.y, 10.0);
    }

    #[test]
    fn gap_collapsing_preserves_large_gap() {
        // Gap larger than margin spacing is preserved on page break as an offset from margin_top
        let parent = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 110.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![child_with_gap(30.0, 50.0), child_with_gap(30.0, 0.0)],
                ..Default::default()
            }),
        });
        let mut p = Paginator::new_paginated(200.0, 100.0, 10.0, 10.0, 0.0);
        p.place(NodeId::new(), &parent);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        let page2_outer = match &pages[1].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        let page2_child = match &page2_outer.children[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(page2_child.rect.y, 40.0);
    }

    #[test]
    fn margin_left_offsets_x() {
        let mut p = Paginator::new_paginated(200.0, 500.0, 0.0, 0.0, 20.0);
        p.place(NodeId::new(), &leaf_container_m(50.0));
        let pages = p.finish();

        let frag = &pages[0].fragments[0];
        assert_eq!(frag.rect().x, 20.0);
    }

    #[test]
    fn empty_document_produces_one_page() {
        let p = Paginator::new_paginated(200.0, 500.0, 0.0, 0.0, 0.0);
        let pages = p.finish();

        assert_eq!(pages.len(), 1);
        assert!(pages[0].fragments.is_empty());
        assert_eq!(pages[0].size.height, 500.0);
    }

    #[test]
    fn empty_continuous_produces_one_page() {
        let p = Paginator::new_continuous(200.0, 1024.0, 0.0, 0.0, 0.0);
        let pages = p.finish();

        assert_eq!(pages.len(), 1);
        assert!(pages[0].fragments.is_empty());
    }

    #[test]
    fn continuous_splits_at_tile_boundary() {
        // Two 600px children, tile_height=1024, margins=10/10: first child fits, second overflows to a new tile
        let mut p = Paginator::new_continuous(200.0, 1024.0, 10.0, 10.0, 0.0);
        let parent = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 1200.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![child_with_gap(600.0, 0.0), child_with_gap(600.0, 0.0)],
                ..Default::default()
            }),
        });
        p.place(NodeId::new(), &parent);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].size.height, 610.0);
        assert_eq!(pages[0].fragments[0].rect().y, 10.0);
        assert_eq!(pages[1].fragments[0].rect().y, 0.0);
        assert_eq!(pages[1].size.height, 600.0 + 10.0);
    }

    #[test]
    fn continuous_page_height_is_actual_content() {
        let mut p = Paginator::new_continuous(200.0, 1024.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &leaf_container_m(300.0));
        let pages = p.finish();

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].size.height, 300.0);
    }

    #[test]
    fn continuous_preserves_hierarchy_across_tiles() {
        let fold = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 1200.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![child_with_gap(600.0, 0.0), child_with_gap(600.0, 0.0)],
                ..Default::default()
            }),
        });
        let mut p = Paginator::new_continuous(200.0, 1024.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &fold);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        let p1 = match &pages[0].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(p1.children.len(), 1);
        assert_eq!(p1.rect.height, 600.0);
        let p2 = match &pages[1].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(p2.children.len(), 1);
        assert_eq!(p2.rect.height, 600.0);
    }

    #[test]
    fn continuous_no_breaks_on_split() {
        let fold = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 1200.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![child_with_gap(600.0, 0.0), child_with_gap(600.0, 0.0)],
                ..Default::default()
            }),
        });
        let mut p = Paginator::new_continuous(200.0, 1024.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &fold);
        let pages = p.finish();

        let page1_container = match &pages[0].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert!(!page1_container.breaks.top);
        assert!(!page1_container.breaks.bottom);
    }

    #[test]
    fn continuous_with_margins() {
        let mut p = Paginator::new_continuous(200.0, 1024.0, 10.0, 10.0, 20.0);
        p.place(NodeId::new(), &leaf_container_m(50.0));
        let pages = p.finish();

        let frag = &pages[0].fragments[0];
        assert_eq!(frag.rect().x, 20.0);
        assert_eq!(frag.rect().y, 10.0);
    }

    #[test]
    fn continuous_preserves_gap_across_tiles() {
        // In continuous mode, the full gap is preserved across a tile break (not collapsed)
        let parent = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 175.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![child_with_gap(80.0, 15.0), child_with_gap(80.0, 0.0)],
                ..Default::default()
            }),
        });
        let mut p = Paginator::new_continuous(200.0, 100.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &parent);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        let page2_outer = match &pages[1].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        let page2_child = match &page2_outer.children[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(page2_child.rect.y, 15.0);
    }

    #[test]
    fn text_block_fits_on_single_page() {
        let mut p = Paginator::new_paginated(200.0, 200.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &text_block_m(&[20.0, 20.0, 20.0]));
        let pages = p.finish();

        assert_eq!(pages.len(), 1);
        let container = match &pages[0].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(container.children.len(), 3);
        assert!(matches!(&container.children[0], Fragment::Line(_)));
    }

    #[test]
    fn atom_fits_on_current_page() {
        let atom = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 50.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Atom {
                parent_id: NodeId::ROOT,
                index: 0,
            },
        });
        let mut p = Paginator::new_paginated(200.0, 200.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &atom);
        let pages = p.finish();

        assert_eq!(pages.len(), 1);
        assert!(matches!(&pages[0].fragments[0], Fragment::Atom(_)));
    }

    #[test]
    fn atom_moves_to_next_page_when_no_room() {
        let mut p = Paginator::new_paginated(200.0, 100.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &leaf_container_m(80.0));
        let atom = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 50.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Atom {
                parent_id: NodeId::ROOT,
                index: 1,
            },
        });
        p.place(NodeId::new(), &atom);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        assert!(matches!(&pages[1].fragments[0], Fragment::Atom(_)));
    }

    #[test]
    fn horizontal_container_not_split() {
        // A horizontal container (e.g. table row) is not split; it moves intact to the next page
        let cell = Arc::new(Measurement {
            size: Size {
                width: 100.0,
                height: 80.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                scope: true,
                ..Default::default()
            }),
        });
        let row = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 80.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![
                    ChildMeasurement {
                        node_id: NodeId::new(),
                        measurement: cell.clone(),
                    },
                    ChildMeasurement {
                        node_id: NodeId::new(),
                        measurement: cell,
                    },
                ],
                direction: LayoutDirection::Horizontal,
                ..Default::default()
            }),
        });

        let mut p = Paginator::new_paginated(200.0, 100.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &leaf_container_m(50.0));
        p.place(NodeId::new(), &row);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].size.height, 100.0);
        assert_eq!(pages[1].size.height, 100.0);
        let row_frag = match &pages[1].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(row_frag.children.len(), 2);
        assert_eq!(row_frag.children[0].rect().x, 0.0);
        assert_eq!(row_frag.children[0].rect().width, 100.0);
        assert_eq!(row_frag.children[1].rect().x, 100.0);
        assert_eq!(row_frag.children[1].rect().width, 100.0);
    }

    #[test]
    fn three_page_split_middle_fragment_breaks() {
        // 3 children of 100px, page_height=100: the middle page fragment has both break flags set
        let parent = container_m(300.0, vec![child(100.0), child(100.0), child(100.0)]);
        let mut p = Paginator::new_paginated(200.0, 100.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &parent);
        let pages = p.finish();

        assert_eq!(pages.len(), 3);

        let p1 = match &pages[0].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert!(!p1.breaks.top);
        assert!(p1.breaks.bottom);

        let p2 = match &pages[1].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert!(p2.breaks.top);
        assert!(p2.breaks.bottom);

        let p3 = match &pages[2].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert!(p3.breaks.top);
        assert!(!p3.breaks.bottom);
    }

    #[test]
    fn gap_applied_within_page() {
        let parent = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 70.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![child_with_gap(30.0, 10.0), child_with_gap(30.0, 0.0)],
                ..Default::default()
            }),
        });
        let mut p = Paginator::new_paginated(200.0, 200.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &parent);
        let pages = p.finish();

        assert_eq!(pages.len(), 1);
        let container = match &pages[0].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        let child1 = match &container.children[1] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(child1.rect.y, 40.0);
    }

    #[test]
    fn continuous_no_wrapper_extension() {
        // In continuous mode, split containers are not extended to fill the tile height
        let parent = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 1200.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![child_with_gap(600.0, 0.0), child_with_gap(600.0, 0.0)],
                ..Default::default()
            }),
        });
        let mut p = Paginator::new_continuous(200.0, 1024.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &parent);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        let page1_outer = match &pages[0].fragments[0] {
            Fragment::Container(c) => c,
            _ => panic!("expected Container"),
        };
        assert_eq!(page1_outer.rect.height, 600.0);
    }

    #[test]
    fn paginated_last_page_has_full_height() {
        let mut p = Paginator::new_paginated(200.0, 500.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &leaf_container_m(100.0));
        let pages = p.finish();

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].size.height, 500.0);
    }

    #[test]
    fn continuous_last_page_has_content_height() {
        let mut p = Paginator::new_continuous(200.0, 1024.0, 10.0, 10.0, 0.0);
        p.place(NodeId::new(), &leaf_container_m(100.0));
        let pages = p.finish();

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].size.height, 120.0);
    }

    #[test]
    fn no_empty_container_on_page_break() {
        // When a container's first child immediately triggers a break, no empty container is left on the previous page
        let parent = container_m(200.0, vec![child(200.0)]);
        let mut p = Paginator::new_paginated(200.0, 100.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &leaf_container_m(80.0));
        p.place(NodeId::new(), &parent);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].fragments.len(), 1);
    }

    #[test]
    fn no_empty_nested_containers_on_page_break() {
        // When a deeply nested container starts right after a page break, no empty wrappers are left on the previous page
        let paragraph = container_m(200.0, vec![child(200.0)]);
        let fold_content = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 200.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![ChildMeasurement {
                    node_id: NodeId::new(),
                    measurement: paragraph,
                }],
                ..Default::default()
            }),
        });
        let fold = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 200.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![ChildMeasurement {
                    node_id: NodeId::new(),
                    measurement: fold_content,
                }],
                ..Default::default()
            }),
        });

        let mut p = Paginator::new_paginated(200.0, 100.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &leaf_container_m(90.0));
        p.place(NodeId::new(), &fold);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].fragments.len(), 1);
    }

    #[test]
    fn container_padding_offsets_children() {
        let mut p = Paginator::new_continuous(200.0, 1024.0, 0.0, 0.0, 0.0);

        let inner_child = ChildMeasurement {
            node_id: NodeId::new(),
            measurement: leaf_container_m(30.0),
        };
        let padding = EdgeInsets {
            top: 10.0,
            left: 20.0,
            bottom: 10.0,
            right: 0.0,
        };
        let outer = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 50.0,
            },
            gap_after: 0.0,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![inner_child],
                padding,
                ..Default::default()
            }),
            alignment: Alignment::Start,
        });
        let outer_id = NodeId::new();

        p.place(outer_id, &outer);
        let pages = p.finish();

        let Fragment::Container(container) = &pages[0].fragments[0] else {
            panic!()
        };
        // Container should include full height with padding
        let Fragment::Container(child) = &container.children[0] else {
            panic!()
        };
        assert_eq!(child.rect.y, 10.0, "child y should start at padding.top");
        assert_eq!(child.rect.x, 20.0, "child x should start at padding.left");
    }

    #[test]
    fn alignment_end_positions_child_at_right() {
        let mut p = Paginator::new_continuous(200.0, 1024.0, 0.0, 0.0, 0.0);

        let inner = Arc::new(Measurement {
            size: Size {
                width: 100.0,
                height: 30.0,
            },
            gap_after: 0.0,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![],
                ..Default::default()
            }),
            alignment: Alignment::End,
        });
        let wrapper = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 30.0,
            },
            gap_after: 0.0,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![ChildMeasurement {
                    node_id: NodeId::new(),
                    measurement: inner,
                }],
                ..Default::default()
            }),
            alignment: Alignment::Start,
        });

        p.place(NodeId::new(), &wrapper);
        let pages = p.finish();

        let Fragment::Container(container) = &pages[0].fragments[0] else {
            panic!()
        };
        let Fragment::Container(child) = &container.children[0] else {
            panic!()
        };
        assert_eq!(
            child.rect.x, 100.0,
            "End alignment: x = container_width - child_width"
        );
    }

    #[test]
    fn separate_border_adds_space() {
        let mut p = Paginator::new_continuous(200.0, 10000.0, 0.0, 0.0, 0.0);

        let child = ChildMeasurement {
            node_id: NodeId::new(),
            measurement: leaf_container_m(20.0),
        };

        let m = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 30.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![child],
                border: EdgeInsets {
                    top: 5.0,
                    left: 0.0,
                    bottom: 5.0,
                    right: 0.0,
                },
                ..Default::default()
            }),
        });

        let node_id = NodeId::new();
        p.place(node_id, &m);
        let pages = p.finish();

        let frag = &pages[0].fragments[0];
        let Fragment::Container(c) = frag else {
            panic!()
        };
        assert_eq!(c.rect.height, 30.0);

        let Fragment::Container(child_c) = &c.children[0] else {
            panic!()
        };
        assert_eq!(child_c.rect.y, 5.0); // border.top offset
    }

    #[test]
    fn separate_border_split_adds_border_both_pages() {
        let mut p = Paginator::new_paginated(200.0, 100.0, 10.0, 10.0, 0.0);

        let child1 = ChildMeasurement {
            node_id: NodeId::new(),
            measurement: leaf_container_m(60.0),
        };
        let child2 = ChildMeasurement {
            node_id: NodeId::new(),
            measurement: leaf_container_m(60.0),
        };

        let m = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 130.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![child1, child2],
                border: EdgeInsets {
                    top: 5.0,
                    left: 0.0,
                    bottom: 5.0,
                    right: 0.0,
                },
                ..Default::default()
            }),
        });

        p.place(NodeId::new(), &m);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);
        let Fragment::Container(c1) = &pages[0].fragments[0] else {
            panic!()
        };
        assert!(c1.breaks.bottom);
        let Fragment::Container(c2) = &pages[1].fragments[0] else {
            panic!()
        };
        assert!(c2.breaks.top);
    }

    #[test]
    fn collapse_border_overlaps_adjacent_children() {
        let mut p = Paginator::new_continuous(200.0, 10000.0, 0.0, 0.0, 0.0);

        let b = 2.0;
        let child1_h = 30.0;
        let child2_h = 30.0;

        let make_bordered_child = |h: f32| -> ChildMeasurement {
            ChildMeasurement {
                node_id: NodeId::new(),
                measurement: Arc::new(Measurement {
                    size: Size {
                        width: 200.0,
                        height: h,
                    },
                    gap_after: 0.0,
                    alignment: Alignment::Start,
                    content: MeasuredContent::Container(ContainerContent {
                        children: vec![],
                        scope: false,
                        direction: LayoutDirection::Vertical,
                        padding: EdgeInsets::ZERO,
                        border: EdgeInsets::all(b),
                        border_mode: BorderMode::Separate,
                        placeholders: vec![],
                    }),
                }),
            }
        };

        let child1 = make_bordered_child(child1_h);
        let child2 = make_bordered_child(child2_h);

        // collapsed height = (2+1)*2 + (30-4) + (30-4) = 6 + 26 + 26 = 58
        let collapsed_h = 3.0 * b + (child1_h - 2.0 * b) + (child2_h - 2.0 * b);
        let m = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: collapsed_h,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![child1, child2],
                scope: false,
                direction: LayoutDirection::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::all(b),
                border_mode: BorderMode::Collapse,
                placeholders: vec![],
            }),
        });

        p.place(NodeId::new(), &m);
        let pages = p.finish();

        let Fragment::Container(c) = &pages[0].fragments[0] else {
            panic!()
        };
        assert_eq!(c.children.len(), 2);

        let Fragment::Container(ch1) = &c.children[0] else {
            panic!()
        };
        assert_eq!(ch1.rect.y, 0.0); // collapsed with container border.top

        let Fragment::Container(ch2) = &c.children[1] else {
            panic!()
        };
        assert_eq!(ch2.rect.y, 28.0); // 30 - 2 overlap
    }

    #[test]
    fn collapse_horizontal_overlaps_cells() {
        let mut p = Paginator::new_continuous(200.0, 10000.0, 0.0, 0.0, 0.0);

        let b = 1.0;
        let cell_w = 50.0;

        let make_cell = || -> ChildMeasurement {
            ChildMeasurement {
                node_id: NodeId::new(),
                measurement: Arc::new(Measurement {
                    size: Size {
                        width: cell_w,
                        height: 30.0,
                    },
                    gap_after: 0.0,
                    alignment: Alignment::Start,
                    content: MeasuredContent::Container(ContainerContent {
                        children: vec![],
                        scope: true,
                        direction: LayoutDirection::Vertical,
                        padding: EdgeInsets::ZERO,
                        border: EdgeInsets::all(b),
                        border_mode: BorderMode::Separate,
                        placeholders: vec![],
                    }),
                }),
            }
        };

        let collapsed_w = 3.0 * b + (cell_w - 2.0 * b) * 2.0; // 99
        let m = Arc::new(Measurement {
            size: Size {
                width: collapsed_w,
                height: 30.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![make_cell(), make_cell()],
                scope: false,
                direction: LayoutDirection::Horizontal,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::all(b),
                border_mode: BorderMode::Collapse,
                placeholders: vec![],
            }),
        });

        p.place(NodeId::new(), &m);
        let pages = p.finish();

        let Fragment::Container(row) = &pages[0].fragments[0] else {
            panic!()
        };
        let Fragment::Container(c1) = &row.children[0] else {
            panic!()
        };
        assert_eq!(c1.rect.x, 0.0);

        let Fragment::Container(c2) = &row.children[1] else {
            panic!()
        };
        assert_eq!(c2.rect.x, 49.0); // 50 - 1 overlap
    }

    #[test]
    fn break_page_emits_placeholders() {
        // Container with a placeholder that spans a page break: the placeholder should appear on the first page
        use crate::fragment::PlaceholderData;

        let placeholder = MeasuredPlaceholder {
            id: 42,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            },
            data: PlaceholderData::Text("marker".to_string()),
        };

        let parent = Arc::new(Measurement {
            size: Size {
                width: 200.0,
                height: 200.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![child(100.0), child(100.0)],
                placeholders: vec![placeholder],
                ..Default::default()
            }),
        });

        let mut p = Paginator::new_paginated(200.0, 150.0, 0.0, 0.0, 0.0);
        p.place(NodeId::new(), &parent);
        let pages = p.finish();

        assert_eq!(pages.len(), 2);

        // Page 1: the container was split, so break_page builds the ContainerFragment
        let Fragment::Container(c1) = &pages[0].fragments[0] else {
            panic!("expected Container on page 1")
        };
        let has_placeholder = c1
            .children
            .iter()
            .any(|f| matches!(f, Fragment::Placeholder(ph) if ph.id == 42));
        assert!(
            has_placeholder,
            "placeholder should be emitted on the first page's container"
        );
    }
}
