use crate::dnd::DndState;
use crate::editor::Editor;
use crate::error::EditorError;
use crate::event::EditorEvent;
use crate::message::{
    AttachmentPlaceholderKind, DndDropPayload, DndOp, ExternalDndPayloadKind, InputModifiers,
};
use editor_clipboard::{PayloadSource, Slice};
use editor_commands::{self as commands};
use editor_crdt::Dot;
use editor_model::{DocView, Node};
use editor_resource::Resource;
use editor_state::{Affinity, Position, Selection, StableResolveCtx, StableSelection, State};
use editor_transaction::{HistoryMeta, Transaction};
use editor_view::DropTarget;

use super::insertion::{insert_attachment_placeholders_at, placeholder_slice, select_inserted_end};

pub fn handle_dnd_op(editor: &mut Editor, op: DndOp) -> Result<(), EditorError> {
    let previous_drop_target = editor.dnd.drop_target().cloned();
    let previous_reuse_node_id = editor.dnd.reuse_node_id();
    let result = match op {
        DndOp::StartInternalSelection => {
            let view = editor.state.view();
            editor.dnd = editor
                .state
                .selection
                .as_ref()
                .filter(|selection| !selection.is_collapsed())
                .map_or(DndState::Idle, |source| DndState::InternalDnd {
                    source: StableSelection::capture(source, &view),
                    drop_target: None,
                });
            Ok(())
        }
        DndOp::EnterExternal { payload } => {
            if !matches!(&editor.dnd, DndState::ExternalDnd { payload: active, .. } if *active == payload)
            {
                editor.dnd = DndState::ExternalDnd {
                    payload,
                    drop_target: None,
                    reuse_node_id: None,
                };
            }
            Ok(())
        }
        DndOp::Over {
            page,
            x,
            y,
            reuse_node_id,
            modifiers,
        } => {
            let internal_source = if let DndState::InternalDnd { source, .. } = &editor.dnd {
                let view = editor.state.view();
                let ctx = StableResolveCtx::from_live(&view, editor.state.projected.seq_checkout());
                source
                    .resolve(&ctx)
                    .filter(|selection| !selection.is_collapsed())
            } else {
                None
            };
            if internal_source.is_none() && matches!(&editor.dnd, DndState::InternalDnd { .. }) {
                editor.dnd = DndState::Idle;
            }

            let target = editor.view.drop_target_at(page, x, y);
            let mut validated_reuse_node_id = None;
            let target = match (editor.dnd.clone(), target) {
                (DndState::InternalDnd { .. }, Some(target)) => {
                    let live_position = resolve_drop_position(&target, &editor.state);
                    live_position.and_then(|live_position| {
                        internal_source.as_ref().and_then(|source| {
                            let view = editor.state.view();
                            let allowed = !position_inside_selection(&view, live_position, source)
                                && judge_apply_drop(
                                    &editor.state,
                                    &editor.resource.lock().unwrap(),
                                    live_position,
                                    &DndDropPayload::InternalSelection,
                                    modifiers,
                                    Some(source),
                                );
                            allowed.then_some(target)
                        })
                    })
                }
                (DndState::ExternalDnd { payload, .. }, Some(target)) => {
                    let live_position = resolve_drop_position(&target, &editor.state);
                    live_position.and_then(|live_position| {
                        let repr = representative_external_payload(payload);
                        let allowed = judge_apply_drop(
                            &editor.state,
                            &editor.resource.lock().unwrap(),
                            live_position,
                            &repr,
                            modifiers,
                            None,
                        );
                        if allowed {
                            validated_reuse_node_id = reuse_node_id.filter(|node_id| {
                                reusable_attachment_position_for_payload(
                                    editor, page, x, y, *node_id, payload,
                                )
                                .is_some()
                            });
                            Some(target)
                        } else {
                            None
                        }
                    })
                }
                _ => None,
            };
            editor.dnd.set_over_target(target, validated_reuse_node_id);
            Ok(())
        }
        DndOp::Drop {
            page,
            x,
            y,
            payload,
            modifiers,
        } => {
            let internal_source = if let DndState::InternalDnd { source, .. } = &editor.dnd {
                let view = editor.state.view();
                let ctx = StableResolveCtx::from_live(&view, editor.state.projected.seq_checkout());
                source
                    .resolve(&ctx)
                    .filter(|selection| !selection.is_collapsed())
            } else {
                None
            };
            let accepts_payload = match (&editor.dnd, &payload) {
                (DndState::InternalDnd { .. }, DndDropPayload::InternalSelection) => {
                    internal_source.is_some()
                }
                (
                    DndState::ExternalDnd { .. },
                    DndDropPayload::Text { .. } | DndDropPayload::Files { .. },
                ) => true,
                _ => false,
            };
            let target = if accepts_payload {
                match (&payload, internal_source.as_ref()) {
                    (DndDropPayload::InternalSelection, Some(source)) => {
                        editor.dnd.drop_target().cloned().and_then(|target| {
                            let position = resolve_drop_position(&target, &editor.state)?;
                            let view = editor.state.view();
                            let allowed = !position_inside_selection(&view, position, source)
                                && judge_apply_drop(
                                    &editor.state,
                                    &editor.resource.lock().unwrap(),
                                    position,
                                    &DndDropPayload::InternalSelection,
                                    modifiers,
                                    Some(source),
                                );
                            allowed.then_some(position)
                        })
                    }
                    _ => editor.dnd.drop_target().cloned().and_then(|target| {
                        let position = resolve_drop_position(&target, &editor.state)?;
                        judge_apply_drop(
                            &editor.state,
                            &editor.resource.lock().unwrap(),
                            position,
                            &payload,
                            modifiers,
                            None,
                        )
                        .then_some(position)
                    }),
                }
            } else {
                None
            };
            let result = if let Some(position) = target {
                apply_drop(
                    editor,
                    position,
                    payload,
                    modifiers,
                    internal_source,
                    Some((page, x, y)),
                )
            } else {
                Ok(())
            };
            editor.dnd = DndState::Idle;
            result
        }
        DndOp::Leave => {
            match &mut editor.dnd {
                DndState::InternalDnd { drop_target, .. } => {
                    *drop_target = None;
                }
                DndState::ExternalDnd { .. } => {
                    editor.dnd = DndState::Idle;
                }
                _ => {}
            }
            Ok(())
        }
        DndOp::End => {
            editor.dnd = DndState::Idle;
            Ok(())
        }
    };
    if editor.dnd.drop_target() != previous_drop_target.as_ref()
        || editor.dnd.reuse_node_id() != previous_reuse_node_id
    {
        editor.invalidate_render();
    }
    result
}

