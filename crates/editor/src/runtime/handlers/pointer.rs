use crate::layout::cursor::{Cursor, NavigationContext};
use crate::runtime::message::{Modifier, PointerButton};
use crate::runtime::pointer::{PointerMode, PressContext};
use crate::runtime::{Effect, Runtime};
use crate::state::position_helpers::compare_positions;
use crate::state::{Position, Selection};
use std::cmp::Ordering;

impl Runtime {
    pub(crate) fn set_pointer_mode(&mut self, mode: PointerMode) {
        if self.pointer.mode != mode {
            self.pointer.mode = mode;
            self.pending.pointer_mode_changed = true;
        }
    }

    pub(crate) fn reset_pointer(&mut self) {
        if !self.pointer.mode.is_idle() {
            self.pending.pointer_mode_changed = true;
        }
        self.pointer.reset();
    }

    pub(crate) fn handle_pointer_down(
        &mut self,
        page_idx: usize,
        x: f32,
        y: f32,
        click_count: u32,
        button: PointerButton,
        modifier: Modifier,
    ) -> Vec<Effect> {
        let Some(page) = self.pages().get(page_idx) else {
            return vec![];
        };

        if button.is_primary() && !modifier.shift {
            if let Some(kind) = page.find_interactive_at(x, y, self.is_read_only()) {
                let should_interact = if self.is_read_only() {
                    kind.allow_in_read_only()
                } else {
                    true
                };

                if should_interact {
                    self.set_pointer_mode(PointerMode::Pressed {
                        page_idx,
                        start_x: x,
                        start_y: y,
                        document_position: self.state.selection.head,
                        context: PressContext::Interactive(kind),
                    });
                    return vec![];
                }
            }
        }

        let ctx = NavigationContext::new(&self.state.doc);
        let Some(hit_selection) = Cursor::hit_test(&ctx, page, x, y) else {
            return vec![];
        };

        let position = hit_selection.head;

        if !button.is_primary() {
            return self.handle_secondary_pointer_down(hit_selection, position);
        }

        if modifier.shift {
            return self.handle_shift_click(hit_selection);
        }

        if let Some(effects) = self.handle_multi_click(click_count, position, hit_selection) {
            return effects;
        }

        self.handle_single_click(page_idx, x, y, position, hit_selection)
    }

    fn handle_shift_click(&mut self, hit_selection: Selection) -> Vec<Effect> {
        self.set_pointer_mode(PointerMode::DraggingSelection);

        let extended_selection = self
            .state
            .selection
            .extend_to(&self.state.doc, hit_selection);

        self.transact(move |tr| {
            tr.set_selection(extended_selection);
            tr.set_preferred_x(None);
            Ok(true)
        })
    }

    fn handle_multi_click(
        &mut self,
        click_count: u32,
        position: Position,
        hit_selection: Selection,
    ) -> Option<Vec<Effect>> {
        if click_count <= 1 {
            return None;
        }

        self.set_pointer_mode(PointerMode::Idle);

        if click_count == 2 && self.is_read_only() && self.is_block_selectable_hit(&hit_selection) {
            return Some(self.transact(move |tr| {
                tr.set_selection(hit_selection);
                tr.set_preferred_x(None);
                Ok(true)
            }));
        }

        let effects = match click_count {
            2 => self.transact(move |tr| {
                tr.select_word_at(position)?;
                tr.set_preferred_x(None);
                Ok(true)
            }),
            3 => self.transact(move |tr| {
                tr.select_paragraph_at(position)?;
                tr.set_preferred_x(None);
                Ok(true)
            }),
            _ => return None,
        };

        Some(effects)
    }

    fn handle_single_click(
        &mut self,
        page_idx: usize,
        x: f32,
        y: f32,
        position: Position,
        hit_selection: Selection,
    ) -> Vec<Effect> {
        let context = self.determine_press_context(position, &hit_selection);
        let should_update_selection_on_down = match context {
            PressContext::InSelection => false,
            PressContext::OnSelectable(_) => {
                !self.is_read_only() && !self.is_position_in_selection(position)
            }
            _ => true,
        };

        self.set_pointer_mode(PointerMode::Pressed {
            page_idx,
            start_x: x,
            start_y: y,
            document_position: position,
            context,
        });

        if should_update_selection_on_down {
            self.transact(move |tr| {
                tr.set_selection(hit_selection);
                tr.set_preferred_x(None);
                Ok(true)
            })
        } else {
            vec![]
        }
    }

