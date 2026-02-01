use crate::layout::{
    Element, LayoutNode, Page, PageBreakPolicy, PositionedNode, RenderHints, SplitEdges,
};
use crate::model::{LayoutMode, NodeId};
use crate::types::{Point, Size};
use std::rc::Rc;

pub struct Paginator {
    page_width: f32,
    page_height: f32,
    margin_top: f32,
    margin_bottom: f32,
    margin_left: f32,
    layout_mode: LayoutMode,
}

struct ParentInfo {
    absolute_position: Point,
    size: Size,
    element: Option<Element>,
    index_in_current_page: Option<usize>,
    page_break_policy: PageBreakPolicy,
    render_hints: RenderHints,
    scope_id: Option<NodeId>,
}

struct PaginationState {
    page_width: f32,
    content_height: f32,
    margin_top: f32,
    margin_bottom: f32,
    margin_left: f32,
    layout_mode: LayoutMode,
    page_start_y: f32,
    current_y: f32,
    current_page_nodes: Vec<PositionedNode>,
    pages: Vec<Page>,
    parent_stack: Vec<ParentInfo>,
    explicit_page_break_pending: bool,
}

impl PaginationState {
    fn new(
        page_width: f32,
        content_height: f32,
        margin_top: f32,
        margin_bottom: f32,
        margin_left: f32,
        layout_mode: LayoutMode,
    ) -> Self {
        Self {
            page_width,
            content_height,
            margin_top,
            margin_bottom,
            margin_left,
            layout_mode,
            page_start_y: 0.0,
            current_y: 0.0,
            current_page_nodes: Vec::new(),
            pages: Vec::new(),
            parent_stack: Vec::new(),
            explicit_page_break_pending: false,
        }
    }

    fn start_new_page_at(&mut self, break_top_y: f32) {
        let top_margin = self.top_margin();
        let bottom_margin = self.bottom_margin(false);

        let original_gap = break_top_y - self.current_y;
        let margin_spacing = bottom_margin + top_margin;
        let desired_spacing = original_gap.max(margin_spacing);
        let additional_spacing = desired_spacing - margin_spacing;
        let new_page_top_spacing = top_margin + additional_spacing;

        self.page_start_y = break_top_y + top_margin - new_page_top_spacing;

        self.replicate_parents();
    }

    fn is_first_page(&self) -> bool {
        self.pages.is_empty()
    }

    fn top_margin(&self) -> f32 {
        match self.layout_mode {
            LayoutMode::Paginated { .. } => self.margin_top,
            LayoutMode::Continuous { .. } => {
                if self.is_first_page() {
                    self.margin_top
                } else {
                    0.0
                }
            }
        }
    }

    fn bottom_margin(&self, is_final: bool) -> f32 {
        match self.layout_mode {
            LayoutMode::Paginated { .. } => self.margin_bottom,
            LayoutMode::Continuous { .. } => {
                if is_final {
                    self.margin_bottom
                } else {
                    0.0
                }
            }
        }
    }

    fn pending_wrapper_bottom_padding(&self) -> f32 {
        self.parent_stack
            .iter()
            .filter_map(|p| Some(p.element.as_ref()?.as_wrapper()?.padding().bottom))
            .sum()
    }

    fn should_create_new_page(&self, node_bottom: f32) -> bool {
        let effective_bottom = node_bottom + self.pending_wrapper_bottom_padding();
        effective_bottom - self.page_start_y > self.content_height
            && !self.current_page_nodes.is_empty()
    }

    fn create_page(&mut self) {
        if self.current_page_nodes.is_empty() {
            return;
        }

        let top = self.top_margin();
        let bottom = self.bottom_margin(false);

        let page_height = match self.layout_mode {
            LayoutMode::Paginated { .. } => self.content_height + top + bottom,
            LayoutMode::Continuous { .. } => {
                let actual_content_height = self.current_y - self.page_start_y;
                actual_content_height + top + bottom
            }
        };

        let content_area_bottom = page_height - bottom;
        self.extend_wrappers_to_page_bottom(content_area_bottom);

        let page_root = PositionedNode {
            position: Point::zero(),
            node: Rc::new(LayoutNode {
                size: Size::new(self.page_width, page_height),
                element: None,
                children: Some(std::mem::take(&mut self.current_page_nodes)),
                page_break_policy: PageBreakPolicy::default(),
                render_hints: Default::default(),
                scope_id: None,
            }),
        };

        self.pages.push(Page::from_root(page_root));
    }

    fn extend_wrappers_to_page_bottom(&mut self, content_area_bottom: f32) {
        for parent in &self.parent_stack {
            let Some(idx) = parent.index_in_current_page else {
                continue;
            };
            let Some(old_node) = self.current_page_nodes.get(idx) else {
                continue;
            };
            let Some(ref element) = old_node.node.element else {
                continue;
            };
            if element.as_wrapper().is_none() {
                continue;
            }

            let node_top = old_node.position.y;
            let new_height = content_area_bottom - node_top;

            if (new_height - old_node.node.size.height).abs() < 0.01 {
                continue;
            }

            let split_edges = SplitEdges {
                top: false,
                bottom: true,
            };

            let Some(new_element) = element.with_adjusted_bounds(new_height, split_edges) else {
                continue;
            };

            let new_node = PositionedNode {
                position: old_node.position,
                node: Rc::new(LayoutNode {
                    size: Size::new(old_node.node.size.width, new_height),
                    element: Some(new_element),
                    children: None,
                    page_break_policy: old_node.node.page_break_policy,
                    render_hints: old_node.node.render_hints.clone(),
                    scope_id: None,
                }),
            };
            self.current_page_nodes[idx] = new_node;
        }
    }