fn resolve_drop_position(target: &DropTarget, state: &State) -> Option<Position> {
    let view = state.view();
    let ctx = StableResolveCtx::from_live(&view, state.projected.seq_checkout());
    target.position.resolve(&ctx)
}

pub(crate) fn position_inside_selection(
    view: &DocView,
    position: Position,
    selection: &Selection,
) -> bool {
    let Some(resolved_selection) = selection.resolve(view) else {
        return false;
    };
    if let Some(cell_rect) = resolved_selection.as_cell_rect() {
        return view.node(position.node).is_some_and(|node| {
            node.ancestors()
                .find(|ancestor| matches!(ancestor.node(), Node::TableCell(_)))
                .is_some_and(|cell| cell_rect.contains(&cell))
        });
    }

    let Some(resolved_position) = position.resolve(view) else {
        return false;
    };
    *resolved_selection.from() < resolved_position && resolved_position < *resolved_selection.to()
}

// Dropping at the source selection's own from/to boundary is a structural no-op:
// the block would be deleted and re-inserted at the same position (with new Dots).
// Detect and skip early so the would-change judgment doesn't report a false positive.
fn drop_position_at_source_boundary(
    view: &DocView,
    position: Position,
    source: &Selection,
) -> bool {
    source.resolve(view).is_some_and(|resolved| {
        position == Position::from(resolved.from()) || position == Position::from(resolved.to())
    })
}

pub(crate) fn judge_apply_drop(
    state: &State,
    resource: &Resource,
    position: Position,
    payload: &DndDropPayload,
    modifiers: InputModifiers,
    source: Option<&Selection>,
) -> bool {
    match payload {
        DndDropPayload::Text { text, html } => {
            let (slice, _) = Slice::from_payload(html.as_deref(), text, resource);
            commands::resolve_slice_insertion(&state.view(), position, slice).is_some()
        }
        DndDropPayload::Files { kinds, .. } => {
            commands::resolve_slice_insertion(&state.view(), position, placeholder_slice(kinds))
                .is_some()
        }
        DndDropPayload::InternalSelection => {
            let Some(source) = source else {
                return false;
            };
            if !modifiers.alt && drop_position_at_source_boundary(&state.view(), position, source) {
                return false;
            }
            let mut extract_state = state.clone();
            extract_state.selection = Some(*source);
            let Some(slice) = Slice::extract(&extract_state) else {
                return false;
            };
            if modifiers.alt {
                // copy: 소스를 삭제하지 않으므로 드롭 전 원시 위치가 곧 실제 삽입 지점이다.
                return commands::resolve_slice_insertion(&state.view(), position, slice).is_some();
            }
            move_insertion_fits_after_delete(state, position, *source, slice)
        }
    }
}

