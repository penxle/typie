use crate::dnd::DndState;
use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::{DndDropPayload, DndOp, ExternalDndPayloadKind, InputModifiers};
use editor_clipboard::Slice;
use editor_commands::{self as commands};
use editor_model::{
    ChildView, ContextExpr, DocView, Fragment, Node, NodeType, PlainFileNode, PlainImageNode,
    PlainNode, PlainRootNode, Schema,
};
use editor_state::{Position, Selection, StableResolveCtx, StableSelection, State};
use editor_view::DropTarget;

#[derive(Debug, Clone, Copy)]
enum SelectionAfterDrop {
    KeepCommandSelection, // 파일/이미지: insert_slice_at의 기본 선택을 유지
    SelectInsertedRange,  // 텍스트: 삽입된 범위를 선택
}

pub fn handle_dnd_op(editor: &mut Editor, op: DndOp) -> Result<(), EditorError> {
    let previous_drop_target = editor.dnd.drop_target().cloned();
    let result = match op {
        DndOp::StartInternalSelection => {
            let view = editor.state.view();
            editor.dnd = editor
                .state
                .selection
                .as_ref()
                .filter(|selection| !selection.is_collapsed())
                .map(|selection| snap_to_block_unit(&view, selection))
                .filter(|selection| !selection.is_collapsed())
                .map_or(DndState::Idle, |source| DndState::InternalDnd {
                    source: StableSelection::capture(&source, &view),
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
            let target = match (editor.dnd.clone(), target) {
                (DndState::InternalDnd { .. }, Some(target)) => {
                    let live_position = resolve_drop_position(&target, &editor.state);
                    live_position.and_then(|live_position| {
                        internal_source.as_ref().and_then(|source| {
                            let inside = {
                                let view = editor.state.view();
                                position_inside_selection(&view, live_position, source)
                            };
                            (!inside
                                && can_apply_drop(
                                    editor,
                                    live_position,
                                    DndDropPayload::InternalSelection,
                                    modifiers,
                                    Some(*source),
                                ))
                            .then_some(target)
                        })
                    })
                }
                (DndState::ExternalDnd { payload, .. }, Some(target)) => {
                    let live_position = resolve_drop_position(&target, &editor.state);
                    live_position.and_then(|live_position| {
                        can_apply_drop(
                            editor,
                            live_position,
                            representative_external_payload(payload),
                            modifiers,
                            None,
                        )
                        .then_some(target)
                    })
                }
                _ => None,
            };
            editor.dnd.set_drop_target(target);
            Ok(())
        }
        DndOp::Drop {
            payload, modifiers, ..
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
                            let inside = {
                                let view = editor.state.view();
                                position_inside_selection(&view, position, source)
                            };
                            (!inside
                                && can_apply_drop(
                                    editor,
                                    position,
                                    DndDropPayload::InternalSelection,
                                    modifiers,
                                    Some(*source),
                                ))
                            .then_some(position)
                        })
                    }
                    _ => editor.dnd.drop_target().cloned().and_then(|target| {
                        let position = resolve_drop_position(&target, &editor.state)?;
                        can_apply_drop(editor, position, payload.clone(), modifiers, None)
                            .then_some(position)
                    }),
                }
            } else {
                None
            };
            let result = if let Some(position) = target {
                apply_drop(editor, position, payload, modifiers, internal_source)
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
    if editor.dnd.drop_target() != previous_drop_target.as_ref() {
        editor.invalidate_render();
    }
    result
}

fn resolve_drop_position(target: &DropTarget, state: &State) -> Option<Position> {
    let view = state.view();
    let ctx = StableResolveCtx::from_live(&view, state.projected.seq_checkout());
    target.position.resolve(&ctx)
}

