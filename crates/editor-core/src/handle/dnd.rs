use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::{DndDropPayload, DndOp, DndPayloadKind, InputModifiers};
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
    match op {
        DndOp::Over {
            page,
            x,
            y,
            payload,
            ..
        } => {
            let source = internal_dnd_source(editor, payload);
            let target = editor
                .view
                .drop_target_at(&editor.state.doc, page, x, y, source);
            // TODO(TR-100): Filter targets by payload-specific drop policy before
            // showing the indicator. Internal drops need the legacy can_drop checks
            // such as page-break restrictions; external drops still need a product
            // decision between rejecting the target and coercing unsupported content.
            editor.set_drop_target(target);
        }
        DndOp::Drop {
            page,
            x,
            y,
            payload,
            modifiers,
        } => {
            let payload_kind = payload_kind_for_drop(&payload);
            let source = internal_dnd_source(editor, payload_kind);
            let target = editor.drop_target().or_else(|| {
                editor
                    .view
                    .drop_target_at(&editor.state.doc, page, x, y, source)
            });
            let result = if let Some(target) = target {
                apply_drop(editor, target.position, payload, modifiers)
            } else {
                Ok(())
            };
            editor.set_drop_target(None);
            result?;
        }
        DndOp::Leave | DndOp::End => {
            editor.set_drop_target(None);
        }
        DndOp::Start { .. } | DndOp::Enter { .. } => {}
    }
    Ok(())
}

fn internal_dnd_source(
    editor: &Editor,
    payload: DndPayloadKind,
) -> Option<&editor_state::Selection> {
    if payload != DndPayloadKind::InternalSelection {
        return None;
    }
    editor
        .state
        .selection
        .as_ref()
        .filter(|selection| !selection.is_collapsed())
}

fn payload_kind_for_drop(payload: &DndDropPayload) -> DndPayloadKind {
    match payload {
        DndDropPayload::InternalSelection => DndPayloadKind::InternalSelection,
        DndDropPayload::Text { html, .. } if html.as_deref().is_some_and(|h| !h.is_empty()) => {
            DndPayloadKind::Html
        }
        DndDropPayload::Text { .. } => DndPayloadKind::Text,
        DndDropPayload::Files {
            image_count,
            file_count,
        } => match (*image_count > 0, *file_count > 0) {
            (true, true) => DndPayloadKind::MixedFiles,
            (true, false) => DndPayloadKind::ImageFiles,
            _ => DndPayloadKind::Files,
        },
    }
}

fn apply_drop(
    editor: &mut Editor,
    position: Position,
    payload: DndDropPayload,
    modifiers: InputModifiers,
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
            drop_internal_selection_at(editor, position, modifiers.alt)
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
        if !slice_is_empty(&slice) {
            return slice;
        }
    }

    Slice::from_text(text)
}

fn slice_is_empty(slice: &Slice) -> bool {
    slice.fragment.children.is_empty()
        && !matches!(
            slice.fragment.node,
            PlainNode::Text(_) | PlainNode::HardBreak(_)
        )
}

fn drop_internal_selection_at(
    editor: &mut Editor,
    position: Position,
    copy: bool,
) -> Result<(), EditorError> {
    let Some(source) = editor
        .state
        .selection
        .filter(|selection| !selection.is_collapsed())
    else {
        return Ok(());
    };
    let Some(slice) = Slice::extract(editor.state()) else {
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
