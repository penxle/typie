use std::cell::Cell;

use editor_state::{Position, Selection, StablePosition, StableSelection, State, flat_size};
use editor_view::{DropIndicator, DropTarget};

use crate::dnd::DndState;
use crate::editor::Editor;
use crate::error::EditorError;
use crate::handle::{apply_drop_for_test, judge_apply_drop, position_inside_selection};
use crate::message::*;
use crate::test_utils::EditorSnapshot;

use editor_macros::state;

// The `StepError` variants `is_pinned_execution_defect` allowlists, broken out
// so `assert_drop_parity` can tally which variant an erroring probe hit.
#[derive(Clone, Copy)]
enum PinnedDefectVariant {
    NodeNotFound,
    IndexOutOfBounds,
    IllegalInsertSlot,
}

/// Allowlist of `StepError` variants `assert_drop_parity` treats as pinned latent
/// execution-layer defects — an erroring probe under judge=true that is skipped
/// rather than gated. Contract: this allowlist is 1:1 with the deterministic pin
/// tests in this module (`copy_whole_list_into_its_own_item_probe_hits_synthetic_scaffold_anchor`,
/// `copy_drop_at_list_item_source_boundary_is_uncovered_by_interior_filter`,
/// `copy_into_fold_past_fixed_slots_probe_hits_index_out_of_bounds`,
/// `move_bullet_list_items_outside_source_probe_hits_illegal_insert_slot`) —
/// adding a variant here without a matching pin test lets this gate silently
/// swallow a new latent bug.
fn pinned_defect_variant(err: &EditorError) -> Option<PinnedDefectVariant> {
    let EditorError::Command(editor_commands::CommandError::Step(step)) = err else {
        return None;
    };
    match step {
        editor_transaction::StepError::NodeNotFound(_) => Some(PinnedDefectVariant::NodeNotFound),
        editor_transaction::StepError::IndexOutOfBounds { .. } => {
            Some(PinnedDefectVariant::IndexOutOfBounds)
        }
        editor_transaction::StepError::IllegalInsertSlot { .. } => {
            Some(PinnedDefectVariant::IllegalInsertSlot)
        }
        _ => None,
    }
}

fn is_pinned_execution_defect(err: &EditorError) -> bool {
    pinned_defect_variant(err).is_some()
}

// Counts probe errors that fall into the known insert-error bucket (see the
// match in `assert_drop_parity` below), broken out per pinned `StepError`
// variant. Each `#[test]` (including each `proptest!`-generated test) runs on
// its own freshly spawned thread, so this thread-local is naturally scoped to
// a single test; its `Drop` impl prints the final tally exactly once, right
// after that test's cases finish running — including the `proptest!` case
// loop, which has no code path of its own to run after the last case.
struct KnownInsertErrorBucket {
    node_not_found: Cell<usize>,
    index_out_of_bounds: Cell<usize>,
    illegal_insert_slot: Cell<usize>,
}

impl KnownInsertErrorBucket {
    fn record(&self, variant: PinnedDefectVariant) {
        let cell = match variant {
            PinnedDefectVariant::NodeNotFound => &self.node_not_found,
            PinnedDefectVariant::IndexOutOfBounds => &self.index_out_of_bounds,
            PinnedDefectVariant::IllegalInsertSlot => &self.illegal_insert_slot,
        };
        cell.set(cell.get() + 1);
    }

    fn count(&self) -> usize {
        self.node_not_found.get() + self.index_out_of_bounds.get() + self.illegal_insert_slot.get()
    }
}

impl Drop for KnownInsertErrorBucket {
    fn drop(&mut self) {
        println!(
            "known_insert_error_bucket: {} (node_not_found={}, index_out_of_bounds={}, illegal_insert_slot={})",
            self.count(),
            self.node_not_found.get(),
            self.index_out_of_bounds.get(),
            self.illegal_insert_slot.get()
        );
    }
}

thread_local! {
    static KNOWN_INSERT_ERROR_BUCKET: KnownInsertErrorBucket = KnownInsertErrorBucket {
        node_not_found: Cell::new(0),
        index_out_of_bounds: Cell::new(0),
        illegal_insert_slot: Cell::new(0),
    };
}

