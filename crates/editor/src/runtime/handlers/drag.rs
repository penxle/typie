use super::super::{Effect, Runtime};
use crate::layout::cursor::{Cursor, NavigationContext};
use crate::model::Fragment;
use crate::runtime::PointerMode;
use crate::runtime::message::Modifier;
use crate::state::{Position, Selection};
use crate::transaction::Transaction;

impl Runtime {
    pub(crate) fn can_drag_at(&self, page_idx: usize, x: f32, y: f32) -> bool {
        if self.state.selection.is_collapsed() {
            return false;
        }

        let Some(page) = self.pages.get(page_idx) else {
            return false;
        };

        let ctx = NavigationContext::new(&self.state.doc);
        let Some(hit_selection) = Cursor::hit_test(&ctx, page, x, y) else {
            return false;
        };

        let position = hit_selection.head;

        if self.is_position_in_selection(position) {
            return true;
        }

        if self.is_block_selectable_hit(&hit_selection) {
            if let (Ok((sel_from, sel_to)), Ok((hit_from, hit_to))) = (
                self.state.selection.as_sorted(&self.state.doc),
                hit_selection.as_sorted(&self.state.doc),
            ) {
                use crate::state::position_helpers::compare_positions;
                use std::cmp::Ordering;

                let start_ok = matches!(
                    compare_positions(&self.state.doc, sel_from, hit_from),
                    Ok(Ordering::Less | Ordering::Equal)
                );
                let end_ok = matches!(
                    compare_positions(&self.state.doc, hit_to, sel_to),
                    Ok(Ordering::Less | Ordering::Equal)
                );

                if start_ok && end_ok {
                    return true;
                }
            }
        }

        false
    }

    pub(crate) fn handle_drag_start(&mut self, _page_idx: usize, _x: f32, _y: f32) -> Vec<Effect> {
        self.set_pointer_mode(PointerMode::DraggingContent);
        vec![]
    }

    pub(crate) fn handle_drag_enter(&mut self) -> Vec<Effect> {
        if self.pointer.mode == PointerMode::Idle {
            self.set_pointer_mode(PointerMode::DraggingExternal);
        }
        vec![]
    }

    pub(crate) fn handle_drag_leave(&mut self) -> Vec<Effect> {
        if self.pointer.is_dragging_external() {
            self.set_pointer_mode(PointerMode::Idle);
        }
        self.pointer.drop_target = None;
        vec![Effect::DropTargetChanged { target: None }]
    }

    pub(crate) fn handle_drag_over(&mut self, page_idx: usize, x: f32, y: f32) -> Vec<Effect> {
        let is_dragging = self.pointer.is_dragging_content() || self.pointer.is_dragging_external();

        if !is_dragging {
            return vec![];
        }

        let Some(page) = self.pages.get(page_idx) else {
            self.pointer.drop_target = None;
            return vec![Effect::DropTargetChanged { target: None }];
        };

        let ctx = NavigationContext::new(&self.state.doc);
        let Some(hit_selection) = Cursor::hit_test_dnd(&ctx, page, x, y) else {
            self.pointer.drop_target = None;
            return vec![Effect::DropTargetChanged { target: None }];
        };

        let position = hit_selection.head;

        if self.pointer.is_dragging_content() {
            let can_drop = {
                let tr = Transaction::new(&self.state);
                tr.can_drop(position)
            };

            if !can_drop {
                self.pointer.drop_target = None;
                return vec![Effect::DropTargetChanged { target: None }];
            }
        }
        // TODO: 외부로부터의 DND도 드랍 못 하는 곳엔 인디케이터 띄우면 안 됨

        self.pointer.drop_target = Some(position);

        vec![Effect::DropTargetChanged {
            target: Some(position),
        }]
    }

    fn resolve_drop_position(&mut self, page_idx: usize, x: f32, y: f32) -> Option<Position> {
        if let Some(pos) = self.pointer.drop_target.take() {
            return Some(pos);
        }

        let page = self.pages.get(page_idx)?;
        let ctx = NavigationContext::new(&self.state.doc);
        let hit_selection = Cursor::hit_test_dnd(&ctx, page, x, y)?;
        Some(hit_selection.head)
    }

