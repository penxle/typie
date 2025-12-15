use super::super::{Effect, Runtime};
use crate::layout::cursor::{Cursor, NavigationContext};
use crate::runtime::message::{Modifier, PointerButton};
use crate::runtime::pointer::{PointerMode, PressContext};
use crate::state::position_helpers::compare_positions;
use crate::state::{Position, Selection};

impl Runtime {
    pub(crate) fn handle_pointer_down(
        &mut self,
        page_idx: usize,
        x: f32,
        y: f32,
        click_count: u32,
        button: PointerButton,
        modifier: Modifier,
    ) -> Vec<Effect> {
        let Some(page) = self.pages.get(page_idx) else {
            return vec![];
        };

        if button.is_primary() && !modifier.shift {
            if let Some(kind) = page.find_interactive_at(x, y) {
                self.pointer.mode = PointerMode::Pressed {
                    page_idx,
                    start_x: x,
                    start_y: y,
                    document_position: self.state.selection.head,
                    context: PressContext::Interactive(kind),
                };
                return vec![];
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
            return self.handle_shift_click(position);
        }

        if let Some(effects) = self.handle_multi_click(click_count, position) {
            return effects;
        }

        self.handle_single_click(page_idx, x, y, position, hit_selection)
    }

    fn handle_shift_click(&mut self, position: Position) -> Vec<Effect> {
        let anchor = self.state.selection.anchor;
        self.pointer.mode = PointerMode::DraggingSelection;

        self.transact(move |tr| {
            tr.set_selection(Selection::new(anchor, position));
            tr.set_preferred_x(None);
            Ok(true)
        })
    }

    fn handle_multi_click(&mut self, click_count: u32, position: Position) -> Option<Vec<Effect>> {
        if click_count <= 1 {
            return None;
        }

        self.pointer.mode = PointerMode::Idle;

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

        let is_in_selection = match context {
            PressContext::InSelection => true,
            PressContext::OnSelectable(_) => self.is_position_in_selection(position),
            _ => false,
        };

        self.pointer.mode = PointerMode::Pressed {
            page_idx,
            start_x: x,
            start_y: y,
            document_position: position,
            context,
        };

        if is_in_selection {
            vec![]
        } else {
            self.transact(move |tr| {
                tr.set_selection(hit_selection);
                tr.set_preferred_x(None);
                Ok(true)
            })
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
            self.pointer.reset();
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
                        self.pointer.mode = PointerMode::DraggingSelection;
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
        let Some(page) = self.pages.get(page_idx) else {
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
                    if let Some(page) = self.pages.get(*start_page_idx) {
                        if page.find_interactive_at(*start_x, *start_y).as_ref() == Some(kind) {
                            effects.extend(self.handle_interaction(kind.clone()));
                            self.pointer.reset();
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
                    let selection = *selection;
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

        self.pointer.reset();
        effects
    }
}