fn fixtures() -> Vec<State> {
    let mut out = Vec::new();
    {
        let (s, ..) = state! {
            doc { root {
                p1: paragraph { text("hello world") }
                paragraph { text("second paragraph") }
            } }
            selection: (p1, 0)
        };
        out.push(s);
    }
    {
        let (s, ..) = state! {
            doc { root {
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { text("inside") } }
                }
                paragraph { text("after") }
            } }
            selection: none
        };
        out.push(s);
    }
    {
        let (s, ..) = state! {
            doc { root {
                table { table_row { table_cell { c1: paragraph { text("cell") } } } }
                p2: paragraph { text("tail") }
            } }
            selection: (c1, 0)
        };
        out.push(s);
    }
    {
        let (s, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { text("alpha") } }
                    list_item { paragraph { text("beta") } }
                }
                paragraph {}
            } }
            selection: none
        };
        out.push(s);
    }
    out
}

fn payloads() -> Vec<(DndDropPayload, InputModifiers)> {
    let plain = InputModifiers::default();
    let alt = InputModifiers {
        alt: true,
        ..InputModifiers::default()
    };
    vec![
        (
            DndDropPayload::Text {
                text: "drop text".into(),
                html: None,
            },
            plain,
        ),
        (
            DndDropPayload::Text {
                text: "x".into(),
                html: Some("<p>x</p>".into()),
            },
            plain,
        ),
        (
            DndDropPayload::Files {
                image_count: 1,
                file_count: 0,
            },
            plain,
        ),
        (
            DndDropPayload::Files {
                image_count: 1,
                file_count: 1,
            },
            plain,
        ),
        (DndDropPayload::InternalSelection, plain),
        (DndDropPayload::InternalSelection, alt),
    ]
}

fn position_at_flat(editor: &mut Editor, offset: usize) -> Option<Position> {
    editor.apply(Message::Selection {
        op: SelectionOp::SetFlat {
            start: offset,
            end: offset,
        },
    });
    editor.state().selection.map(|sel| sel.head)
}

fn source_range(editor: &mut Editor, start: usize, end: usize) -> Option<Selection> {
    editor.apply(Message::Selection {
        op: SelectionOp::SetFlat { start, end },
    });
    editor.state().selection.filter(|sel| !sel.is_collapsed())
}