/// move 판정의 유일한 권위 경로. move 실행은 원시 위치에 삽입하는 법이 없고 항상 소스
/// 삭제 → stable 재앵커 → 삽입 순으로 진행하므로, 삭제-전 원시 위치 평가는 어느
/// 방향으로도(과소·과대) 권위가 없다. 대신 그 실행 시퀀스(set_selection →
/// delete_selection → 재앵커)를 insert 직전까지 scratch 트랜잭션으로 미러하며,
/// 커밋하지 않으므로 관측 부작용이 없다. 재앵커 실패는 실행의 rollback no-op과 정확히
/// 합치해 false를 돌린다.
///
/// 마지막 삽입 가능성은 `resolve_slice_insertion`으로 판정한다 — 그 계약("Some(plan) ⇒
/// 관측 가능한 삽입 op 방출")이 `insert_slice_at`과 정확히 대응하므로(빈 슬라이스가 splice
/// edge join으로 소진돼 no-op이 되는 경우까지 `splice_emits_change`로 흡수됨) 이 근사는
/// 실행과 합치한다.
fn move_insertion_fits_after_delete(
    state: &State,
    position: Position,
    source: Selection,
    slice: Slice,
) -> bool {
    let stable_target = StableSelection::capture(&Selection::collapsed(position), &state.view());
    let mut tr = Transaction::new(state);
    if commands::set_selection(&mut tr, source).is_err() {
        return false;
    }
    if commands::delete_selection(&mut tr).is_err() {
        return false;
    }
    let view = tr.view();
    let ctx = StableResolveCtx::from_live(&view, tr.state().projected.seq_checkout());
    let Some(target) = stable_target.resolve(&ctx).map(|sel| sel.head) else {
        return false;
    };
    commands::resolve_slice_insertion(&view, target, slice).is_some()
}

fn representative_external_payload(payload: ExternalDndPayloadKind) -> DndDropPayload {
    match payload {
        ExternalDndPayloadKind::Text => DndDropPayload::Text {
            text: "x".into(),
            html: None,
        },
        ExternalDndPayloadKind::Html => DndDropPayload::Text {
            text: "x".into(),
            html: Some("<p>x</p>".into()),
        },
        ExternalDndPayloadKind::ImageFiles => DndDropPayload::Files {
            request_id: String::new(),
            kinds: vec![AttachmentPlaceholderKind::Image],
            reuse_node_id: None,
        },
        ExternalDndPayloadKind::Files => DndDropPayload::Files {
            request_id: String::new(),
            kinds: vec![AttachmentPlaceholderKind::File],
            reuse_node_id: None,
        },
        ExternalDndPayloadKind::MixedFiles => DndDropPayload::Files {
            request_id: String::new(),
            kinds: vec![
                AttachmentPlaceholderKind::Image,
                AttachmentPlaceholderKind::File,
            ],
            reuse_node_id: None,
        },
    }
}

fn reusable_attachment_position_for_payload(
    editor: &Editor,
    page: usize,
    x: f32,
    y: f32,
    node_id: Dot,
    payload: ExternalDndPayloadKind,
) -> Option<Position> {
    let kind = match (payload, editor.state.view().leaf(node_id)?.node()?) {
        (ExternalDndPayloadKind::ImageFiles, Node::Image(_)) => AttachmentPlaceholderKind::Image,
        (
            ExternalDndPayloadKind::ImageFiles
            | ExternalDndPayloadKind::Files
            | ExternalDndPayloadKind::MixedFiles,
            Node::File(_),
        ) => AttachmentPlaceholderKind::File,
        _ => return None,
    };
    reusable_attachment_position(editor, page, x, y, node_id, kind)
}