    pub(crate) fn handle_drop(
        &mut self,
        page_idx: usize,
        x: f32,
        y: f32,
        text: Option<String>,
        _html: Option<String>,
        fragment: Option<String>,
        modifier: Modifier,
    ) -> Vec<Effect> {
        let Some(drop_position) = self.resolve_drop_position(page_idx, x, y) else {
            return self.handle_drag_end_internal();
        };

        let is_internal_drag = self.pointer.is_dragging_content();

        self.pointer.drop_target = None;
        self.set_pointer_mode(PointerMode::Idle);

        let mut effects = if is_internal_drag {
            if modifier.alt {
                self.transact(move |tr| tr.drag_and_copy(drop_position))
            } else {
                self.transact(move |tr| tr.drag_and_drop(drop_position))
            }
        } else if let Some(text) = text {
            self.transact(move |tr| {
                let fragment = if let Some(json) = fragment {
                    Fragment::from_json(&json).ok()
                } else {
                    None
                };
                let fragment = fragment.unwrap_or_else(|| Fragment::from_text(&text));
                tr.drop_external(drop_position, fragment)
            })
        } else {
            return self.handle_drag_end_internal();
        };

        if effects.is_empty() {
            return self.handle_drag_end_internal();
        }

        effects.push(Effect::DropTargetChanged { target: None });
        effects
    }

    pub(crate) fn handle_drop_images(
        &mut self,
        page_idx: usize,
        x: f32,
        y: f32,
        upload_ids: Vec<String>,
    ) -> Vec<Effect> {
        let Some(drop_position) = self.resolve_drop_position(page_idx, x, y) else {
            return self.handle_drag_end_internal();
        };

        self.pointer.drop_target = None;
        self.set_pointer_mode(PointerMode::Idle);

        let mut effects = self.transact(|tr| {
            tr.set_selection(Selection::collapsed(drop_position));
            for upload_id in upload_ids {
                tr.insert_node(crate::model::Node::Image(crate::model::ImageNode {
                    id: None,
                    proportion: 1.0,
                    upload_id: Some(upload_id),
                }))?;
            }
            Ok(true)
        });
        effects.push(Effect::DropTargetChanged { target: None });
        effects
    }

    pub(crate) fn handle_drop_files(
        &mut self,
        page_idx: usize,
        x: f32,
        y: f32,
        upload_ids: Vec<String>,
    ) -> Vec<Effect> {
        let Some(drop_position) = self.resolve_drop_position(page_idx, x, y) else {
            return self.handle_drag_end_internal();
        };

        self.pointer.drop_target = None;
        self.set_pointer_mode(PointerMode::Idle);

        let mut effects = self.transact(|tr| {
            tr.set_selection(Selection::collapsed(drop_position));
            for upload_id in upload_ids {
                tr.insert_node(crate::model::Node::File(crate::model::FileNode {
                    id: None,
                    upload_id: Some(upload_id),
                }))?;
            }
            Ok(true)
        });
        effects.push(Effect::DropTargetChanged { target: None });
        effects
    }

    pub(crate) fn handle_drag_end(&mut self) -> Vec<Effect> {
        self.handle_drag_end_internal()
    }

    fn handle_drag_end_internal(&mut self) -> Vec<Effect> {
        self.set_pointer_mode(PointerMode::Idle);
        self.pointer.drop_target = None;
        vec![Effect::DropTargetChanged { target: None }]
    }
}

#[cfg(test)]
mod tests {
    use crate::layout::cursor::{Cursor, NavigationContext};

    use crate::model::NodeId;
    use crate::runtime::DropIndicator;
    use crate::runtime::message::{Modifier, PointerButton};
    use crate::state::Position;
    use crate::types::Affinity;

    fn find_position_coordinates(
        rt: &mut crate::runtime::Runtime,
        position: Position,
    ) -> (usize, f32, f32) {
        let ctx = NavigationContext::new(&rt.state.doc);
        let (page_idx, rect) =
            Cursor::bounds(&ctx, &rt.pages, position).expect("Bounds should be found for position");
        (
            page_idx,
            rect.x + rect.width / 2.0,
            rect.y + rect.height / 2.0,
        )
    }

    fn find_gap_coordinates(
        rt: &mut crate::runtime::Runtime,
        position: Position,
    ) -> (usize, f32, f32) {
        let ctx = NavigationContext::new(&rt.state.doc);
        let drop_indicator = DropIndicator::from_position(&ctx, &rt.pages, position)
            .expect("Drop indicator should be found for position");
        match drop_indicator {
            DropIndicator::Inline {
                page_idx,
                x,
                y,
                height,
                ..
            } => (page_idx, x, y + height / 2.0),
            DropIndicator::Block {
                page_idx,
                x,
                y,
                width,
                ..
            } => {
                let doc = &rt.state.doc;
                let node = doc.node(position.node_id).expect("Node not found");
                let child_count = node.children().count();

                let y_adjusted = if position.offset == child_count {
                    y + 1.0
                } else {
                    y - 1.0
                };

                (page_idx, x + width / 2.0, y_adjusted)
            }
        }
    }