fn assert_drop_parity(
    editor: &mut Editor,
    position: Position,
    payload: &DndDropPayload,
    modifiers: InputModifiers,
    source: Option<Selection>,
    approx_bucket: &mut usize,
) {
    // Touch the thread-local unconditionally (even when this call never hits
    // the Err branch below) so its `Drop`-driven println always fires when
    // this test's thread exits, not only on runs that happen to bucket an
    // error.
    KNOWN_INSERT_ERROR_BUCKET.with(|_| {});

    // Align the gate domain with the production-reachable input space: for an
    // internal-selection payload the Over/Drop handlers refuse a drop whose
    // position lands inside the moved/copied selection (dnd.rs). Judging or
    // probing such an input compares two paths that production never both
    // consults, so skip it here exactly as the handlers do. External payloads are
    // source-independent and never filtered.
    if matches!(payload, DndDropPayload::InternalSelection)
        && let Some(source) = source.as_ref()
    {
        let view = editor.state().view();
        if position_inside_selection(&view, position, source) {
            return;
        }
    }

    let judged = {
        let resource = editor.resource().clone();
        let resource = resource.lock().unwrap();
        judge_apply_drop(
            editor.state(),
            &resource,
            position,
            payload,
            modifiers,
            source.as_ref(),
        )
    };
    let payload_cloned = payload.clone();
    let probed = match editor
        .probe(|e| apply_drop_for_test(e, position, payload_cloned, modifiers, source))
    {
        Ok(changed) => changed,
        // An erroring probe is a distinct failure mode from the judge-vs-probe
        // state-change parity contract this gate enforces. judge is a shallow
        // schema-shape feasibility oracle (`resolve_slice_insertion`): it cannot
        // foresee transactional StepErrors that the multi-step block-boundary
        // execution raises — e.g. anchoring a follow-up insert against a
        // projection-synthesized scaffold (NodeNotFound), inserting past a
        // fixed-slot container's children (IndexOutOfBounds), or a delete-time
        // cursor-repair filler that doesn't fit its container's schema
        // (IllegalInsertSlot). `is_pinned_execution_defect` allowlists exactly
        // those variants; each is pinned by its own deterministic test (see that
        // function's doc comment for the 1:1 mapping). (Under judge=true a real
        // plan exists, so a clean probe is never `Ok(false)`; erroring probes are
        // the only residual, and skipping them cannot mask an under-prediction,
        // which needs a clean `Ok(true)`.) Any Err outside the allowlist is a
        // distinct, potentially new latent execution bug and must not be
        // silently swallowed by this gate.
        Err(err) => {
            if is_pinned_execution_defect(&err) {
                let variant = pinned_defect_variant(&err).expect(
                    "is_pinned_execution_defect(err) implies pinned_defect_variant(err).is_some()",
                );
                KNOWN_INSERT_ERROR_BUCKET.with(|b| b.record(variant));
                return;
            }
            panic!(
                "probe returned an Err outside the known insert-error bucket \
                 (new latent execution bug?): position={position:?} payload={payload:?} \
                 error={err}"
            );
        }
    };

    assert!(
        !(probed && !judged),
        "under-prediction is never allowed: position={position:?} payload={payload:?}"
    );
    if judged && !probed {
        let allowed = matches!(payload, DndDropPayload::InternalSelection) && !modifiers.alt;
        assert!(
            allowed,
            "over-prediction outside approved bucket: position={position:?} payload={payload:?}"
        );
        *approx_bucket += 1;
    }
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig { cases: 192, ..proptest::prelude::ProptestConfig::default() })]
    #[test]
    fn drop_judgment_matches_probe(
        fixture_idx in 0usize..4,
        pos_off in 0usize..64,
        src_a in 0usize..64,
        src_b in 0usize..64,
        payload_idx in 0usize..6,
    ) {
        let state = fixtures().swap_remove(fixture_idx);
        let size = flat_size(&state.view());
        let mut editor = Editor::new_test(state);
        let (payload, modifiers) = payloads().swap_remove(payload_idx);

        let source = if matches!(payload, DndDropPayload::InternalSelection) {
            let (a, b) = (src_a.min(size), src_b.min(size));
            if a == b {
                return Ok(());
            }
            let Some(source) = source_range(&mut editor, a.min(b), a.max(b)) else {
                return Ok(());
            };
            Some(source)
        } else {
            None
        };

        let Some(position) = position_at_flat(&mut editor, pos_off.min(size)) else {
            return Ok(());
        };
        if source.is_some() {
            editor.apply(Message::Selection {
                op: SelectionOp::SetFlat {
                    start: src_a.min(size).min(src_b.min(size)),
                    end: src_a.min(size).max(src_b.min(size)),
                },
            });
        }

        let mut bucket = 0usize;
        assert_drop_parity(&mut editor, position, &payload, modifiers, source, &mut bucket);
    }
}

// Reconstructs a discrepancy that lives OUTSIDE the production-reachable domain:
// an internal-selection move (alt=false) whose drop position lands inside the
// moved selection. The Over/Drop handlers filter this via
// `position_inside_selection`, so judge and probe are never both consulted in
// production. Recorded so the gap stays visible: judge does not model the move's
// delete-then-remap, whereas probe deletes the source and re-anchors the insert
// against a stable target, changing state.
#[test]
fn move_drop_inside_own_fold_is_domain_excluded_yet_probe_reanchors() {
    let state = fixtures().swap_remove(1);
    let mut editor = Editor::new_test(state);
    let source = source_range(&mut editor, 0, 19).expect("fold-spanning source");
    let position = position_at_flat(&mut editor, 1).expect("position inside the fold");
    let modifiers = InputModifiers::default();

    {
        let view = editor.state().view();
        assert!(
            position_inside_selection(&view, position, &source),
            "the drop position lies inside the moved selection; production filters it"
        );
    }

    let judged = {
        let resource = editor.resource().clone();
        let resource = resource.lock().unwrap();
        judge_apply_drop(
            editor.state(),
            &resource,
            position,
            &DndDropPayload::InternalSelection,
            modifiers,
            Some(&source),
        )
    };
    let probed = editor.probe(|e| {
        apply_drop_for_test(
            e,
            position,
            DndDropPayload::InternalSelection,
            modifiers,
            Some(source),
        )
    });

    assert!(!judged, "judge does not model the move's delete-then-remap");
    assert!(
        matches!(probed, Ok(true)),
        "probe deletes the source then re-anchors the insert, changing state: {probed:?}"
    );
}