    fn replicate_parents(&mut self) {
        let top = self.top_margin();
        let margin_x = self.margin_left;
        let page_start_y = self.page_start_y;

        for i in 0..self.parent_stack.len() {
            let parent = &self.parent_stack[i];

            let adjusted_y = parent.absolute_position.y - page_start_y + top;
            let adjusted_x = parent.absolute_position.x + margin_x;

            let (final_y, final_height, final_element) = if adjusted_y < top
                && parent
                    .element
                    .as_ref()
                    .is_some_and(|e| e.as_wrapper().is_some())
            {
                let overflow = top - adjusted_y;
                let new_height = parent.size.height - overflow;
                let split_edges = SplitEdges {
                    top: true,
                    bottom: false,
                };
                let new_element = parent
                    .element
                    .as_ref()
                    .and_then(|e| e.with_adjusted_bounds(new_height, split_edges));
                (top, new_height, new_element)
            } else {
                (adjusted_y, parent.size.height, parent.element.clone())
            };

            let replicated = PositionedNode {
                position: Point::new(adjusted_x, final_y),
                node: Rc::new(LayoutNode {
                    size: Size::new(parent.size.width, final_height),
                    element: final_element,
                    children: None,
                    page_break_policy: parent.page_break_policy,
                    render_hints: parent.render_hints.clone(),
                    scope_id: None,
                }),
            };
            self.current_page_nodes.push(replicated);

            self.parent_stack[i].index_in_current_page = Some(self.current_page_nodes.len() - 1);
        }
    }

    fn create_final_page(&mut self) {
        if self.current_page_nodes.is_empty() {
            return;
        }

        let top = self.top_margin();
        let bottom = self.bottom_margin(true);
        let content_height = self.current_y - self.page_start_y;
        let page_root = PositionedNode {
            position: Point::zero(),
            node: Rc::new(LayoutNode {
                size: Size::new(self.page_width, content_height + top + bottom),
                element: None,
                children: Some(std::mem::take(&mut self.current_page_nodes)),
                page_break_policy: PageBreakPolicy::default(),
                render_hints: Default::default(),
                scope_id: None,
            }),
        };

        self.pages.push(Page::from_root(page_root));
    }

    fn finish(mut self) -> Vec<Page> {
        self.create_final_page();

        if self.pages.is_empty() {
            self.pages.push(Page::from_root(PositionedNode {
                position: Point::zero(),
                node: Rc::new(LayoutNode {
                    size: Size::new(self.page_width, self.margin_top + self.margin_bottom),
                    element: None,
                    children: None,
                    page_break_policy: Default::default(),
                    render_hints: Default::default(),
                    scope_id: None,
                }),
            }));
        }

        self.pages
    }
}

impl Paginator {
    pub fn new(
        page_width: f32,
        page_height: f32,
        margin_top: f32,
        margin_bottom: f32,
        margin_left: f32,
        layout_mode: LayoutMode,
    ) -> Self {
        Self {
            page_width,
            page_height,
            margin_top,
            margin_bottom,
            margin_left,
            layout_mode,
        }
    }

    fn content_height(&self) -> f32 {
        match self.layout_mode {
            LayoutMode::Paginated { .. } => self.page_height - self.margin_top - self.margin_bottom,
            LayoutMode::Continuous { .. } => 1024.0 - self.margin_top - self.margin_bottom,
        }
    }

    pub fn paginate(&self, root: LayoutNode) -> Vec<Page> {
        let content_height = self.content_height();
        let mut state = PaginationState::new(
            self.page_width,
            content_height,
            self.margin_top,
            self.margin_bottom,
            self.margin_left,
            self.layout_mode,
        );

        let root_positioned = PositionedNode {
            position: Point::zero(),
            node: Rc::new(root),
        };
        self.collect_nodes(&root_positioned, Point::zero(), &mut state);

        state.finish()
    }

    fn find_first_leaf_position(node: &LayoutNode, abs_y: f32) -> Option<(f32, f32)> {
        if let Some(children) = &node.children {
            if let Some(first_child) = children.first() {
                let child_abs_y = abs_y + first_child.position.y;
                Self::find_first_leaf_position(&first_child.node, child_abs_y)
            } else {
                Some((abs_y, node.size.height))
            }
        } else {
            Some((abs_y, node.size.height))
        }
    }