    fn drag_and_drop(rt: &mut crate::runtime::Runtime, from: Position, to: Position) {
        drag_and_drop_with_modifier(rt, from, to, Modifier::default());
    }

    fn drag_and_drop_with_modifier(
        rt: &mut crate::runtime::Runtime,
        from: Position,
        to: Position,
        modifier: Modifier,
    ) {
        let (from_page, from_x, from_y) = find_position_coordinates(rt, from);
        let (to_page, to_x, to_y) = find_position_coordinates(rt, to);

        rt.update(crate::runtime::Message::PointerDown {
            page_idx: from_page,
            x: from_x,
            y: from_y,
            click_count: 1,
            button: PointerButton::Primary,
            modifier: Modifier::default(),
        });

        rt.update(crate::runtime::Message::PointerMove {
            page_idx: from_page,
            x: from_x,
            y: from_y,
            buttons: 1,
            modifier: Modifier::default(),
        });

        rt.update(crate::runtime::Message::DragStart {
            page_idx: from_page,
            x: from_x,
            y: from_y,
        });

        rt.update(crate::runtime::Message::DragEnter);

        rt.update(crate::runtime::Message::DragOver {
            page_idx: to_page,
            x: to_x,
            y: to_y,
        });

        rt.update(crate::runtime::Message::Drop {
            page_idx: to_page,
            x: to_x,
            y: to_y,
            text: None,
            html: None,
            fragment: None,
            modifier,
        });
    }

    fn drag_and_drop_to_gap(rt: &mut crate::runtime::Runtime, from: Position, to: Position) {
        let (from_page, from_x, from_y) = find_position_coordinates(rt, from);
        let (to_page, to_x, to_y) = find_gap_coordinates(rt, to);

        rt.update(crate::runtime::Message::PointerDown {
            page_idx: from_page,
            x: from_x,
            y: from_y,
            click_count: 1,
            button: PointerButton::Primary,
            modifier: Modifier::default(),
        });

        rt.update(crate::runtime::Message::PointerMove {
            page_idx: from_page,
            x: from_x,
            y: from_y,
            buttons: 1,
            modifier: Modifier::default(),
        });

        rt.update(crate::runtime::Message::DragStart {
            page_idx: from_page,
            x: from_x,
            y: from_y,
        });

        rt.update(crate::runtime::Message::DragEnter);

        rt.update(crate::runtime::Message::DragOver {
            page_idx: to_page,
            x: to_x,
            y: to_y,
        });

        rt.update(crate::runtime::Message::Drop {
            page_idx: to_page,
            x: to_x,
            y: to_y,
            text: None,
            html: None,
            fragment: None,
            modifier: Modifier::default(),
        });
    }

    fn drag_and_drop_external(
        rt: &mut crate::runtime::Runtime,
        to: Position,
        text: Option<String>,
        html: Option<String>,
        fragment: Option<String>,
    ) {
        let (to_page, to_x, to_y) = find_gap_coordinates(rt, to);

        rt.update(crate::runtime::Message::DragEnter);
        rt.update(crate::runtime::Message::DragOver {
            page_idx: to_page,
            x: to_x,
            y: to_y,
        });

        rt.update(crate::runtime::Message::Drop {
            page_idx: to_page,
            x: to_x,
            y: to_y,
            text,
            html,
            fragment,
            modifier: Modifier::default(),
        });
    }