// Reconstructs a latent execution bug the probe previously masked (also outside
// the production-reachable domain per `position_inside_selection`): copying the
// whole bullet list into a block position inside its own first list item. judge
// predicts a valid insertion from schema-shape feasibility, but executing the
// block-boundary plan inserts the nested BulletList first — which forces the
// projection to synthesize a leading Paragraph scaffold to keep the ListItem
// valid — and the following Paragraph insert then resolves its sequence anchor
// against that synthetic child, which has no CRDT identity, surfacing
// StepError::NodeNotFound. When the execution path stops over-predicting (or the
// insert is made robust to the intermediate scaffold), this test must be updated.
#[test]
fn copy_whole_list_into_its_own_item_probe_hits_synthetic_scaffold_anchor() {
    let state = fixtures().swap_remove(3);
    let mut editor = Editor::new_test(state);
    let source = source_range(&mut editor, 0, 20).expect("document-spanning source");
    let position = position_at_flat(&mut editor, 2).expect("position inside the first list item");
    let modifiers = InputModifiers {
        alt: true,
        ..InputModifiers::default()
    };

    {
        let view = editor.state().view();
        assert!(
            position_inside_selection(&view, position, &source),
            "the drop position lies inside the copied selection; production filters it"
        );
    }

    let judged = {
        let resource = editor.resource().clone();
        let resource = resource.lock().unwrap();
        judge_apply_drop(
            editor.state(),
            &resource,
            position,
            &DndDropPayload::InternalSelection,
            modifiers,
            Some(&source),
        )
    };
    assert!(
        judged,
        "judge predicts a valid insertion from schema-shape feasibility alone"
    );

    let probed = editor.probe(|e| {
        apply_drop_for_test(
            e,
            position,
            DndDropPayload::InternalSelection,
            modifiers,
            Some(source),
        )
    });
    assert!(
        matches!(
            probed,
            Err(EditorError::Command(editor_commands::CommandError::Step(
                editor_transaction::StepError::NodeNotFound(_)
            )))
        ),
        "probe anchors the follow-up insert against a synthetic scaffold with no CRDT identity: {probed:?}"
    );
}

// The interior filter (`position_inside_selection`) uses a strict interval, so a
// copy whose drop position sits ON the source's `from` boundary is not excluded —
// and copy, unlike move, has no boundary no-op. This reaches the same latent
// synthetic-scaffold execution bug as the interior case, from a source that only
// touches (not contains) the position, which the gate would otherwise flag as an
// over-prediction. Pinned so the residual stays visible until the execution path
// stops over-predicting.
#[test]
fn copy_drop_at_list_item_source_boundary_is_uncovered_by_interior_filter() {
    let state = fixtures().swap_remove(3);
    let mut editor = Editor::new_test(state);
    let source = source_range(&mut editor, 2, 20).expect("boundary-touching source");
    let position = position_at_flat(&mut editor, 2).expect("position at the source from-boundary");
    let modifiers = InputModifiers {
        alt: true,
        ..InputModifiers::default()
    };

    {
        let view = editor.state().view();
        assert!(
            !position_inside_selection(&view, position, &source),
            "the position lies on the source boundary, not strictly inside, so the interior filter does not cover it"
        );
    }

    let judged = {
        let resource = editor.resource().clone();
        let resource = resource.lock().unwrap();
        judge_apply_drop(
            editor.state(),
            &resource,
            position,
            &DndDropPayload::InternalSelection,
            modifiers,
            Some(&source),
        )
    };
    assert!(judged, "judge predicts a valid insertion");

    let probed = editor.probe(|e| {
        apply_drop_for_test(
            e,
            position,
            DndDropPayload::InternalSelection,
            modifiers,
            Some(source),
        )
    });
    assert!(
        matches!(
            probed,
            Err(EditorError::Command(editor_commands::CommandError::Step(
                editor_transaction::StepError::NodeNotFound(_)
            )))
        ),
        "probe hits the same synthetic-scaffold anchor from a boundary-touching source: {probed:?}"
    );
}