    fn determine_press_context(
        &self,
        position: Position,
        hit_selection: &Selection,
    ) -> PressContext {
        if self.is_block_selectable_hit(hit_selection) {
            PressContext::OnSelectable(*hit_selection)
        } else if self.is_position_in_selection(position) {
            PressContext::InSelection
        } else {
            PressContext::Empty
        }
    }

    fn handle_secondary_pointer_down(
        &mut self,
        hit_selection: Selection,
        position: Position,
    ) -> Vec<Effect> {
        let selection = self.state.selection;

        if selection.is_collapsed() {
            if hit_selection.is_collapsed() {
                self.transact(move |tr| {
                    tr.set_selection(Selection::collapsed(position));
                    tr.set_preferred_x(None);
                    Ok(true)
                })
            } else {
                self.transact(move |tr| {
                    tr.set_selection(hit_selection);
                    tr.set_preferred_x(None);
                    Ok(true)
                })
            }
        } else if hit_selection.is_collapsed() {
            if self.is_position_in_range(position) {
                vec![]
            } else {
                self.transact(move |tr| {
                    tr.set_selection(Selection::collapsed(position));
                    tr.set_preferred_x(None);
                    Ok(true)
                })
            }
        } else {
            vec![]
        }
    }

    fn is_position_in_range(&self, position: Position) -> bool {
        let selection = self.state.selection;
        let Ok((from, to)) = selection.as_sorted(&self.state.doc) else {
            return false;
        };

        matches!(
            (
                compare_positions(&self.state.doc, from, position),
                compare_positions(&self.state.doc, position, to),
            ),
            (
                Ok(std::cmp::Ordering::Less | std::cmp::Ordering::Equal),
                Ok(std::cmp::Ordering::Less | std::cmp::Ordering::Equal),
            )
        )
    }

    pub(crate) fn handle_pointer_move(
        &mut self,
        page_idx: usize,
        x: f32,
        y: f32,
        buttons: u16,
        _modifier: Modifier,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();

        let style = self.get_pointer_style(page_idx, x, y);
        effects.push(Effect::PointerStyleChanged { style });

        if buttons == 0 {
            self.reset_pointer();
            return effects;
        }

        match &self.pointer.mode {
            PointerMode::Pressed {
                start_x,
                start_y,
                context,
                ..
            } => {
                const DRAG_THRESHOLD: f32 = 5.0;
                let dx = x - *start_x;
                let dy = y - *start_y;
                if (dx * dx + dy * dy).sqrt() >= DRAG_THRESHOLD {
                    if !context.can_drag_content() {
                        self.set_pointer_mode(PointerMode::DraggingSelection);
                        effects.extend(self.handle_selection_drag(page_idx, x, y));
                    }
                }
            }
            PointerMode::DraggingContent | PointerMode::DraggingExternal => {
                // DND는 DragOver 메시지에서 처리
            }
            PointerMode::DraggingSelection => {
                effects.extend(self.handle_selection_drag(page_idx, x, y));
            }
            PointerMode::Idle => {}
        }

        effects
    }

    fn handle_selection_drag(&mut self, page_idx: usize, x: f32, y: f32) -> Vec<Effect> {
        let Some(page) = self.pages().get(page_idx) else {
            return vec![];
        };

        let ctx = NavigationContext::new(&self.state.doc);
        let Some(hit_selection) = Cursor::hit_test_drag(&ctx, page, x, y) else {
            return vec![];
        };

        let selection = self.state.selection;
        let anchor = selection.anchor;
        let position = hit_selection.head;

        let new_head = if hit_selection.is_collapsed() {
            position
        } else {
            let Ok((from, to)) = hit_selection.as_sorted(&self.state.doc) else {
                return vec![];
            };

            match compare_positions(&self.state.doc, anchor, position) {
                Ok(std::cmp::Ordering::Greater) => from,
                Ok(std::cmp::Ordering::Less) | Ok(std::cmp::Ordering::Equal) => to,
                Err(_) => position,
            }
        };

        if selection.head != new_head {
            self.transact(move |tr| {
                tr.set_selection(Selection::new(anchor, new_head));
                tr.set_preferred_x(None);
                Ok(true)
            })
        } else {
            vec![]
        }
    }