fn apply_drop(
    editor: &mut Editor,
    position: Position,
    payload: DndDropPayload,
    modifiers: InputModifiers,
    source: Option<Selection>,
    final_point: Option<(usize, f32, f32)>,
) -> Result<(), EditorError> {
    match payload {
        DndDropPayload::Text { text, html } => {
            let (slice, source) = {
                let resource = editor.resource.lock().unwrap();
                Slice::from_payload(html.as_deref(), &text, &resource)
            };
            let provenance = match source {
                PayloadSource::Html => commands::types::SliceProvenance::Formatted,
                PayloadSource::Text => commands::types::SliceProvenance::Plain,
            };
            // TODO: Legacy drop_external filled missing inline styles from
            // the target cascade. Keep this as a separate parity issue rather than
            // broadening insert_slice_at as part of the DnD contract commit.
            drop_slice_at(editor, position, slice, provenance)
        }
        DndDropPayload::Files {
            request_id,
            kinds,
            reuse_node_id,
        } => {
            if kinds.is_empty() {
                return Ok(());
            }
            let reuse = reuse_node_id.and_then(|node_id| {
                let (page, x, y) = final_point?;
                reusable_attachment_position(editor, page, x, y, node_id, kinds[0])
                    .map(|position| (node_id, position))
            });
            if let Some((node_id, mut end)) = reuse
                && kinds.len() == 1
            {
                end.affinity = Affinity::Upstream;
                editor.transact(|tr| {
                    tr.update_meta(|meta| meta.history = HistoryMeta::Skip);
                    select_inserted_end(tr, Selection::collapsed(end))
                })?;
                editor.push_event(EditorEvent::AttachmentPlaceholdersInserted {
                    request_id,
                    node_ids: vec![node_id],
                });
                return Ok(());
            }

            let mut inserted_node_ids = None;
            editor.transact(|tr| {
                if let Some((node_id, tail_position)) = reuse {
                    let savepoint = tr.savepoint();
                    match insert_attachment_placeholders_at(tr, tail_position, &kinds[1..]) {
                        Ok(Some((inserted, mut tail_node_ids))) => {
                            let start = Position {
                                offset: tail_position.offset.checked_sub(1).ok_or_else(|| {
                                    commands::CommandError::Corrupted(
                                        "reused placeholder has no preceding slot".into(),
                                    )
                                })?,
                                affinity: Affinity::Downstream,
                                ..tail_position
                            };
                            let end = {
                                let view = tr.view();
                                inserted
                                    .resolve(&view)
                                    .map(|resolved| resolved.to().position())
                                    .ok_or_else(|| {
                                        commands::CommandError::Corrupted(
                                            "inserted attachment range became unresolvable".into(),
                                        )
                                    })?
                            };
                            commands::set_selection(tr, Selection::new(start, end))?;
                            tail_node_ids.insert(0, node_id);
                            inserted_node_ids = Some(tail_node_ids);
                            return Ok(());
                        }
                        Ok(None) => tr.rollback(savepoint),
                        Err(err) => {
                            tr.rollback(savepoint);
                            return Err(err);
                        }
                    }
                }
                if let Some((inserted, node_ids)) =
                    insert_attachment_placeholders_at(tr, position, &kinds)?
                {
                    commands::set_selection(tr, inserted)?;
                    inserted_node_ids = Some(node_ids);
                }
                Ok(())
            })?;
            if let Some(node_ids) = inserted_node_ids {
                editor.push_event(EditorEvent::AttachmentPlaceholdersInserted {
                    request_id,
                    node_ids,
                });
            }
            Ok(())
        }
        DndDropPayload::InternalSelection => {
            drop_internal_selection_at(editor, position, modifiers.alt, source)
        }
    }
}

#[cfg(test)]
pub(crate) fn apply_drop_for_test(
    editor: &mut Editor,
    position: Position,
    payload: DndDropPayload,
    modifiers: InputModifiers,
    source: Option<Selection>,
) -> Result<(), EditorError> {
    apply_drop(editor, position, payload, modifiers, source, None)
}

fn reusable_attachment_position(
    editor: &Editor,
    page: usize,
    x: f32,
    y: f32,
    node_id: Dot,
    kind: AttachmentPlaceholderKind,
) -> Option<Position> {
    editor
        .view
        .page_external_elements(&editor.state, page, None)
        .into_iter()
        .find(|element| element.node == node_id && element.bounds.contains(x, y))?;
    let view = editor.state.view();
    let leaf = view.leaf(node_id)?;
    let kind_matches_empty = match (kind, leaf.node()?) {
        (AttachmentPlaceholderKind::Image, Node::Image(image)) => image.id.get().is_none(),
        (AttachmentPlaceholderKind::File, Node::File(file)) => file.id.get().is_none(),
        _ => false,
    };
    if !kind_matches_empty {
        return None;
    }

    let parent = leaf.parent()?;
    let index = parent.children().position(
        |child| matches!(child, editor_model::ChildView::Leaf(leaf) if leaf.dot() == node_id),
    )?;
    let position = Position {
        node: parent.id(),
        offset: index + 1,
        affinity: Affinity::Downstream,
    };
    position.resolve(&view)?;
    Some(position)
}

fn drop_slice_at(
    editor: &mut Editor,
    position: Position,
    slice: Slice,
    provenance: commands::types::SliceProvenance,
) -> Result<(), EditorError> {
    editor.transact(|tr| {
        let inserted_selection =
            commands::insert_slice_at(tr, position, slice.clone(), provenance)?;
        if let Some(inserted_selection) = inserted_selection
            && !inserted_selection.is_collapsed()
        {
            commands::set_selection(tr, inserted_selection)?;
        }
        Ok(())
    })
}