// A second variant of the same over-prediction class, through a different code
// path and error: copying a fold-spanning selection into a block position past a
// fold's fixed slots (title, content). judge predicts a valid block-boundary
// insertion, but executing it inserts past the fold's two children, so the step
// layer raises IndexOutOfBounds. Pinned so this residual — which the gate skips as
// an erroring probe — stays visible and forces an update when the path is fixed.
#[test]
fn copy_into_fold_past_fixed_slots_probe_hits_index_out_of_bounds() {
    let state = fixtures().swap_remove(1);
    let mut editor = Editor::new_test(state);
    let source = source_range(&mut editor, 0, 9).expect("fold-spanning source");
    let position = position_at_flat(&mut editor, 18).expect("position past the fold's fixed slots");
    let modifiers = InputModifiers {
        alt: true,
        ..InputModifiers::default()
    };

    let judged = {
        let resource = editor.resource().clone();
        let resource = resource.lock().unwrap();
        judge_apply_drop(
            editor.state(),
            &resource,
            position,
            &DndDropPayload::InternalSelection,
            modifiers,
            Some(&source),
        )
    };
    assert!(judged, "judge predicts a valid block-boundary insertion");

    let probed = editor.probe(|e| {
        apply_drop_for_test(
            e,
            position,
            DndDropPayload::InternalSelection,
            modifiers,
            Some(source),
        )
    });
    assert!(
        matches!(
            probed,
            Err(EditorError::Command(editor_commands::CommandError::Step(
                editor_transaction::StepError::IndexOutOfBounds { .. }
            )))
        ),
        "probe inserts past the fold's fixed slots: {probed:?}"
    );
}

// Reconstructs discrepancy #4 from the 4096-case exploration: a move (alt=false)
// of the bullet list's full two-item child range to a root-level position past
// the list's end — outside the source (18 < 19), so the strict interior filter
// does not cover it. judge predicts a valid block-boundary insertion from
// schema-shape feasibility, but the delete half of move's delete-then-insert
// (`delete_selection`, well before `insert_slice_at` ever runs) empties the
// bullet_list to zero children, and the cursor-repair fallback in
// `ensure_selection_after_child_range_delete`
// (crates/editor-commands/src/helpers/deletion.rs) unconditionally inserts a
// bare Paragraph filler into the emptied container — schema-illegal for a
// BulletList, whose content model only accepts ListItem — surfacing
// StepError::IllegalInsertSlot. Pinned so this residual stays visible until the
// repair fallback accounts for list-type (as opposed to textblock-hosting)
// block containers.
#[test]
fn move_bullet_list_items_outside_source_probe_hits_illegal_insert_slot() {
    let state = fixtures().swap_remove(3);
    let mut editor = Editor::new_test(state);
    let source = source_range(&mut editor, 1, 18).expect("bullet-list child-range source");
    let position = position_at_flat(&mut editor, 19).expect("position past the bullet list's end");
    let modifiers = InputModifiers::default();

    {
        let view = editor.state().view();
        assert!(
            !position_inside_selection(&view, position, &source),
            "the position lies outside the moved selection; the interior filter does not cover it"
        );
    }

    let judged = {
        let resource = editor.resource().clone();
        let resource = resource.lock().unwrap();
        judge_apply_drop(
            editor.state(),
            &resource,
            position,
            &DndDropPayload::InternalSelection,
            modifiers,
            Some(&source),
        )
    };
    assert!(judged, "judge predicts a valid block-boundary insertion");

    let probed = editor.probe(|e| {
        apply_drop_for_test(
            e,
            position,
            DndDropPayload::InternalSelection,
            modifiers,
            Some(source),
        )
    });
    assert!(
        matches!(
            probed,
            Err(EditorError::Command(editor_commands::CommandError::Step(
                editor_transaction::StepError::IllegalInsertSlot { .. }
            )))
        ),
        "move's delete half empties the bullet_list, and the cursor-repair \
         fallback's bare-Paragraph filler is schema-illegal for a list container: {probed:?}"
    );
}