    fn wrapper_needs_page_break(node: &LayoutNode, abs_y: f32, state: &PaginationState) -> bool {
        let wrapper = match node.element.as_ref().and_then(|e| e.as_wrapper()) {
            Some(w) if w.prevent_empty_on_page_break() => w,
            _ => return false,
        };

        let (first_leaf_abs_y, first_leaf_height) =
            match Self::find_first_leaf_position(node, abs_y) {
                Some(pos) => pos,
                None => return false,
            };

        let first_leaf_bottom = first_leaf_abs_y + first_leaf_height;
        let total_padding = state.pending_wrapper_bottom_padding() + wrapper.padding().bottom;
        let effective_bottom = first_leaf_bottom + total_padding;

        effective_bottom - state.page_start_y > state.content_height
    }

    fn collect_nodes(
        &self,
        positioned: &PositionedNode,
        offset: Point,
        state: &mut PaginationState,
    ) {
        let abs_x = offset.x + positioned.position.x;
        let abs_y = offset.y + positioned.position.y;
        let node = &positioned.node;
        let node_bottom = abs_y + node.size.height;

        let has_children = node.children.is_some();
        let is_page_break = matches!(state.layout_mode, LayoutMode::Paginated { .. })
            && matches!(&node.element, Some(Element::Line(line)) if line.has_page_break);

        let avoid_break = node.page_break_policy == PageBreakPolicy::Avoid;
        let should_check_fit = !has_children || avoid_break;

        // 큰 avoid 노드가 무한히 break를 트리거하는 것을 방지하기 위해 페이지 top에서는 break하지 않음
        let is_at_start_of_page = state.current_page_nodes.len() <= state.parent_stack.len();

        let wrapper_overflows =
            !is_at_start_of_page && Self::wrapper_needs_page_break(&node, abs_y, state);
        let node_overflows =
            should_check_fit && !is_at_start_of_page && state.should_create_new_page(node_bottom);

        if wrapper_overflows || node_overflows {
            state.create_page();
            state.start_new_page_at(abs_y);
        }

        if state.explicit_page_break_pending {
            state.explicit_page_break_pending = false;
            state.start_new_page_at(abs_y);
        }

        let top = state.top_margin();
        let horizontal_margin = state.margin_left;

        let adjusted_y = abs_y - state.page_start_y + top;

        let adjusted_position = Point::new(abs_x + horizontal_margin, adjusted_y);

        if let Some(children) = &node.children {
            let adjusted_node = PositionedNode {
                position: adjusted_position,
                node: Rc::new(LayoutNode {
                    size: node.size,
                    element: node.element.clone(),
                    children: None,
                    page_break_policy: node.page_break_policy,
                    render_hints: node.render_hints.clone(),
                    scope_id: node.scope_id,
                }),
            };

            state.current_page_nodes.push(adjusted_node);
            let added_index = state.current_page_nodes.len() - 1;

            state.parent_stack.push(ParentInfo {
                absolute_position: Point::new(abs_x, abs_y),
                size: node.size,
                element: node.element.clone(),
                index_in_current_page: Some(added_index),
                page_break_policy: node.page_break_policy,
                render_hints: node.render_hints.clone(),
                scope_id: node.scope_id,
            });

            for child in children {
                self.collect_nodes(child, Point::new(abs_x, abs_y), state);
            }
            state.parent_stack.pop();
        } else {
            let merged_hints = state
                .parent_stack
                .iter()
                .fold(positioned.node.render_hints.clone(), |acc, parent| {
                    acc.merge(&parent.render_hints)
                });

            let inherited_scope_id = positioned
                .node
                .scope_id
                .or_else(|| state.parent_stack.iter().rev().find_map(|p| p.scope_id));

            let adjusted_node = if merged_hints.default_text_color.is_some()
                || inherited_scope_id != positioned.node.scope_id
            {
                PositionedNode {
                    position: adjusted_position,
                    node: Rc::new(LayoutNode {
                        size: positioned.node.size,
                        element: positioned.node.element.clone(),
                        children: None,
                        page_break_policy: positioned.node.page_break_policy,
                        render_hints: merged_hints,
                        scope_id: inherited_scope_id,
                    }),
                }
            } else {
                PositionedNode {
                    position: adjusted_position,
                    node: Rc::clone(&positioned.node),
                }
            };
            state.current_y = node_bottom;
            state.current_page_nodes.push(adjusted_node);

            if is_page_break {
                state.create_page();
                state.explicit_page_break_pending = true;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::layout::elements::{LineElement, LineMetric};
    use crate::layout::{Element, LayoutNode, Paginator, PositionedNode, SplitEdges};
    use crate::model::{LayoutMode, NodeId};
    use crate::types::{Point, Size};
    use std::rc::Rc;

    fn create_dummy_line_element(has_page_break: bool) -> LineElement {
        LineElement::build(
            NodeId::new(),
            Size::new(100.0, 20.0),
            0,
            Rc::new(parley::Layout::default()),
            LineMetric {
                top: 0.0,
                left: 0.0,
                height: 20.0,
                leading: 0.0,
                baseline: 14.0,
                ascent: 14.0,
                content_width: 100.0,
                start_offset: 0,
                end_offset: 0,
                clusters: vec![],
                break_reason: parley::layout::BreakReason::None,
                grapheme_offsets: vec![],
            },
            None,
            false,
            Rc::from(""),
            vec![],
            vec![],
            has_page_break,
        )
    }

    #[test]
    fn test_paginator_discards_gap_at_page_break() {
        let page_height = 100.0;
        let page_margin = 10.0;
        let layout_mode = LayoutMode::Paginated {
            page_width: 100.0,
            page_height,
            page_margin_top: page_margin,
            page_margin_bottom: page_margin,
            page_margin_left: page_margin,
            page_margin_right: page_margin,
        };

        let node1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 50.0),
            element: None,
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let node2 = Rc::new(LayoutNode {
            size: Size::new(80.0, 50.0),
            element: None,
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let children = vec![
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: node1,
            },
            PositionedNode {
                position: Point::new(0.0, 70.0),
                node: node2,
            },
        ];

        let root = LayoutNode {
            size: Size::new(80.0, 120.0),
            element: None,
            children: Some(children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        };

        let paginator = Paginator::new(
            100.0,
            page_height,
            page_margin,
            page_margin,
            page_margin,
            layout_mode,
        );
        let pages = paginator.paginate(root);

        assert_eq!(pages.len(), 2);

        let page2 = &pages[1];
        let p2_children = page2.root.node.children.as_ref().unwrap();
        assert_eq!(p2_children.len(), 2);
        let p2_node2 = &p2_children[1];

        // gap = 20, margins = 20 → 10 + max(0, 0) = 10
        assert_eq!(p2_node2.position.y, 10.0);
    }

    #[test]
    fn test_paginator_collapses_gap_larger_than_margin() {
        let page_height = 100.0;
        let page_margin = 10.0;
        let layout_mode = LayoutMode::Paginated {
            page_width: 100.0,
            page_height,
            page_margin_top: page_margin,
            page_margin_bottom: page_margin,
            page_margin_left: page_margin,
            page_margin_right: page_margin,
        };

        let node1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: None,
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let node2 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: None,
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let children = vec![
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: node1,
            },
            PositionedNode {
                position: Point::new(0.0, 70.0),
                node: node2,
            },
        ];

        let root = LayoutNode {
            size: Size::new(80.0, 90.0),
            element: None,
            children: Some(children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        };

        let paginator = Paginator::new(
            100.0,
            page_height,
            page_margin,
            page_margin,
            page_margin,
            layout_mode,
        );
        let pages = paginator.paginate(root);

        assert_eq!(pages.len(), 2);

        let page2 = &pages[1];
        let p2_children = page2.root.node.children.as_ref().unwrap();
        let p2_node2 = &p2_children[1];

        // gap = 50, margins = 20 → 10 + max(0, 30) = 40
        assert_eq!(p2_node2.position.y, 40.0);
    }

    #[test]
    fn test_paginator_three_way_collapse() {
        let page_height = 100.0;
        let page_margin = 10.0;
        let layout_mode = LayoutMode::Paginated {
            page_width: 100.0,
            page_height,
            page_margin_top: page_margin,
            page_margin_bottom: page_margin,
            page_margin_left: page_margin,
            page_margin_right: page_margin,
        };

        let node1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 50.0),
            element: None,
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let node2 = Rc::new(LayoutNode {
            size: Size::new(80.0, 50.0),
            element: None,
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let children = vec![
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: node1,
            },
            PositionedNode {
                position: Point::new(0.0, 55.0),
                node: node2,
            },
        ];

        let root = LayoutNode {
            size: Size::new(80.0, 105.0),
            element: None,
            children: Some(children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        };

        let paginator = Paginator::new(
            100.0,
            page_height,
            page_margin,
            page_margin,
            page_margin,
            layout_mode,
        );
        let pages = paginator.paginate(root);

        assert_eq!(pages.len(), 2);

        let page2 = &pages[1];
        let p2_children = page2.root.node.children.as_ref().unwrap();
        let p2_node2 = &p2_children[1];

        // gap = 5, margins = 20 → 10 + max(0, -15) = 10
        assert_eq!(p2_node2.position.y, 10.0);
    }

    #[test]
    fn test_paginator_inline_page_break() {
        let page_height = 100.0;
        let page_margin = 10.0;
        let layout_mode = LayoutMode::Paginated {
            page_width: 100.0,
            page_height,
            page_margin_top: page_margin,
            page_margin_bottom: page_margin,
            page_margin_left: page_margin,
            page_margin_right: page_margin,
        };

        let node1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: Some(Element::Line(create_dummy_line_element(false))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let node2 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: Some(Element::Line(create_dummy_line_element(true))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let node3 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: Some(Element::Line(create_dummy_line_element(false))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let children = vec![
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: node1,
            },
            PositionedNode {
                position: Point::new(0.0, 20.0),
                node: node2,
            },
            PositionedNode {
                position: Point::new(0.0, 40.0),
                node: node3,
            },
        ];

        let root = LayoutNode {
            size: Size::new(80.0, 60.0),
            element: None,
            children: Some(children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        };

        let paginator = Paginator::new(
            100.0,
            page_height,
            page_margin,
            page_margin,
            page_margin,
            layout_mode,
        );
        let pages = paginator.paginate(root);

        assert_eq!(pages.len(), 2);

        let page1 = &pages[0];
        let p1_children = page1.root.node.children.as_ref().unwrap();
        assert_eq!(p1_children.len(), 3);

        let page2 = &pages[1];
        let p2_children = page2.root.node.children.as_ref().unwrap();
        assert_eq!(p2_children.len(), 2);
    }

    #[test]
    fn test_paginator_no_page_break() {
        let page_height = 100.0;
        let page_margin = 10.0;
        let layout_mode = LayoutMode::Paginated {
            page_width: 100.0,
            page_height,
            page_margin_top: page_margin,
            page_margin_bottom: page_margin,
            page_margin_left: page_margin,
            page_margin_right: page_margin,
        };

        let node1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: Some(Element::Line(create_dummy_line_element(false))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let node2 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: Some(Element::Line(create_dummy_line_element(false))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let node3 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: Some(Element::Line(create_dummy_line_element(false))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let children = vec![
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: node1,
            },
            PositionedNode {
                position: Point::new(0.0, 20.0),
                node: node2,
            },
            PositionedNode {
                position: Point::new(0.0, 40.0),
                node: node3,
            },
        ];

        let root = LayoutNode {
            size: Size::new(80.0, 60.0),
            element: None,
            children: Some(children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        };

        let paginator = Paginator::new(
            100.0,
            page_height,
            page_margin,
            page_margin,
            page_margin,
            layout_mode,
        );
        let pages = paginator.paginate(root);

        assert_eq!(pages.len(), 1);
    }

    #[test]
    fn test_paginator_large_avoid_node() {
        let page_height = 100.0;
        let page_margin = 10.0;
        let layout_mode = LayoutMode::Paginated {
            page_width: 100.0,
            page_height,
            page_margin_top: page_margin,
            page_margin_bottom: page_margin,
            page_margin_left: page_margin,
            page_margin_right: page_margin,
        };

        let node1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: None,
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let node2 = Rc::new(LayoutNode {
            size: Size::new(80.0, 150.0),
            element: None,
            children: None,
            page_break_policy: crate::layout::PageBreakPolicy::Avoid,
            render_hints: Default::default(),
            scope_id: None,
        });

        let children = vec![
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: node1,
            },
            PositionedNode {
                position: Point::new(0.0, 20.0),
                node: node2,
            },
        ];

        let root = LayoutNode {
            size: Size::new(80.0, 170.0),
            element: None,
            children: Some(children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        };

        let paginator = Paginator::new(
            100.0,
            page_height,
            page_margin,
            page_margin,
            page_margin,
            layout_mode,
        );
        let pages = paginator.paginate(root);

        assert!(pages.len() >= 2);
        let p1 = &pages[0];
        assert_eq!(p1.root.node.children.as_ref().unwrap().len(), 2);

        let p2 = &pages[1];
        let p2_children = p2.root.node.children.as_ref().unwrap();
        assert!(!p2_children.is_empty());
        let p2_item = &p2_children[1];
        assert_eq!(p2_item.node.size.height, 150.0);
        assert_eq!(p2_item.position.y, 10.0);
    }

    #[test]
    fn test_paginator_nested_auto_split() {
        let page_height = 100.0;
        let page_margin = 10.0;
        let layout_mode = LayoutMode::Paginated {
            page_width: 100.0,
            page_height,
            page_margin_top: page_margin,
            page_margin_bottom: page_margin,
            page_margin_left: page_margin,
            page_margin_right: page_margin,
        };

        let item1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 40.0),
            element: None,
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let item2 = Rc::new(LayoutNode {
            size: Size::new(80.0, 110.0),
            element: None,
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let content_children = vec![
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: item1,
            },
            PositionedNode {
                position: Point::new(0.0, 40.0),
                node: item2,
            },
        ];

        let content_node = Rc::new(LayoutNode {
            size: Size::new(80.0, 150.0),
            element: None,
            children: Some(content_children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let wrapper_node = Rc::new(LayoutNode {
            size: Size::new(80.0, 170.0), // Content + Padding
            element: Some(Element::FoldContent(
                crate::layout::elements::FoldContentElement::new(
                    Size::new(80.0, 170.0),
                    SplitEdges::default(),
                    crate::model::NodeId::new(),
                ),
            )),
            children: Some(vec![PositionedNode {
                position: Point::new(10.0, 10.0), // Padding
                node: content_node,
            }]),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let root_children = vec![PositionedNode {
            position: Point::new(0.0, 0.0),
            node: wrapper_node,
        }];

        let root = LayoutNode {
            size: Size::new(80.0, 170.0),
            element: None,
            children: Some(root_children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        };

        let paginator = Paginator::new(
            100.0,
            page_height,
            page_margin,
            page_margin,
            page_margin,
            layout_mode,
        );
        let pages = paginator.paginate(root);

        assert!(pages.len() >= 2);

        let p2 = &pages[1];
        let p2_children = p2.root.node.children.as_ref().unwrap(); // Root children: Root(Repl), Wrapper(Repl), Item2
        assert!(p2_children.len() >= 2);
        let p2_wrapper = &p2_children[1];

        assert!(matches!(
            p2_wrapper.node.element,
            Some(Element::FoldContent(_))
        ));
        assert_eq!(p2_wrapper.node.size.height, 120.0);

        assert_eq!(p2_wrapper.position.y, 10.0);
    }

    #[test]
    fn test_paginator_avoid_break() {
        let page_height = 100.0;
        let page_margin = 10.0;
        let layout_mode = LayoutMode::Paginated {
            page_width: 100.0,
            page_height,
            page_margin_top: page_margin,
            page_margin_bottom: page_margin,
            page_margin_left: page_margin,
            page_margin_right: page_margin,
        };

        let node1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 60.0),
            element: None,
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let node2 = Rc::new(LayoutNode {
            size: Size::new(80.0, 50.0),
            element: None,
            children: None,
            page_break_policy: crate::layout::PageBreakPolicy::Avoid,
            render_hints: Default::default(),
            scope_id: None,
        });

        let children = vec![
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: node1,
            },
            PositionedNode {
                position: Point::new(0.0, 60.0),
                node: node2,
            },
        ];

        let root = LayoutNode {
            size: Size::new(80.0, 110.0),
            element: None,
            children: Some(children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        };

        let paginator = Paginator::new(
            100.0,
            page_height,
            page_margin,
            page_margin,
            page_margin,
            layout_mode,
        );
        let pages = paginator.paginate(root);

        assert_eq!(pages.len(), 2);

        let p1 = &pages[0];
        let p1_children = p1.root.node.children.as_ref().unwrap();

        let p2 = &pages[1];
        let p2_children = p2.root.node.children.as_ref().unwrap();

        let p2_item2 = p2_children
            .iter()
            .find(|n| n.node.size.height == 50.0)
            .expect("Item 2 should be on Page 2");
        assert_eq!(
            p2_item2.position.y, 10.0,
            "Item 2 should start at top margin"
        );

        let p1_item2 = p1_children.iter().find(|n| n.node.size.height == 50.0);
        assert!(p1_item2.is_none(), "Item 2 should not be on Page 1");
    }
    #[test]
    fn test_paginator_page_break_reset_y() {
        let page_height = 100.0;
        let page_margin = 10.0;
        let layout_mode = LayoutMode::Paginated {
            page_width: 100.0,
            page_height,
            page_margin_top: page_margin,
            page_margin_bottom: page_margin,
            page_margin_left: page_margin,
            page_margin_right: page_margin,
        };

        let p1_line1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: Some(Element::Line(create_dummy_line_element(false))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });
        let p1_line2 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: Some(Element::Line(create_dummy_line_element(true))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });
        let p1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 40.0),
            element: None,
            children: Some(vec![
                PositionedNode {
                    position: Point::new(0.0, 0.0),
                    node: p1_line1,
                },
                PositionedNode {
                    position: Point::new(0.0, 20.0),
                    node: p1_line2,
                },
            ]),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let p2_line1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: Some(Element::Line(create_dummy_line_element(false))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });
        let p2 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: None,
            children: Some(vec![PositionedNode {
                position: Point::new(0.0, 0.0),
                node: p2_line1,
            }]),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let children = vec![
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: p1,
            },
            PositionedNode {
                position: Point::new(0.0, 40.0),
                node: p2,
            },
        ];

        let root = LayoutNode {
            size: Size::new(80.0, 60.0),
            element: None,
            children: Some(children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        };

        let paginator = Paginator::new(
            100.0,
            page_height,
            page_margin,
            page_margin,
            page_margin,
            layout_mode,
        );
        let pages = paginator.paginate(root);

        assert_eq!(pages.len(), 2);

        let page2 = &pages[1];
        let p2_children = page2.root.node.children.as_ref().unwrap();

        let p2_node = p2_children
            .iter()
            .find(|n| n.node.size.height == 20.0)
            .expect("Should find P2 node");

        assert_eq!(
            p2_node.position.y, 10.0,
            "Paragraph 2 should start at top margin"
        );
    }

    #[test]
    fn test_paginator_list_item_split() {
        let mut p = id!();

        let mut runtime = runtime! {
            viewport { 200, 200, 1.0 }
            doc {
                @p paragraph {
                    text { "Pre-line 1" }
                    hard_break {}
                    text { "Pre-line 2" }
                    hard_break {}
                    text { "Pre-line 3" }
                }
                bullet_list {
                    list_item {
                        paragraph {
                            text { "Item 1" }
                            hard_break {}
                            text { "Item 2" }
                            hard_break {}
                            text { "Item 3" }
                        }
                    }
                }
            }
            selection { (p, 0) }
        };

        runtime.update(crate::runtime::Message::SetLayoutMode {
            mode: crate::model::LayoutMode::Paginated {
                page_width: 200.0,
                page_height: 125.0,
                page_margin_top: 0.0,
                page_margin_bottom: 0.0,
                page_margin_left: 0.0,
                page_margin_right: 0.0,
            },
        });

        runtime.layout();
        let pages = runtime.pages();

        assert!(pages.len() >= 2, "Should split into at least 2 pages");

        let p1 = &pages[0];
        let has_item1_p1 = p1.spatial_index().iter().any(|entry| {
            if let Element::Line(line) = entry.element() {
                line.text.contains("Item 1")
            } else {
                false
            }
        });

        assert!(
            has_item1_p1,
            "Page 1 should contain 'Item 1' (indicating the list item started on Page 1)"
        );

        let p2 = &pages[1];
        let has_item_rest_p2 = p2.spatial_index().iter().any(|entry| {
            if let Element::Line(line) = entry.element() {
                line.text.contains("Item 2")
            } else {
                false
            }
        });

        assert!(
            has_item_rest_p2,
            "Page 2 should contain 'Item 2' (indicating the list item continued to Page 2)"
        );
    }

    #[test]
    fn test_paginator_continuous_mode_ignores_page_break() {
        let page_height = 100.0;
        let page_margin = 10.0;
        let layout_mode = LayoutMode::Continuous { max_width: 100.0 };

        let node1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: Some(Element::Line(create_dummy_line_element(false))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let node2 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: Some(Element::Line(create_dummy_line_element(true))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let node3 = Rc::new(LayoutNode {
            size: Size::new(80.0, 20.0),
            element: Some(Element::Line(create_dummy_line_element(false))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let children = vec![
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: node1,
            },
            PositionedNode {
                position: Point::new(0.0, 20.0),
                node: node2,
            },
            PositionedNode {
                position: Point::new(0.0, 40.0),
                node: node3,
            },
        ];

        let root = LayoutNode {
            size: Size::new(80.0, 60.0),
            element: None,
            children: Some(children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        };

        let paginator = Paginator::new(
            100.0,
            page_height,
            page_margin,
            page_margin,
            page_margin,
            layout_mode,
        );
        let pages = paginator.paginate(root);

        assert_eq!(pages.len(), 1, "Continuous mode should ignore page break");
    }

    #[test]
    fn test_paginator_replicates_parent_element() {
        use crate::layout::elements::FoldContentElement;

        let page_height = 100.0;
        let page_margin = 10.0;
        let layout_mode = LayoutMode::Paginated {
            page_width: 100.0,
            page_height,
            page_margin_top: page_margin,
            page_margin_bottom: page_margin,
            page_margin_left: page_margin,
            page_margin_right: page_margin,
        };

        let parent_element = Element::FoldContent(FoldContentElement::new(
            Size::new(80.0, 150.0),
            SplitEdges::default(),
            crate::model::NodeId::new(),
        ));

        let node1 = Rc::new(LayoutNode {
            size: Size::new(80.0, 50.0),
            element: Some(Element::Line(create_dummy_line_element(false))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let node2 = Rc::new(LayoutNode {
            size: Size::new(80.0, 50.0),
            element: Some(Element::Line(create_dummy_line_element(false))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        });

        let children = vec![
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: node1,
            },
            PositionedNode {
                position: Point::new(0.0, 60.0), // Gap 10
                node: node2,
            },
        ];

        let root = LayoutNode {
            size: Size::new(80.0, 150.0),
            element: Some(parent_element),
            children: Some(children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        };

        let paginator = Paginator::new(
            100.0,
            page_height,
            page_margin,
            page_margin,
            page_margin,
            layout_mode,
        );
        let pages = paginator.paginate(root);

        assert_eq!(pages.len(), 2);

        let p1_children = pages[0].root.node.children.as_ref().unwrap();
        assert!(p1_children.len() >= 1);
        let p1_bg = &p1_children[0];
        assert!(
            matches!(p1_bg.node.element, Some(Element::FoldContent(_))),
            "Page 1 should have parent element"
        );

        let p2_children = pages[1].root.node.children.as_ref().unwrap();
        assert!(p2_children.len() >= 1);
        let p2_bg = &p2_children[0];

        assert!(
            matches!(p2_bg.node.element, Some(Element::FoldContent(_))),
            "Page 2 should have replicated parent element"
        );
    }

    #[test]
    fn test_paginator_fold_node_split() {
        let mut intro_id = id!();
        let mut fold_id = id!();
        let mut title_id = id!();
        let mut content_id = id!();
        let mut p1_id = id!();

        let mut runtime = runtime! {
            viewport { paginated { width: 200.0, height: 100.0, margin: 0.0 } }
            doc {
                @intro_id paragraph {
                    text { "Intro" }
                }
                @fold_id fold {
                    @title_id fold_title {
                        text { "Title" }
                    }
                    @content_id fold_content {
                        @p1_id paragraph {
                            text { "Line 1" }
                            hard_break {}
                            text { "Line 2" }
                            hard_break {}
                            text { "Line 3" }
                            hard_break {}
                            text { "Line 4" }
                            hard_break {}
                            text { "Line 5" }
                            hard_break {}
                            text { "Line 6" }
                            hard_break {}
                            text { "Line 7" }
                            hard_break {}
                            text { "Line 8" }
                        }
                    }
                }
            }
            selection { (title_id, 0) }
        };

        runtime.update(crate::runtime::Message::ToggleFoldExpansion {
            node_id: fold_id.to_string(),
        });

        runtime.layout();
        let pages = runtime.pages();
        assert!(
            pages.len() >= 2,
            "Should split into at least 2 pages, got {}",
            pages.len()
        );

        let p1_children = pages[0].root.node.children.as_ref().unwrap();
        assert!(p1_children.len() >= 2);

        let fold_container = p1_children
            .iter()
            .find(|n| matches!(n.node.element, Some(Element::FoldContent(_))));
        assert!(fold_container.is_some(), "Page 1 should have FoldContent");

        let p2_children = pages[1].root.node.children.as_ref().unwrap();
        let fold_container_p2 = p2_children
            .iter()
            .find(|n| matches!(n.node.element, Some(Element::FoldContent(_))));
        assert!(
            fold_container_p2.is_some(),
            "Page 2 should have replicated FoldContent"
        );
    }

    #[test]
    fn test_paginator_nested_fold_split() {
        let mut outer_fold_id = id!();
        let mut inner_fold_id = id!();
        let mut inner_title_id = id!();

        let mut runtime = runtime! {
            viewport { paginated { width: 200.0, height: 100.0, margin: 0.0 } }
            doc {
                @outer_fold_id fold {
                    fold_title { text { "Outer" } }
                    fold_content {
                        paragraph {
                            text { "Filler Line 1" }
                            hard_break {}
                            text { "Filler Line 2" }
                            hard_break {}
                            text { "Filler Line 3" }
                        }
                        @inner_fold_id fold {
                            @inner_title_id fold_title { text { "Inner Title" } }
                            fold_content {
                                paragraph { text { "Inner Content" } }
                            }
                        }
                    }
                }
            }
        };

        // Expand folds
        runtime.update(crate::runtime::Message::ToggleFoldExpansion {
            node_id: outer_fold_id.to_string(),
        });
        runtime.update(crate::runtime::Message::ToggleFoldExpansion {
            node_id: inner_fold_id.to_string(),
        });

        runtime.layout();
        let pages = runtime.pages();

        assert!(pages.len() >= 2, "Should be split into at least 2 pages");
        let p1_root = &pages[0].root.node;
        let p1_children = p1_root.children.as_ref().unwrap();

        let outer_container_p1 = p1_children
            .iter()
            .find(|n| matches!(n.node.element, Some(Element::FoldContent(_))));
        assert!(
            outer_container_p1.is_some(),
            "Page 1 should have Outer Fold Content"
        );

        let fold_contents_p1 = p1_children
            .iter()
            .filter(|n| matches!(n.node.element, Some(Element::FoldContent(_))))
            .count();
        assert_eq!(
            fold_contents_p1, 1,
            "Page 1 should only have Outer Fold Content, Inner should be gone"
        );

        let p2_root = &pages[1].root.node;
        let p2_children = p2_root.children.as_ref().unwrap();

        let fold_contents_p2 = p2_children
            .iter()
            .filter(|n| matches!(n.node.element, Some(Element::FoldContent(_))))
            .count();
        assert!(
            fold_contents_p2 >= 1,
            "Page 2 should have at least one replicated Fold Content"
        );
    }

    #[test]
    fn test_paginator_callout_split() {
        let runtime = runtime! {
            viewport { paginated { width: 300.0, height: 200.0, margin: 20.0 } }
            doc {
                callout {
                    paragraph { text { "Header" } }
                    paragraph { text { "Content 1" } }
                    paragraph { text { "Content 2" } }
                    paragraph { text { "Content 3" } }
                    paragraph { text { "Content 4" } }
                    paragraph { text { "Content 5 - Long enough to maybe wrap or take space" } }
                    paragraph { text { "Content 6" } }
                    paragraph { text { "Content 7" } }
                    paragraph { text { "Content 8" } }
                }
            }
        };

        let pages = runtime.pages();
        assert!(pages.len() >= 2, "Should split into multiple pages");

        let p1_root = &pages[0].root.node;
        assert!(
            find_element(p1_root, |e| matches!(e, Element::CalloutBackground(_))),
            "Page 1 missing Background"
        );
        assert!(
            find_element(p1_root, |e| matches!(e, Element::CalloutIcon(_))),
            "Page 1 missing Icon"
        );

        let p2_root = &pages[1].root.node;
        assert!(
            find_element(p2_root, |e| matches!(e, Element::CalloutBackground(_))),
            "Page 2 missing Background"
        );
        assert!(
            !find_element(p2_root, |e| matches!(e, Element::CalloutIcon(_))),
            "Page 2 has Icon (should not)"
        );
    }

    fn find_element<F>(node: &LayoutNode, predicate: F) -> bool
    where
        F: Fn(&crate::layout::Element) -> bool + Copy,
    {
        if let Some(e) = &node.element {
            if predicate(e) {
                return true;
            }
        }
        if let Some(children) = &node.children {
            for child in children {
                if find_element(&child.node, predicate) {
                    return true;
                }
            }
        }
        false
    }
}