fn position_inside_selection(view: &DocView, position: Position, selection: &Selection) -> bool {
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

/// anchor가 isolating+monolithic 블록(fold/table)의 parent 경계에 있으면
/// 해당 블록의 단위 선택으로 스냅한다. promote_cross_isolating이 만드는
/// (parent, fold_idx)→(external_pos) 선택을 DnD 소스로 정제하기 위해 사용.
fn snap_to_block_unit(view: &DocView, selection: &Selection) -> Selection {
    let anchor = selection.anchor;
    if let Some(node) = view.node(anchor.node) {
        let spec = node.spec();
        if !spec.is_textblock()
            && !spec.inline
            && let Some(ChildView::Block(child)) = node.children().nth(anchor.offset)
        {
            let child_spec = child.spec();
            if child_spec.isolating && child_spec.monolithic {
                let unit_end = Position::new(anchor.node, anchor.offset + 1);
                return Selection::new(anchor, unit_end);
            }
        }
    }
    *selection
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
            let (slice, provenance) = {
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
                provenance,
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
            commands::types::SliceProvenance::Formatted,
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

/// A drop is rejected when the slice carries a leaf whose schema context cannot
/// be satisfied at the drop target (e.g. a `page_break`, valid only directly
/// under `Root > Paragraph`, dropped into a paragraph nested in a blockquote).
/// Without this the inline-insert path silently relocates or drops such a leaf,
/// so the probe would wrongly accept the drop.
fn slice_content_fits_target_context(view: &DocView, position: Position, slice: &Slice) -> bool {
    let mut textblock = view.node(position.node);
    while let Some(node) = &textblock {
        if node.spec().is_textblock() {
            break;
        }
        textblock = node.parent();
    }
    let Some(textblock) = textblock else {
        return true;
    };
    let mut base: Vec<NodeType> = textblock.ancestors().map(|a| a.node_type()).collect();
    base.reverse();

    fn check(fragment: &Fragment, base: &[NodeType]) -> bool {
        let ty = fragment.node.as_type();
        let spec = Schema::node_spec(ty);
        if spec.is_leaf() && spec.context != ContextExpr::Any {
            let mut path = base.to_vec();
            path.push(ty);
            if !spec.context.matches(&path) {
                return false;
            }
        }
        fragment.children.iter().all(|c| check(c, base))
    }
    check(&slice.fragment, &base)
}

fn drop_slice_at(
    editor: &mut Editor,
    position: Position,
    slice: Slice,
    provenance: commands::types::SliceProvenance,
    selection: SelectionAfterDrop,
) -> Result<(), EditorError> {
    if !slice_content_fits_target_context(&editor.state.view(), position, &slice) {
        return Ok(());
    }
    editor.transact(|tr| {
        let inserted_selection =
            commands::insert_slice_at(tr, position, slice.clone(), provenance)?;
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
) -> (Slice, commands::types::SliceProvenance) {
    if let Some(html) = html.filter(|html| !html.is_empty()) {
        let slice = Slice::from_html(html, resource);
        if !slice.is_empty() {
            return (slice, commands::types::SliceProvenance::Formatted);
        }
    }

    (
        Slice::from_text(text),
        commands::types::SliceProvenance::Plain,
    )
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
    // the block would be deleted and re-inserted at the same position (with new Dots).
    // Detect and skip early so probe mode doesn't report a false "state changed".
    if !copy {
        let view = editor.state.view();
        if let Some(resolved) = source.resolve(&view) {
            let from = Position::from(resolved.from());
            let to = Position::from(resolved.to());
            if position == from || position == to {
                return Ok(());
            }
        }
    }

    let mut state = editor.state().clone();
    state.selection = Some(source);
    let Some(slice) = Slice::extract(&state) else {
        return Ok(());
    };

    if !slice_content_fits_target_context(&editor.state.view(), position, &slice) {
        return Ok(());
    }

    if copy {
        return drop_slice_at(
            editor,
            position,
            slice,
            commands::types::SliceProvenance::Formatted,
            SelectionAfterDrop::SelectInsertedRange,
        );
    }

    let stable_target =
        StableSelection::capture(&Selection::collapsed(position), &editor.state.view());
    editor.transact(|tr| {
        commands::set_selection(tr, source)?;
        commands::delete_selection(tr)?;

        let resolved_target = {
            let view = tr.view();
            let ctx = StableResolveCtx::from_live(&view, tr.state().projected.seq_checkout());
            stable_target.resolve(&ctx)
        };
        let Some(target) = resolved_target.map(|sel| sel.head) else {
            return Ok(());
        };
        if let Some(inserted_selection) = commands::insert_slice_at(
            tr,
            target,
            slice.clone(),
            commands::types::SliceProvenance::Formatted,
        )? && !inserted_selection.is_collapsed()
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
            carry: vec![],
            children,
        },
        open_start: 0,
        open_end: 0,
    }
}
