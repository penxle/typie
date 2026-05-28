use crate::editor::Editor;
use crate::error::EditorError;
use crate::event::EditorEvent;
use crate::interaction::InteractionState;
use crate::message::{DndDropPayload, DndOp, InputModifiers};
use editor_clipboard::Slice;
use editor_commands::{self as commands};
use editor_model::{Fragment, PlainFileNode, PlainImageNode, PlainNode, PlainRootNode};
use editor_state::{Position, Selection, StableSelection};

#[derive(Debug, Clone, Copy)]
enum SelectionAfterDrop {
    KeepCommandSelection, // 파일/이미지: insert_slice_at의 기본 선택을 유지
    SelectInsertedRange,  // 텍스트: 삽입된 범위를 선택
}

pub fn handle_dnd_op(editor: &mut Editor, op: DndOp) -> Result<(), EditorError> {
    let previous_drop_target = editor.interaction.drop_target();
    let result = match op {
        DndOp::StartInternalSelection => {
            editor.interaction = editor
                .state
                .selection
                .as_ref()
                .filter(|selection| !selection.is_collapsed())
                .map_or(InteractionState::Idle, |source| {
                    InteractionState::InternalDnd {
                        source: StableSelection::freeze(source, &editor.state.doc),
                        drop_target: None,
                    }
                });
            Ok(())
        }
        DndOp::EnterExternal { payload } => {
            if !matches!(&editor.interaction, InteractionState::ExternalDnd { payload: active, .. } if *active == payload)
            {
                editor.interaction = InteractionState::ExternalDnd {
                    payload,
                    drop_target: None,
                };
            }
            Ok(())
        }
        DndOp::Over { page, x, y, .. } => {
            let internal_source =
                if let InteractionState::InternalDnd { source, .. } = &editor.interaction {
                    let selection = source.thaw(&editor.state.doc);
                    (!selection.is_collapsed()).then_some(selection)
                } else {
                    None
                };
            if internal_source.is_none()
                && matches!(&editor.interaction, InteractionState::InternalDnd { .. })
            {
                editor.interaction = InteractionState::Idle;
            }

            let target = match &editor.interaction {
                InteractionState::InternalDnd { .. } => editor.view.drop_target_at(
                    &editor.state.doc,
                    page,
                    x,
                    y,
                    internal_source.as_ref(),
                ),
                InteractionState::ExternalDnd { .. } => {
                    editor
                        .view
                        .drop_target_at(&editor.state.doc, page, x, y, None)
                }
                _ => None,
            };
            // TODO(TR-100): Filter targets by payload-specific drop policy before
            // showing the indicator. Internal drops need the legacy can_drop checks
            // such as page-break restrictions; external drops still need a product
            // decision between rejecting the target and coercing unsupported content.
            editor.interaction.set_drop_target(target);
            Ok(())
        }
        DndOp::Drop {
            page,
            x,
            y,
            payload,
            modifiers,
        } => {
            let internal_source =
                if let InteractionState::InternalDnd { source, .. } = &editor.interaction {
                    let selection = source.thaw(&editor.state.doc);
                    (!selection.is_collapsed()).then_some(selection)
                } else {
                    None
                };
            let accepts_payload = match (&editor.interaction, &payload) {
                (InteractionState::InternalDnd { .. }, DndDropPayload::InternalSelection) => {
                    internal_source.is_some()
                }
                (
                    InteractionState::ExternalDnd { .. },
                    DndDropPayload::Text { .. } | DndDropPayload::Files { .. },
                ) => true,
                _ => false,
            };
            let target = if accepts_payload {
                let source = internal_source.as_ref();
                editor.interaction.drop_target().or_else(|| {
                    editor
                        .view
                        .drop_target_at(&editor.state.doc, page, x, y, source)
                })
            } else {
                None
            };
            let result = if let Some(target) = target {
                apply_drop(editor, target.position, payload, modifiers, internal_source)
            } else {
                Ok(())
            };
            editor.interaction = InteractionState::Idle;
            result
        }
        DndOp::Leave => {
            match &mut editor.interaction {
                InteractionState::InternalDnd { drop_target, .. } => {
                    *drop_target = None;
                }
                InteractionState::ExternalDnd { .. } => {
                    editor.interaction = InteractionState::Idle;
                }
                _ => {}
            }
            Ok(())
        }
        DndOp::End => {
            editor.interaction = InteractionState::Idle;
            Ok(())
        }
    };
    if editor.interaction.drop_target() != previous_drop_target {
        editor.push_event(EditorEvent::RenderInvalidated);
    }
    result
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
