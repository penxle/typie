use crate::dnd::DndState;
use crate::editor::Editor;
use crate::error::EditorError;
use crate::event::EditorEvent;
use crate::message::{DndDropPayload, DndOp, ExternalDndPayloadKind, InputModifiers};
use editor_clipboard::Slice;
use editor_commands::{self as commands};
use editor_model::{
    Doc, Fragment, Node, PlainFileNode, PlainImageNode, PlainNode, PlainRootNode, Schema,
};
use editor_state::{Position, Selection, StableSelection};

#[derive(Debug, Clone, Copy)]
enum SelectionAfterDrop {
    KeepCommandSelection, // 파일/이미지: insert_slice_at의 기본 선택을 유지
    SelectInsertedRange,  // 텍스트: 삽입된 범위를 선택
}

pub fn handle_dnd_op(editor: &mut Editor, op: DndOp) -> Result<(), EditorError> {
    let previous_drop_target = editor.dnd.drop_target();
    let result = match op {
        DndOp::StartInternalSelection => {
            editor.dnd = editor
                .state
                .selection
                .as_ref()
                .filter(|selection| !selection.is_collapsed())
                .map(|selection| snap_to_block_unit(&editor.state.doc, selection))
                .filter(|selection| !selection.is_collapsed())
                .map_or(DndState::Idle, |source| DndState::InternalDnd {
                    source: StableSelection::freeze(&source, &editor.state.doc),
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
                };
            }
            Ok(())
        }
        DndOp::Over {
            page,
            x,
            y,
            modifiers,
        } => {
            let internal_source = if let DndState::InternalDnd { source, .. } = &editor.dnd {
                let selection = source.thaw(&editor.state.doc);
                (!selection.is_collapsed()).then_some(selection)
            } else {
                None
            };
            if internal_source.is_none() && matches!(&editor.dnd, DndState::InternalDnd { .. }) {
                editor.dnd = DndState::Idle;
            }

            let target = match editor.dnd.clone() {
                DndState::InternalDnd { .. } => editor
                    .view
                    .drop_target_at(&editor.state.doc, page, x, y)
                    .filter(|target| {
                        internal_source.as_ref().is_some_and(|source| {
                            !position_inside_selection(&editor.state.doc, target.position, source)
                                && can_apply_drop(
                                    editor,
                                    target.position,
                                    DndDropPayload::InternalSelection,
                                    modifiers,
                                    Some(*source),
                                )
                        })
                    }),
                DndState::ExternalDnd { payload, .. } => editor
                    .view
                    .drop_target_at(&editor.state.doc, page, x, y)
                    .filter(|target| {
                        can_apply_drop(
                            editor,
                            target.position,
                            representative_external_payload(payload),
                            modifiers,
                            None,
                        )
                    }),
                _ => None,
            };
            editor.dnd.set_drop_target(target);
            Ok(())
        }
        DndOp::Drop {
            payload, modifiers, ..
        } => {
            let internal_source = if let DndState::InternalDnd { source, .. } = &editor.dnd {
                let selection = source.thaw(&editor.state.doc);
                (!selection.is_collapsed()).then_some(selection)
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
                        editor.dnd.drop_target().filter(|target| {
                            !position_inside_selection(&editor.state.doc, target.position, source)
                                && can_apply_drop(
                                    editor,
                                    target.position,
                                    DndDropPayload::InternalSelection,
                                    modifiers,
                                    Some(*source),
                                )
                        })
                    }
                    _ => editor.dnd.drop_target().filter(|target| {
                        can_apply_drop(editor, target.position, payload.clone(), modifiers, None)
                    }),
                }
            } else {
                None
            };
            let result = if let Some(target) = target {
                apply_drop(editor, target.position, payload, modifiers, internal_source)
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
    if editor.dnd.drop_target() != previous_drop_target {
        editor.push_event(EditorEvent::RenderInvalidated);
    }
    result
}

fn position_inside_selection(doc: &Doc, position: Position, selection: &Selection) -> bool {
    let Some(resolved_selection) = selection.resolve(doc) else {
        return false;
    };
    if let Some(cell_rect) = resolved_selection.as_cell_rect() {
        return doc.node(position.node_id).is_some_and(|node| {
            node.ancestors()
                .find(|ancestor| matches!(ancestor.node(), Node::TableCell(_)))
                .is_some_and(|cell| cell_rect.contains(&cell))
        });
    }

    let Some(resolved_position) = position.resolve(doc) else {
        return false;
    };
    *resolved_selection.from() < resolved_position && resolved_position < *resolved_selection.to()
}

/// anchor가 isolating+monolithic 블록(fold/table)의 parent 경계에 있으면
/// 해당 블록의 단위 선택으로 스냅한다. promote_cross_isolating이 만드는
/// (parent, fold_idx)→(external_pos) 선택을 DnD 소스로 정제하기 위해 사용.
fn snap_to_block_unit(doc: &Doc, selection: &Selection) -> Selection {
    let anchor = selection.anchor;
    if let Some(node) = doc.node(anchor.node_id) {
        let spec = Schema::node_spec(node.as_type());
        if !spec.is_textblock() && !spec.inline {
            if let Some(child) = node.children().nth(anchor.offset) {
                let child_spec = Schema::node_spec(child.as_type());
                if child_spec.isolating && child_spec.monolithic {
                    let unit_end = Position::new(anchor.node_id, anchor.offset + 1);
                    return Selection::new(anchor, unit_end);
                }
            }
        }
    }
    selection.clone()
}

fn can_apply_drop(
    editor: &mut Editor,
    position: Position,
    payload: DndDropPayload,
    modifiers: InputModifiers,
    source: Option<Selection>,
) -> bool {
    editor
        .probe(|editor| apply_drop(editor, position, payload, modifiers, source))
        .unwrap_or(false)
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
            image_count: 1,
            file_count: 0,
        },
        ExternalDndPayloadKind::Files => DndDropPayload::Files {
            image_count: 0,
            file_count: 1,
        },
        ExternalDndPayloadKind::MixedFiles => DndDropPayload::Files {
            image_count: 1,
            file_count: 1,
        },
    }
}