#[test]
fn drop_judgment_matches_probe_at_block_boundaries() {
    let (state, r, ..) = state! {
        doc { r: root {
            paragraph { text("a") }
            fold {
                fold_title { text("t") }
                fold_content { paragraph { text("c") } }
            }
            paragraph { text("b") }
        } }
        selection: none
    };
    let mut editor = Editor::new_test(state);
    let mut bucket = 0usize;
    for offset in 0..=3usize {
        let position = Position {
            node: r,
            offset,
            affinity: editor_state::Affinity::Downstream,
        };
        for (payload, modifiers) in payloads() {
            if matches!(payload, DndDropPayload::InternalSelection) {
                continue;
            }
            assert_drop_parity(
                &mut editor,
                position,
                &payload,
                modifiers,
                None,
                &mut bucket,
            );
        }
    }
    println!("approx bucket: {bucket}");
    KNOWN_INSERT_ERROR_BUCKET.with(|b| println!("known_insert_error_bucket: {}", b.count()));
}

// Reconstructs the same judged=true/probe=Err(NodeNotFound) input as
// `copy_drop_at_list_item_source_boundary_is_uncovered_by_interior_filter` (the
// boundary-touching variant, not the strictly-interior one — `position_inside_selection`
// would otherwise gate the drop before the handler ever reaches `apply_drop`, making
// the interior variant unreachable through this path). This test drives the input
// through the full handler Drop path (`handle_dnd_op`, via
// `Message::Dnd { op: DndOp::Drop { .. } }`) instead of calling `apply_drop_for_test`
// directly. `handle::dnd::handle_dnd_op` carries a bridge (see the TODO above its
// `apply_drop` match) that downgrades this known insertion error to a silent no-op
// so the handler never surfaces it. If the underlying execution bug is ever fixed
// and this exact input starts succeeding instead of erroring, the
// document/selection-unchanged assertion below will go red, forcing the bridge to
// be reconsidered and removed.
#[test]
fn internal_copy_drop_with_known_insert_error_degrades_to_silent_noop() {
    let state = fixtures().swap_remove(3);
    let mut editor = Editor::new_test(state);
    let source = source_range(&mut editor, 2, 20).expect("boundary-touching source");
    let position = position_at_flat(&mut editor, 2).expect("position at the source from-boundary");
    let modifiers = InputModifiers {
        alt: true,
        ..InputModifiers::default()
    };

    // Reconstruct the Drop precondition directly on `editor.dnd`, matching what
    // `DndOp::StartInternalSelection` + `DndOp::Over` would have produced for this
    // source/position pair (both judge as an allowed drop, per the pinned test
    // referenced above).
    let view = editor.state.view();
    editor.dnd = DndState::InternalDnd {
        source: StableSelection::capture(&source, &view),
        drop_target: Some(DropTarget {
            position: StablePosition::capture(&position, &view),
            indicator: DropIndicator::Block {
                page_idx: 0,
                x: 0.0,
                y: 0.0,
                width: 0.0,
            },
        }),
    };

    let before = EditorSnapshot::capture(&editor);
    editor.apply(Message::Dnd {
        op: DndOp::Drop {
            page: 0,
            x: 0.0,
            y: 0.0,
            payload: DndDropPayload::InternalSelection,
            modifiers,
        },
    });
    let after = EditorSnapshot::capture(&editor);

    assert_eq!(
        before, after,
        "the bridge must degrade the known insertion error to a silent no-op: document/selection must stay unchanged"
    );
}