    pub(crate) fn handle_pointer_up(
        &mut self,
        _page_idx: usize,
        _x: f32,
        _y: f32,
        _button: PointerButton,
        _modifier: Modifier,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();

        match &self.pointer.mode {
            PointerMode::Pressed {
                document_position,
                context,
                start_x,
                start_y,
                page_idx: start_page_idx,
            } => match context {
                PressContext::Interactive(kind) => {
                    if let Some(page) = self.pages().get(*start_page_idx) {
                        if page
                            .find_interactive_at(*start_x, *start_y, self.is_read_only())
                            .as_ref()
                            == Some(kind)
                        {
                            effects.extend(self.handle_interaction(kind.clone()));
                            self.reset_pointer();
                            return effects;
                        }
                    }
                }
                PressContext::InSelection => {
                    let pos = *document_position;
                    effects.extend(self.transact(move |tr| {
                        tr.set_selection(Selection::collapsed(pos));
                        tr.set_preferred_x(None);
                        Ok(true)
                    }));
                }
                PressContext::OnSelectable(selection) => {
                    let selection = if self.is_read_only() {
                        Selection::collapsed(selection.anchor)
                    } else {
                        *selection
                    };

                    effects.extend(self.transact(move |tr| {
                        tr.set_selection(selection);
                        tr.set_preferred_x(None);
                        Ok(true)
                    }));
                }
                PressContext::Empty => {}
            },
            PointerMode::DraggingContent | PointerMode::DraggingExternal => {
                // DND는 Drop 메시지에서 처리
            }
            PointerMode::DraggingSelection => {}
            PointerMode::Idle => {}
        }

        self.reset_pointer();
        effects
    }

    pub(crate) fn handle_extend_selection_to(
        &mut self,
        anchor_page_idx: usize,
        anchor_x: f32,
        anchor_y: f32,
        head_page_idx: usize,
        head_x: f32,
        head_y: f32,
        double_tap_initial_range: Option<Selection>,
    ) -> Vec<Effect> {
        let Some(head_page) = self.pages().get(head_page_idx) else {
            return vec![];
        };

        let ctx = NavigationContext::new(&self.state.doc);

        let Some(head_hit) = Cursor::hit_test_drag(&ctx, head_page, head_x, head_y) else {
            return vec![];
        };

        let selection = self.state.selection;
        let new_selection = if let Some((initial_from, initial_to)) =
            double_tap_initial_range.and_then(|range| range.as_sorted(&self.state.doc).ok())
        {
            self.extend_selection_with_double_tap_range(initial_from, initial_to, head_hit)
        } else {
            let Some(anchor_page) = self.pages().get(anchor_page_idx) else {
                return vec![];
            };
            let Some(anchor_hit) = Cursor::hit_test_drag(&ctx, anchor_page, anchor_x, anchor_y)
            else {
                return vec![];
            };
            anchor_hit.extend_to(&self.state.doc, head_hit)
        };

        if new_selection.is_collapsed() {
            return vec![];
        }

        if selection != new_selection {
            self.transact(move |tr| {
                tr.set_selection(new_selection);
                tr.set_preferred_x(None);
                Ok(true)
            })
        } else {
            vec![]
        }
    }

    fn extend_selection_with_double_tap_range(
        &self,
        initial_from: Position,
        initial_to: Position,
        head_hit: Selection,
    ) -> Selection {
        let (head_from, head_to) = head_hit
            .as_sorted(&self.state.doc)
            .unwrap_or((head_hit.anchor, head_hit.head));

        if matches!(
            compare_positions(&self.state.doc, head_from, initial_from),
            Ok(Ordering::Less)
        ) {
            return Selection::collapsed(initial_to).extend_to(&self.state.doc, head_hit);
        }

        if matches!(
            compare_positions(&self.state.doc, head_to, initial_to),
            Ok(Ordering::Greater)
        ) {
            return Selection::collapsed(initial_from).extend_to(&self.state.doc, head_hit);
        }

        Selection::new(initial_from, initial_to)
    }
}