fn apply_drop(
    editor: &mut Editor,
    position: Position,
    payload: DndDropPayload,
    modifiers: InputModifiers,
    source: Option<Selection>,
) -> Result<(), EditorError> {
    match payload {
        DndDropPayload::Text { text, html } => {
            let slice = {
                let resource = editor.resource.lock().unwrap();
                slice_from_drop_text_payload(&text, html.as_deref(), &resource)
            };
            // TODO: Legacy drop_external filled missing inline styles from
            // the target cascade. Keep this as a separate parity issue rather than
            // broadening insert_slice_at as part of the DnD contract commit.
            drop_slice_at(
                editor,
                position,
                slice,
                SelectionAfterDrop::SelectInsertedRange,
            )
        }
        DndDropPayload::Files {
            image_count,
            file_count,
        } => drop_slice_at(
            editor,
            position,
            files_slice(image_count, file_count),
            SelectionAfterDrop::KeepCommandSelection,
        ),
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
    apply_drop(editor, position, payload, modifiers, source)
}

fn drop_slice_at(
    editor: &mut Editor,
    position: Position,
    slice: Slice,
    selection: SelectionAfterDrop,
) -> Result<(), EditorError> {
    editor.transact(|tr| {
        let inserted_selection = commands::insert_slice_at(tr, position, slice.clone())?;
        if let Some(inserted_selection) = inserted_selection
            && matches!(selection, SelectionAfterDrop::SelectInsertedRange)
            && !inserted_selection.is_collapsed()
        {
            commands::set_selection(tr, inserted_selection)?;
        }
        Ok(())
    })
}

fn slice_from_drop_text_payload(
    text: &str,
    html: Option<&str>,
    resource: &editor_resource::Resource,
) -> Slice {
    if let Some(html) = html.filter(|html| !html.is_empty()) {
        let slice = Slice::from_html(html, resource);
        if !slice.is_empty() {
            return slice;
        }
    }

    Slice::from_text(text)
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

    // Dropping at the source selection's own from/to boundary is a structural no-op:
    // the block would be deleted and re-inserted at the same position (with new NodeIds).
    // Detect and skip early so probe mode doesn't report a false "state changed".
    if !copy {
        if let Some(resolved) = source.resolve(&editor.state.doc) {
            let from = Position::from(resolved.from());
            let to = Position::from(resolved.to());
            if position == from || position == to {
                return Ok(());
            }
        }
    }

    let mut state = editor.state().clone();
    state.selection = Some(source.clone());
    let Some(slice) = Slice::extract(&state) else {
        return Ok(());
    };

    if copy {
        return drop_slice_at(
            editor,
            position,
            slice,
            SelectionAfterDrop::SelectInsertedRange,
        );
    }

    let stable_target = StableSelection::freeze(&Selection::collapsed(position), &editor.state.doc);
    editor.transact(|tr| {
        commands::set_selection(tr, source)?;
        commands::delete_selection(tr)?;

        let target = stable_target.thaw(&tr.doc()).head;
        if let Some(inserted_selection) = commands::insert_slice_at(tr, target, slice.clone())?
            && !inserted_selection.is_collapsed()
        {
            commands::set_selection(tr, inserted_selection)?;
        }
        Ok(())
    })
}

fn files_slice(image_count: u32, file_count: u32) -> Slice {
    let mut children = Vec::with_capacity((image_count + file_count) as usize);
    // NOTE: Files payload는 개수만 담기 때문에 mixed drop의 원래 순서를 잃고 이미지가 먼저 삽입된다.
    for _ in 0..image_count {
        children.push(Fragment::leaf(PlainNode::Image(PlainImageNode {
            id: None,
            proportion: 100,
        })));
    }
    for _ in 0..file_count {
        children.push(Fragment::leaf(PlainNode::File(PlainFileNode { id: None })));
    }

    Slice {
        fragment: Fragment {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: vec![],
            children,
        },
        open_start: 0,
        open_end: 0,
    }
}