    #[test]
    fn test_dnd_preserves_selection_inline_move() {
        let mut p1 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "Hello World" } }
            }
            selection { (p1, 6) -> (p1, 11) } // Select "World"
        };
        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p1, 8, Affinity::default()),
            Position::new(p1, 0, Affinity::default()),
        );

        let expected = state! {
            doc {
                @p1 paragraph { text { "WorldHello " } }
            }
            selection { (p1, 0) -> (p1, 5) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_alt_drag_copies_instead_of_moving() {
        let mut p1 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "Hello World" } }
            }
            selection { (p1, 6) -> (p1, 11) }
        };
        rt.layout();

        drag_and_drop_with_modifier(
            &mut rt,
            Position::new(p1, 8, Affinity::default()),
            Position::new(p1, 0, Affinity::default()),
            Modifier {
                alt: true,
                ..Default::default()
            },
        );

        let expected = state! {
            doc {
                @p1 paragraph { text { "WorldHello World" } }
            }
            selection { (p1, 0) -> (p1, 5) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_cross_block_selection_inline_move() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "Start" } }
                @p2 paragraph { text { "End" } }
                @p3 paragraph { text { "Target" } }
            }
            selection { (p1, 2) -> (p2, 2) }
        };
        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p1, 3, Affinity::default()),
            Position::new(p3, 0, Affinity::default()),
        );
        let doc = rt.doc();

        let p3_node = doc.node(p3).expect("p3 should exist");
        let next_node = p3_node.next_sibling().expect("Should have next sibling");
        let mut next_id = next_node.node_id();

        let expected = state! {
            doc {
                @p1 paragraph { text { "Std" } }
                @p3 paragraph { text { "art" } }
                @next_id paragraph { text { "EnTarget" } }
            }
            selection { (p3, 0) -> (next_id, 2) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_inline_text_to_block_gap() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "Hello" } }
                @p2 paragraph { text { "World" } }
            }
            selection { (p1, 0) -> (p1, 5) }
        };
        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p1, 2, Affinity::default()),
            Position::new(NodeId::ROOT, 1, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph { }
                @p1 paragraph { text { "Hello" } }
                @p2 paragraph { text { "World" } }
            }
            selection { (p1, 0) -> (p1, 5) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_cross_paragraph_selection_to_block_gap() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "Hello" } }
                @p2 paragraph { text { "World" } }
                paragraph { text { "Target" } }
            }
            selection { (p1, 2) -> (p2, 3) } // "llo" + "Wor"
        };
        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p1, 3, Affinity::default()),
            Position::new(NodeId::ROOT, 2, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph { text { "Held" } }
                @p1 paragraph { text { "llo" } }
                @p2 paragraph { text { "Wor" } }
                paragraph { text { "Target" } }
            }
            selection { (p1, 0) -> (p2, 3) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_selectable_node_immediate_drag() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut img = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "Before" } }
                @img image (id: Some("test-image-id".to_string()),) {}
                @p2 paragraph { text { "After" } }
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
        };
        rt.layout();

        // Drag image to end (position 3)
        drag_and_drop_to_gap(
            &mut rt,
            Position::new(NodeId::ROOT, 1, Affinity::default()),
            Position::new(NodeId::ROOT, 3, Affinity::default()),
        );

        // Result: p1("Before"), p2("After"), Image, (trailing empty paragraph)
        // The trailing paragraph is added by ensure_trailing_paragraph since
        // the document can't end with an Image
        let expected = state! {
            doc {
                @p1 paragraph { text { "Before" } }
                @p2 paragraph { text { "After" } }
                @img image (id: Some("test-image-id".to_string()),) {}
                paragraph { }
            }
            selection { (NodeId::ROOT, 2) -> (NodeId::ROOT, 3, Affinity::Upstream) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_click_selectable_node_selects_it() {
        let mut p1 = id!();
        let mut img = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "Before" } }
                @img image (id: Some("test-image-id".to_string()),) {}
                @p2 paragraph { text { "After" } }
            }
            selection { (p1, 0) }
        };
        rt.layout();

        let (img_x, img_y) = {
            let ctx = NavigationContext::new(&rt.state.doc);
            let bounds = crate::layout::query::find_node_bounds(ctx.doc, &rt.pages, img)
                .expect("Image bounds should exist");
            (
                bounds.x + bounds.width / 2.0,
                bounds.y + bounds.height / 2.0,
            )
        };

        rt.update(crate::runtime::Message::PointerDown {
            page_idx: 0,
            x: img_x,
            y: img_y,
            click_count: 1,
            button: PointerButton::Primary,
            modifier: Modifier::default(),
        });

        rt.update(crate::runtime::Message::PointerUp {
            page_idx: 0,
            x: img_x,
            y: img_y,
            button: PointerButton::Primary,
            modifier: Modifier::default(),
        });

        let expected = state! {
            doc {
                @p1 paragraph { text { "Before" } }
                @img image (id: Some("test-image-id".to_string()),) {}
                @p2 paragraph { text { "After" } }
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2, Affinity::Upstream) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_image_to_inline_position_splits_paragraph() {
        let mut p1 = id!();
        let mut img = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "Hello World" } }
                @img image (id: Some("test-image-id".to_string()),) {}
                paragraph {}
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
        };
        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(NodeId::ROOT, 1, Affinity::default()),
            Position::new(p1, 6, Affinity::default()),
        );

        let expected = state! {
            doc {
                @p1 paragraph { text { "Hello " } }
                image (id: Some("test-image-id".to_string()),) {}
                paragraph { text { "World" } }
                paragraph { }
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2, Affinity::Upstream) }

        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_mixed_selection_to_block_gap() {
        let mut p = id!();
        let mut img = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @img image (id: Some("test-image-id".to_string()),) {}
                @p paragraph { text { "Hello" } }
            }
            selection { (NodeId::ROOT, 0) -> (p, 2) }
        };
        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p, 0, Affinity::default()),
            Position::new(NodeId::ROOT, 2, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph { text { "llo" } }
                image (id: Some("test-image-id".to_string()),) {}
                @p paragraph { text { "He" } }
            }
            selection { (NodeId::ROOT, 1) -> (p, 2) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_mixed_selection_to_inline_gap() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut img = id!();
        let mut last_p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @img image (id: Some("test-image-id".to_string()),) {}
                @p1 paragraph { text { "Hello" } }
                @p2 paragraph { text { "Target" } }
            }
            selection { (NodeId::ROOT, 0) -> (p1, 2) }
        };
        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p1, 0, Affinity::default()),
            Position::new(p2, 3, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph { text { "llo" } }
                @p2 paragraph { text { "Tar" } }
                image (id: Some("test-image-id".to_string()),) {}
                @last_p paragraph { text { "Heget" } }
            }
            selection { (NodeId::ROOT, 2) -> (last_p, 2) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_selection_drop_order_bug() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();
        let mut p4 = id!();
        let mut img = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "1" } }
                @p2 paragraph { text { "2" } }
                @p3 paragraph { text { "3" } }
                @img image (id: Some("test-image-id".to_string()),) {}
                @p4 paragraph { text { "4" } }
            }
            selection { (p3, 0) -> (p4, 1) }
        };
        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p3, 0, Affinity::default()),
            Position::new(NodeId::ROOT, 1, Affinity::default()),
        );

        let expected = state! {
            doc {
                @p1 paragraph { text { "1" } }
                @p3 paragraph { text { "3" } }
                image (id: Some("test-image-id".to_string()),) {}
                @p4 paragraph { text { "4" } }
                @p2 paragraph { text { "2" } }
                paragraph {}
            }
            selection { (p3, 0) -> (p4, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_selection_with_image_inline_drop() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();
        let mut p4 = id!();
        let mut img = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "1" } }
                @img image (id: Some("test-image-id".to_string()),) {}
                @p2 paragraph { text { "2" } }
                @p3 paragraph { text { "3" } }
                @p4 paragraph { text { "4" } }
            }
            selection { (p1, 0) -> (p2, 1) }
        };
        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p1, 1, Affinity::default()),
            Position::new(p4, 1, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph {}
                @p3 paragraph { text { "3" } }
                @p1 paragraph { text { "41" } }
                image (id: Some("test-image-id".to_string()),) {}
                @p2 paragraph { text { "2" } }
            }
            selection { (p1, 1) -> (p2, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_selection_with_blockquote_block_gap_drop_over_image() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();
        let mut p4 = id!();
        let mut bq = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "1" } }
                @bq blockquote { paragraph { text { "bq" } } }
                @p2 paragraph { text { "2" } }
                image (id: Some("test-image-id".to_string()),) {}
                @p3 paragraph { text { "3" } }
                @p4 paragraph { text { "4" } }
            }
            selection { (p1, 0) -> (p2, 1) }
        };
        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p1, 1, Affinity::default()),
            Position::new(NodeId::ROOT, 4, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph {}
                image (id: Some("test-image-id".to_string()),) {}
                @p1 paragraph { text { "1" } }
                @bq blockquote { paragraph { text { "bq" } } }
                @p2 paragraph { text { "2" } }
                @p3 paragraph { text { "3" } }
                @p4 paragraph { text { "4" } }
            }
            selection { (p1, 0) -> (p2, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_selection_with_blockquote_inline_drop() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();
        let mut p4 = id!();
        let mut bq = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "1" } }
                @bq blockquote { paragraph { text { "bq" } } }
                @p2 paragraph { text { "2" } }
                @p3 paragraph { text { "3" } }
                @p4 paragraph { text { "4" } }
            }
            selection { (p1, 0) -> (p2, 1) }
        };
        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p1, 1, Affinity::default()),
            Position::new(p4, 1, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph {}
                @p3 paragraph { text { "3" } }
                @p1 paragraph { text { "41" } }
                @bq blockquote { paragraph { text { "bq" } } }
                @p2 paragraph { text { "2" } }
            }
            selection { (p1, 1) -> (p2, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_external_text_to_block_gap() {
        let mut p = id!();
        let mut n = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph { text { "1" } }
                paragraph { text { "2" } }
            }
            selection { (p, 0) }
        };
        rt.layout();

        drag_and_drop_external(
            &mut rt,
            Position::new(NodeId::ROOT, 1, Affinity::default()),
            Some("New".to_string()),
            None,
            None,
        );

        let expected = state! {
            doc {
                paragraph { text { "1" } }
                @n paragraph { text { "New" } }
                paragraph { text { "2" } }
            }
            selection { (n, 0) -> (n, 3) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_selection_with_blockquote_inline_drop_2() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p4 = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph {
                    text { "11" }
                }
                blockquote {
                    paragraph {
                        text { "22" }
                    }
                }
                @p2 paragraph {
                    text { "33" }
                }
                @p4 paragraph {
                    text { "44" }
                }
            }
            selection { (p1, 1) -> (p2, 1) }
        };

        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p1, 2, Affinity::default()),
            Position::new(p4, 1, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph {
                    text { "13" }
                }
                @p1 paragraph {
                    text { "41" }
                }
                blockquote {
                    paragraph {
                        text { "22" }
                    }
                }
                @p2 paragraph {
                    text { "34" }
                }
            }
            selection { (p1, 1) -> (p2, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_selection_with_hard_break() {
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "1" }
                    hard_break {}
                    text { "2" }
                }
            }
            selection { (p, 0) -> (p, 3) }
        };
        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p, 1, Affinity::default()),
            Position::new(NodeId::ROOT, 1, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph { }
                @p paragraph {
                    text { "1" }
                    hard_break {}
                    text { "2" }
                }
            }
            selection { (p, 0) -> (p, 3) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_selectable_node_within_selection_preserves_selection() {
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph { text { "Hello" } }
                image (id: Some("test-image-id".to_string()),) {}
                paragraph { text { "World" } }
            }
            selection { (p, 0) -> (NodeId::ROOT, 2) }
        };
        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(NodeId::ROOT, 1, Affinity::default()),
            Position::new(NodeId::ROOT, 3, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph {}
                paragraph { text { "World" } }
                @p paragraph { text { "Hello" } }
                image (id: Some("test-image-id".to_string()),) {}
                paragraph { }
            }
            selection { (p, 0) -> (NodeId::ROOT, 4) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_page_break_to_inline_position_splits_paragraph() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph {
                    text { "Hello" }
                    page_break {}
                }
                @p2 paragraph { text { "World" } }
            }
            selection { (p1, 5) -> (p1, 6) }
        };
        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p1, 5, Affinity::default()),
            Position::new(p2, 3, Affinity::default()),
        );

        let mut pb_para = id!();
        let expected = state! {
            doc {
                @p1 paragraph { text { "Hello" } }
                @pb_para paragraph {
                    text { "Wor" }
                    page_break {}
                }
                @p2 paragraph { text { "ld" } }
            }
            selection { (pb_para, 3) -> (p2, 0) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_page_break_to_paragraph_end_no_split() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph {
                    text { "Hello" }
                    page_break {}
                }
                @p2 paragraph { text { "World" } }
            }
            selection { (p1, 5) -> (p1, 6) }
        };
        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p1, 5, Affinity::default()),
            Position::new(p2, 5, Affinity::default()),
        );

        let mut p3 = id!();
        let expected = state! {
            doc {
                @p1 paragraph { text { "Hello" } }
                @p2 paragraph {
                    text { "World" }
                    page_break {}
                }
                @p3 paragraph { }
            }
            selection { (p2, 5) -> (p3, 0) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_page_break_to_nested_position_rejected() {
        let mut p = id!();
        let mut bq_p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "Hello" }
                    page_break {}
                }
                blockquote {
                    @bq_p paragraph { text { "Nested" } }
                }
            }
            selection { (p, 5) -> (p, 6) }
        };
        rt.layout();
        drag_and_drop(
            &mut rt,
            Position::new(p, 5, Affinity::default()),
            Position::new(bq_p, 3, Affinity::default()),
        );

        let expected = state! {
            doc {
                @p paragraph {
                    text { "Hello" }
                    page_break {}
                }
                blockquote {
                    @bq_p paragraph { text { "Nested" } }
                }
            }
            selection { (p, 5) -> (p, 6) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_page_break_to_root_block_gap_success() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph {
                    text { "Hello" }
                    page_break {}
                }
                @p2 paragraph { text { "World" } }
            }
            selection { (p1, 5) -> (p1, 6) }
        };
        rt.layout();
        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p1, 5, Affinity::default()),
            Position::new(NodeId::ROOT, 2, Affinity::default()),
        );

        let mut pb_para = id!();
        let expected = state! {
            doc {
                @p1 paragraph { text { "Hello" } }
                @p2 paragraph { text { "World" } }
                @pb_para paragraph { page_break {} }
                paragraph { }
            }
            selection { (pb_para, 0) -> (pb_para, 1) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_text_and_page_break_together() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph {
                    text { "Hello" }
                    page_break {}
                }
                @p2 paragraph { text { "World" } }
            }
            selection { (p1, 3) -> (p1, 6) }
        };
        rt.layout();
        drag_and_drop(
            &mut rt,
            Position::new(p1, 3, Affinity::default()),
            Position::new(p2, 3, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph { text { "Hel" } }
                @p1 paragraph {
                    text { "Worlo" }
                    page_break {}
                }
                @p2 paragraph { text { "ld" } }
            }
            selection { (p1, 3) -> (p2, 0) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_selection_after_dnd() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph {
                    text { "11" }
                }
                @p2 paragraph {
                    text { "22" }
                }
                @p3 paragraph {
                    text { "33" }
                }
            }
            selection { (p2, 1) -> (p3, 1) }
        };

        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p2, 2, Affinity::default()),
            Position::new(p1, 1, Affinity::default()),
        );

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "12" }
                }
                @p2 paragraph {
                    text { "31" }
                }
                @p3 paragraph {
                    text { "23" }
                }
            }
            selection { (p1, 1) -> (p2, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_selection_after_dnd_to_gap() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "11" } }
                @p2 paragraph { text { "22" } }
                @p3 paragraph { text { "33" } }
            }
            selection { (p2, 1) -> (p3, 1) }
        };

        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p2, 2, Affinity::default()),
            Position::new(NodeId::ROOT, 1, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph { text { "11" } }
                @p2 paragraph { text { "2" } }
                @p3 paragraph { text { "3" } }
                paragraph { text { "23" } }
            }
            selection { (p2, 0) -> (p3, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_selection_after_dnd_to_gap_2() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "11" } }
                @p2 paragraph { text { "22" } }
                @p3 paragraph { text { "33" } }
            }
            selection { (p2, 1) -> (p3, 1) }
        };

        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p2, 2, Affinity::default()),
            Position::new(NodeId::ROOT, 3, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph { text { "11" } }
                paragraph { text { "23" } }
                @p2 paragraph { text { "2" } }
                @p3 paragraph { text { "3" } }
            }
            selection { (p2, 0) -> (p3, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_with_list() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "11" } }
                bullet_list { list_item { paragraph { text { "22" } } } }
                @p2 paragraph { text { "3333" } }
            }
            selection { (p1, 1) -> (p2, 1) }
        };

        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p1, 2, Affinity::default()),
            Position::new(p2, 3, Affinity::default()),
        );

        let expected = state! {
            doc {
                @p1 paragraph { text { "1331" } }
                bullet_list { list_item { paragraph { text { "22" } } } }
                @p2 paragraph { text { "33" } }
            }
            selection { (p1, 3) -> (p2, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_merge_blockquotes() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                blockquote { @p1 paragraph { text { "AA" } } }
                blockquote { @p2 paragraph { text { "BB" } } }
            }
            selection { (p1, 1) -> (p2, 1) }
        };

        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p1, 2, Affinity::default()),
            Position::new(p1, 0, Affinity::default()),
        );

        let expected = state! {
            doc {
                blockquote {
                    @p1 paragraph { text { "A" } }
                    @p2 paragraph { text { "BAB" } }
                }
                paragraph { }
            }
            selection { (p1, 0) -> (p2, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_blockquote_with_paragraph_to_gap() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                blockquote { @p1 paragraph { text { "AA" } } }
                @p2 paragraph { text { "BB" } }
            }
            selection { (p1, 1) -> (p2, 1) }
        };

        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p1, 2, Affinity::default()),
            Position::new(NodeId::ROOT, 0, Affinity::default()),
        );

        let expected = state! {
            doc {
                blockquote { @p1 paragraph { text { "A" } } }
                @p2 paragraph { text { "B" } }
                blockquote { paragraph { text { "AB" } } }
                paragraph { }
            }
            selection { (p1, 0) -> (p2, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_paragraph_with_blockquote_to_gap() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "AA" } }
                blockquote { @p2 paragraph { text { "BB" } } }
                paragraph {}
            }
            selection { (p1, 1) -> (p2, 1) }
        };

        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p1, 2, Affinity::default()),
            Position::new(NodeId::ROOT, 0, Affinity::default()),
        );

        let expected = state! {
            doc {
                @p1 paragraph { text { "A" } }
                blockquote { @p2 paragraph { text { "B" } } }
                paragraph { text { "AB" } }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_whole_paragraph_with_blockquote_to_gap() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "A" } }
                blockquote { @p2 paragraph { text { "B" } } }
                paragraph { text { "C" } }
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p1, 1, Affinity::default()),
            Position::new(NodeId::ROOT, 3, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph {}
                paragraph { text { "C" } }
                @p1 paragraph { text { "A" } }
                blockquote { @p2 paragraph { text { "B" } } }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };
        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_blockquote_internal_block_selection_reorders() {
        let mut bq = id!();
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @bq blockquote {
                    @p1 paragraph { text { "A" } }
                    @p2 paragraph { text { "B" } }
                    paragraph { text { "C" } }
                    paragraph { text { "D" } }
                }
                paragraph { text { "Tail" } }
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        rt.layout();

        drag_and_drop_to_gap(
            &mut rt,
            Position::new(p2, 0, Affinity::default()),
            Position::new(bq, 3, Affinity::default()),
        );

        let expected = state! {
            doc {
                blockquote {
                    paragraph {} // TODO: <- 이 empty paragraph 없어졌으면 좋겠음
                    paragraph { text { "C" } }
                    @p1 paragraph { text { "A" } }
                    @p2 paragraph { text { "B" } }
                    paragraph { text { "D" } }
                }
                paragraph { text { "Tail" } }
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_blockquote_paragraph_selection_to_empty_paragraph() {
        let mut bq_p = id!();
        let mut p2 = id!();
        let mut target = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                blockquote {
                    @bq_p paragraph { text { "AA" } }
                }
                @p2 paragraph { text { "BB" } }
                @target paragraph {}
            }
            selection { (bq_p, 0) -> (p2, 2) }
        };

        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(bq_p, 1, Affinity::default()),
            Position::new(target, 0, Affinity::default()),
        );

        let expected = state! {
            doc {
                paragraph {}
                blockquote {
                    @bq_p paragraph { text { "AA" } }
                }
                @p2 paragraph { text { "BB" } }
            }
            selection { (bq_p, 0) -> (p2, 2) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_inline_to_same_line_end() {
        let mut p1 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "AABB" } }
            }
            selection { (p1, 0) -> (p1, 2) }
        };

        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p1, 1, Affinity::default()),
            Position::new(p1, 4, Affinity::default()),
        );

        let expected = state! {
            doc {
                @p1 paragraph { text { "BBAA" } }
            }
            selection { (p1, 2) -> (p1, 4) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn test_dnd_inline_to_between_hard_breaks() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph {
                    text { "1111" }
                    hard_break {}
                    hard_break {}
                    text { "3333" }
                }
                @p2 paragraph { text { "2222" } }
            }
            selection { (p2, 0) -> (p2, 4) }
        };

        rt.layout();

        drag_and_drop(
            &mut rt,
            Position::new(p2, 2, Affinity::default()),
            Position::new(p1, 5, Affinity::default()),
        );

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "1111" }
                    hard_break {}
                    text { "2222" }
                    hard_break {}
                    text { "3333" }
                }
                paragraph {}
            }
            selection { (p1, 5) -> (p1, 9) }
        };

        assert_state_eq!(rt.state(), expected);
    }
}