fn drop_internal_selection_at(
    editor: &mut Editor,
    position: Position,
    copy: bool,
    source: Option<Selection>,
) -> Result<(), EditorError> {
    let Some(source) = source else {
        return Ok(());
    };

    if !copy {
        let view = editor.state.view();
        if drop_position_at_source_boundary(&view, position, &source) {
            return Ok(());
        }
    }

    let mut state = editor.state().clone();
    state.selection = Some(source);
    let Some(slice) = Slice::extract(&state) else {
        return Ok(());
    };

    if copy {
        return drop_slice_at(
            editor,
            position,
            slice,
            commands::types::SliceProvenance::Formatted,
        );
    }

    let stable_target =
        StableSelection::capture(&Selection::collapsed(position), &editor.state.view());
    editor.transact(|tr| {
        let savepoint = tr.savepoint();
        commands::set_selection(tr, source)?;
        commands::delete_selection(tr)?;

        let resolved_target = {
            let view = tr.view();
            let ctx = StableResolveCtx::from_live(&view, tr.state().projected.seq_checkout());
            stable_target.resolve(&ctx)
        };
        let Some(target) = resolved_target.map(|sel| sel.head) else {
            tr.rollback(savepoint);
            return Ok(());
        };
        let Some(inserted_selection) = commands::insert_slice_at(
            tr,
            target,
            slice.clone(),
            commands::types::SliceProvenance::Formatted,
        )?
        else {
            tr.rollback(savepoint);
            return Ok(());
        };
        if !inserted_selection.is_collapsed() {
            commands::set_selection(tr, inserted_selection)?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{HistoryOp, Message, NodeOp, SystemEvent};
    use editor_macros::state;
    use editor_model::{ChildView, NodeType, PlainNode};

    #[test]
    fn dnd_attachment_placeholders_preserve_order_and_emit_one_matching_receipt() {
        let (initial, p1, existing_image, existing_file) = state! {
            doc { root {
                p1: paragraph { text("hello") }
                existing_image: image
                existing_file: file
            } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial.clone());
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(editor.state(), &Position::new(p1, 5))
            .expect("cursor metrics")
            .caret;

        let enter_events = editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::MixedFiles,
            },
        });
        assert!(
            !enter_events
                .iter()
                .any(|event| matches!(event, EditorEvent::AttachmentPlaceholdersInserted { .. })),
            "EnterExternal is only a probe",
        );

        let over_events = editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                reuse_node_id: None,
                modifiers: InputModifiers::default(),
            },
        });
        assert!(
            !over_events
                .iter()
                .any(|event| matches!(event, EditorEvent::AttachmentPlaceholdersInserted { .. })),
            "Over is only a probe",
        );

        let drop_events = editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                payload: DndDropPayload::Files {
                    request_id: "dnd-request".into(),
                    kinds: vec![
                        AttachmentPlaceholderKind::Image,
                        AttachmentPlaceholderKind::File,
                        AttachmentPlaceholderKind::Image,
                    ],
                    reuse_node_id: None,
                },
                modifiers: InputModifiers::default(),
            },
        });

        let view = editor.state().view();
        let root = view.node(Dot::ROOT).unwrap();
        let inserted: Vec<_> = root
            .children()
            .filter_map(|child| match child {
                ChildView::Block(block) => Some((block.id(), block.node_type())),
                ChildView::Leaf(leaf) => Some((leaf.dot(), leaf.node_type())),
            })
            .filter(|(node_id, node_type)| {
                *node_id != existing_image
                    && *node_id != existing_file
                    && matches!(node_type, NodeType::Image | NodeType::File)
            })
            .collect();
        assert_eq!(
            inserted
                .iter()
                .map(|(_, node_type)| *node_type)
                .collect::<Vec<_>>(),
            vec![NodeType::Image, NodeType::File, NodeType::Image]
        );
        let inserted_node_ids = inserted
            .iter()
            .map(|(node_id, _)| *node_id)
            .collect::<Vec<_>>();
        let matching_receipts = drop_events
            .iter()
            .filter_map(|event| match event {
                EditorEvent::AttachmentPlaceholdersInserted {
                    request_id,
                    node_ids,
                } if request_id == "dnd-request" => Some(node_ids),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(matching_receipts, vec![&inserted_node_ids]);
        assert!(
            matches!(root.last_child(), Some(ChildView::Block(block)) if block.node_type() == NodeType::Paragraph),
            "root schema should keep a trailing paragraph after inserted file blocks",
        );
        let first_text = root.child_blocks().next().map(|p| p.inline_text());
        assert_eq!(first_text.as_deref(), Some("hello"));

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        let view = editor.state().view();
        for node_id in inserted_node_ids {
            assert!(view.leaf(node_id).is_none(), "undo must remove {node_id:?}");
        }
        assert!(view.leaf(existing_image).is_some());
        assert!(view.leaf(existing_file).is_some());
    }

    #[test]
    fn dnd_attachment_placeholders_reuse_compatible_empty_candidate_and_insert_tail_after_it() {
        let (initial, p1, candidate) = state! {
            doc { root {
                p1: paragraph { text("target") }
                candidate: image
            } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        let candidate_element = editor
            .view()
            .page_external_elements(editor.state(), 0, None)
            .into_iter()
            .find(|element| element.node == candidate)
            .expect("candidate external element");
        let candidate_point = (
            candidate_element.bounds.center_x(),
            candidate_element.bounds.y + candidate_element.bounds.height / 2.0,
        );
        let caret = editor
            .view()
            .cursor_metrics(editor.state(), &Position::new(p1, 6))
            .expect("normal drop target")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::MixedFiles,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                reuse_node_id: None,
                modifiers: InputModifiers::default(),
            },
        });
        let events = editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: candidate_point.0,
                y: candidate_point.1,
                payload: DndDropPayload::Files {
                    request_id: "reuse-tail".into(),
                    kinds: vec![
                        AttachmentPlaceholderKind::Image,
                        AttachmentPlaceholderKind::File,
                        AttachmentPlaceholderKind::Image,
                    ],
                    reuse_node_id: Some(candidate),
                },
                modifiers: InputModifiers::default(),
            },
        });

        let receipt = events
            .iter()
            .find_map(|event| match event {
                EditorEvent::AttachmentPlaceholdersInserted {
                    request_id,
                    node_ids,
                } if request_id == "reuse-tail" => Some(node_ids),
                _ => None,
            })
            .expect("reuse receipt");
        assert_eq!(receipt.len(), 3);
        assert_eq!(receipt.first(), Some(&candidate));
        let children = editor
            .state()
            .view()
            .root()
            .expect("root")
            .children()
            .map(|child| match child {
                ChildView::Block(block) => (block.id(), block.node_type()),
                ChildView::Leaf(leaf) => (leaf.dot(), leaf.node_type()),
            })
            .collect::<Vec<_>>();
        let candidate_index = children
            .iter()
            .position(|(node_id, _)| *node_id == candidate)
            .expect("candidate remains");
        assert_eq!(
            &children[candidate_index..candidate_index + 3],
            &[
                (receipt[0], NodeType::Image),
                (receipt[1], NodeType::File),
                (receipt[2], NodeType::Image),
            ]
        );
    }

    #[test]
    fn dnd_attachment_drop_selects_dropped_placeholder_range() {
        let (initial, root, candidate, _image_a, _image_b) = state! {
            doc { root: root {
                candidate: file
                image_a: image
                image_b: image
            } }
            selection: (root, 2, >) -> (root, 3, <)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        let candidate_element = editor
            .view()
            .page_external_elements(editor.state(), 0, None)
            .into_iter()
            .find(|element| element.node == candidate)
            .expect("candidate external element");
        let x = candidate_element.bounds.center_x();
        let y = candidate_element.bounds.y + candidate_element.bounds.height / 2.0;

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::Files,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x,
                y,
                reuse_node_id: Some(candidate),
                modifiers: InputModifiers::default(),
            },
        });
        let events = editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x,
                y,
                payload: DndDropPayload::Files {
                    request_id: "select-range".into(),
                    kinds: vec![
                        AttachmentPlaceholderKind::File,
                        AttachmentPlaceholderKind::File,
                    ],
                    reuse_node_id: Some(candidate),
                },
                modifiers: InputModifiers::default(),
            },
        });
        let receipt = events
            .iter()
            .find_map(|event| match event {
                EditorEvent::AttachmentPlaceholdersInserted {
                    request_id,
                    node_ids,
                } if request_id == "select-range" => Some(node_ids),
                _ => None,
            })
            .expect("matching receipt");
        let selection = editor.state().selection.expect("selection remains");
        assert_eq!(selection.anchor.node, root);
        assert_eq!(selection.head.node, root);
        assert_eq!(
            selection.anchor.offset.min(selection.head.offset),
            0,
            "the reused placeholder starts the dropped range"
        );
        assert_eq!(
            selection.anchor.offset.max(selection.head.offset),
            receipt.len(),
            "the inserted tail ends the dropped range"
        );
    }

    #[test]
    fn dnd_over_reusable_placeholder_keeps_target_and_toggles_only_visible_indicator() {
        let (initial, _root, candidate) = state! {
            doc { root: root {
                paragraph { text("target") }
                candidate: image
            } }
            selection: (root, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        let candidate_element = editor
            .view()
            .page_external_elements(editor.state(), 0, None)
            .into_iter()
            .find(|element| element.node == candidate)
            .expect("candidate external element");
        let x = candidate_element.bounds.center_x();
        let y = candidate_element.bounds.y + candidate_element.bounds.height / 2.0;

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::ImageFiles,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x,
                y,
                reuse_node_id: None,
                modifiers: InputModifiers::default(),
            },
        });
        assert!(editor.drop_indicator_for_test().is_some());

        let hidden_events = editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x,
                y,
                reuse_node_id: Some(candidate),
                modifiers: InputModifiers::default(),
            },
        });
        assert!(editor.dnd.drop_target().is_some());
        assert!(editor.drop_indicator_for_test().is_none());
        assert!(
            hidden_events
                .iter()
                .any(|event| matches!(event, EditorEvent::RenderInvalidated))
        );

        let restored_events = editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x,
                y,
                reuse_node_id: None,
                modifiers: InputModifiers::default(),
            },
        });
        assert!(editor.dnd.drop_target().is_some());
        assert!(editor.drop_indicator_for_test().is_some());
        assert!(
            restored_events
                .iter()
                .any(|event| matches!(event, EditorEvent::RenderInvalidated))
        );

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::Files,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x,
                y,
                reuse_node_id: Some(candidate),
                modifiers: InputModifiers::default(),
            },
        });
        assert!(
            editor.drop_indicator_for_test().is_some(),
            "the engine must not trust a host candidate that is incompatible with the active payload"
        );
    }

    #[test]
    fn dnd_attachment_candidate_only_selects_existing_id_without_document_mutation() {
        let (initial, root, _p1, candidate) = state! {
            doc { root: root {
                p1: paragraph { text("target") }
                candidate: image
            } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial.clone());
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        let candidate_element = editor
            .view()
            .page_external_elements(editor.state(), 0, None)
            .into_iter()
            .find(|element| element.node == candidate)
            .expect("candidate external element");
        let history_len = editor.history_undos_len();

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::ImageFiles,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: candidate_element.bounds.center_x(),
                y: candidate_element.bounds.y + candidate_element.bounds.height / 2.0,
                modifiers: InputModifiers::default(),
                reuse_node_id: Some(candidate),
            },
        });
        assert!(
            editor.drop_indicator_for_test().is_none(),
            "a validated reusable placeholder must replace the generic drop indicator"
        );
        let events = editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: candidate_element.bounds.center_x(),
                y: candidate_element.bounds.y + candidate_element.bounds.height / 2.0,
                payload: DndDropPayload::Files {
                    request_id: "candidate-only".into(),
                    kinds: vec![AttachmentPlaceholderKind::Image],
                    reuse_node_id: Some(candidate),
                },
                modifiers: InputModifiers::default(),
            },
        });

        assert!(events.iter().any(|event| matches!(
            event,
            EditorEvent::AttachmentPlaceholdersInserted { request_id, node_ids }
                if request_id == "candidate-only" && node_ids == &vec![candidate]
        )));
        assert_eq!(editor.state().to_plain(), initial.to_plain());
        let selection = editor.state().selection.expect("candidate is selected");
        assert_eq!(selection.anchor.node, root);
        assert_eq!(selection.head.node, root);
        assert_eq!(selection.anchor.offset.abs_diff(selection.head.offset), 1);
        let selected_index = selection.anchor.offset.min(selection.head.offset);
        assert!(
            matches!(
                editor.state().view().node(root).and_then(|root| root.child_at(selected_index)),
                Some(ChildView::Leaf(leaf)) if leaf.dot() == candidate
            ),
            "the reused placeholder should be selected"
        );
        assert_eq!(editor.history_undos_len(), history_len);
    }

    #[test]
    fn dnd_attachment_placeholders_ignore_reuse_candidate_outside_final_point() {
        let (initial, p1, candidate, point_target) = state! {
            doc { root {
                p1: paragraph { text("target") }
                candidate: image
                point_target: file
            } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        let point_element = editor
            .view()
            .page_external_elements(editor.state(), 0, None)
            .into_iter()
            .find(|element| element.node == point_target)
            .expect("point target external element");
        let caret = editor
            .view()
            .cursor_metrics(editor.state(), &Position::new(p1, 6))
            .expect("normal drop target")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::MixedFiles,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                reuse_node_id: None,
                modifiers: InputModifiers::default(),
            },
        });
        let events = editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: point_element.bounds.center_x(),
                y: point_element.bounds.y + point_element.bounds.height / 2.0,
                payload: DndDropPayload::Files {
                    request_id: "outside-candidate".into(),
                    kinds: vec![
                        AttachmentPlaceholderKind::Image,
                        AttachmentPlaceholderKind::File,
                    ],
                    reuse_node_id: Some(candidate),
                },
                modifiers: InputModifiers::default(),
            },
        });

        let receipt = events
            .iter()
            .find_map(|event| match event {
                EditorEvent::AttachmentPlaceholdersInserted {
                    request_id,
                    node_ids,
                } if request_id == "outside-candidate" => Some(node_ids),
                _ => None,
            })
            .expect("fallback receipt");
        assert!(!receipt.contains(&candidate));
        assert_eq!(receipt.len(), 2);
        let view = editor.state().view();
        let root = view.root().expect("root");
        assert!(
            matches!(root.child_at(1), Some(ChildView::Leaf(leaf)) if leaf.dot() == receipt[0])
        );
        assert!(
            matches!(root.child_at(2), Some(ChildView::Leaf(leaf)) if leaf.dot() == receipt[1])
        );
        assert!(matches!(root.child_at(3), Some(ChildView::Leaf(leaf)) if leaf.dot() == candidate));
        assert!(
            matches!(root.child_at(4), Some(ChildView::Leaf(leaf)) if leaf.dot() == point_target)
        );
    }

    #[test]
    fn dnd_attachment_placeholders_ignore_wrong_kind_reuse_candidate() {
        let (initial, p1, candidate) = state! {
            doc { root {
                p1: paragraph { text("target") }
                candidate: file
            } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        let candidate_element = editor
            .view()
            .page_external_elements(editor.state(), 0, None)
            .into_iter()
            .find(|element| element.node == candidate)
            .expect("candidate external element");
        let caret = editor
            .view()
            .cursor_metrics(editor.state(), &Position::new(p1, 6))
            .expect("normal drop target")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::ImageFiles,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                reuse_node_id: None,
                modifiers: InputModifiers::default(),
            },
        });
        let events = editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: candidate_element.bounds.center_x(),
                y: candidate_element.bounds.y + candidate_element.bounds.height / 2.0,
                payload: DndDropPayload::Files {
                    request_id: "wrong-kind".into(),
                    kinds: vec![AttachmentPlaceholderKind::Image],
                    reuse_node_id: Some(candidate),
                },
                modifiers: InputModifiers::default(),
            },
        });

        let receipt = events
            .iter()
            .find_map(|event| match event {
                EditorEvent::AttachmentPlaceholdersInserted {
                    request_id,
                    node_ids,
                } if request_id == "wrong-kind" => Some(node_ids),
                _ => None,
            })
            .expect("fallback receipt");
        assert_eq!(receipt.len(), 1);
        assert_ne!(receipt[0], candidate);
        let view = editor.state().view();
        let root = view.root().expect("root");
        assert!(
            matches!(root.child_at(1), Some(ChildView::Leaf(leaf)) if leaf.dot() == receipt[0])
        );
        assert!(matches!(root.child_at(2), Some(ChildView::Leaf(leaf)) if leaf.dot() == candidate));
    }

    #[test]
    fn dnd_attachment_placeholders_ignore_committed_reuse_candidate() {
        let (initial, p1, candidate) = state! {
            doc { root {
                p1: paragraph { text("target") }
                candidate: image
            } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::Node {
            op: NodeOp::SetAttrs {
                id: candidate,
                attrs: PlainNode::Image(editor_model::PlainImageNode {
                    id: Some("asset-id".into()),
                    proportion: 100,
                }),
            },
        });
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        let candidate_element = editor
            .view()
            .page_external_elements(editor.state(), 0, None)
            .into_iter()
            .find(|element| element.node == candidate)
            .expect("candidate external element");
        let caret = editor
            .view()
            .cursor_metrics(editor.state(), &Position::new(p1, 6))
            .expect("normal drop target")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::ImageFiles,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                reuse_node_id: None,
                modifiers: InputModifiers::default(),
            },
        });
        let events = editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: candidate_element.bounds.center_x(),
                y: candidate_element.bounds.y + candidate_element.bounds.height / 2.0,
                payload: DndDropPayload::Files {
                    request_id: "committed-candidate".into(),
                    kinds: vec![AttachmentPlaceholderKind::Image],
                    reuse_node_id: Some(candidate),
                },
                modifiers: InputModifiers::default(),
            },
        });

        let receipt = events
            .iter()
            .find_map(|event| match event {
                EditorEvent::AttachmentPlaceholdersInserted {
                    request_id,
                    node_ids,
                } if request_id == "committed-candidate" => Some(node_ids),
                _ => None,
            })
            .expect("fallback receipt");
        assert_eq!(receipt.len(), 1);
        assert_ne!(receipt[0], candidate);
        let view = editor.state().view();
        let root = view.root().expect("root");
        assert!(
            matches!(root.child_at(1), Some(ChildView::Leaf(leaf)) if leaf.dot() == receipt[0])
        );
        assert!(matches!(root.child_at(2), Some(ChildView::Leaf(leaf)) if leaf.dot() == candidate));
        let candidate_node = editor
            .state()
            .view()
            .leaf(candidate)
            .expect("candidate remains")
            .node()
            .expect("candidate node");
        assert!(
            matches!(candidate_node, Node::Image(image) if image.id.get().as_deref() == Some("asset-id"))
        );
    }

    #[test]
    fn dnd_attachment_placeholders_empty_and_rejected_drops_emit_no_receipt() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("target") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial.clone());
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(editor.state(), &Position::new(p1, 6))
            .expect("normal drop target")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::Files,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                reuse_node_id: None,
                modifiers: InputModifiers::default(),
            },
        });
        let empty_events = editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                payload: DndDropPayload::Files {
                    request_id: "empty-dnd".into(),
                    kinds: vec![],
                    reuse_node_id: None,
                },
                modifiers: InputModifiers::default(),
            },
        });
        assert!(
            !empty_events
                .iter()
                .any(|event| matches!(event, EditorEvent::AttachmentPlaceholdersInserted { .. }))
        );

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::ImageFiles,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: usize::MAX,
                x: 0.0,
                y: 0.0,
                reuse_node_id: None,
                modifiers: InputModifiers::default(),
            },
        });
        let rejected_events = editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: usize::MAX,
                x: 0.0,
                y: 0.0,
                payload: DndDropPayload::Files {
                    request_id: "rejected-dnd".into(),
                    kinds: vec![AttachmentPlaceholderKind::Image],
                    reuse_node_id: None,
                },
                modifiers: InputModifiers::default(),
            },
        });
        assert!(
            !rejected_events
                .iter()
                .any(|event| matches!(event, EditorEvent::AttachmentPlaceholdersInserted { .. }))
        );
        editor_state::assert_state_eq!(editor.state(), &initial);
    }
}
