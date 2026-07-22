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
// so `assert_drop_parity` can tally which variant an erroring execution hit.
#[derive(Clone, Copy)]
enum PinnedDefectVariant {
    NodeNotFound,
    IndexOutOfBounds,
    IllegalInsertSlot,
}

/// Allowlist of `StepError` variants `assert_drop_parity` treats as pinned latent
/// execution-layer defects — an erroring execution under judge=true that is
/// skipped rather than gated. Contract: this allowlist is 1:1 with the
/// deterministic pin tests in this module
/// (`copy_whole_list_into_its_own_item_execution_hits_synthetic_scaffold_anchor`,
/// `copy_drop_at_list_item_source_boundary_is_uncovered_by_interior_filter`,
/// `copy_into_fold_past_fixed_slots_execution_hits_index_out_of_bounds`,
/// `move_bullet_list_items_outside_source_execution_hits_illegal_insert_slot`) —
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

// Counts execution errors that fall into the known insert-error bucket (see the
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
) {
    // Touch the thread-local unconditionally (even when this call never hits
    // the Err branch below) so its `Drop`-driven println always fires when
    // this test's thread exits, not only on runs that happen to bucket an
    // error.
    KNOWN_INSERT_ERROR_BUCKET.with(|_| {});

    // Align the gate domain with the production-reachable input space: for an
    // internal-selection payload the Over/Drop handlers refuse a drop whose
    // position lands inside the moved/copied selection (dnd.rs). Judging or
    // executing such an input compares two paths that production never both
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
    let (run_result, run_changed) = {
        let mut scratch = Editor::new_test(editor.state().clone());
        let before = crate::test_utils::EditorSnapshot::capture(&scratch);
        let result =
            apply_drop_for_test(&mut scratch, position, payload.clone(), modifiers, source);
        let after = crate::test_utils::EditorSnapshot::capture(&scratch);
        (result, before != after)
    };

    let executed = match run_result {
        Ok(()) => run_changed,
        // An erroring execution is a distinct failure mode from the
        // judge-vs-execution state-change parity contract this gate enforces.
        // judge is a shallow schema-shape feasibility oracle
        // (`resolve_slice_insertion`): it cannot foresee transactional
        // StepErrors that the multi-step block-boundary execution raises —
        // e.g. anchoring a follow-up insert against a projection-synthesized
        // scaffold (NodeNotFound), inserting past a fixed-slot container's
        // children (IndexOutOfBounds), or a delete-time cursor-repair filler
        // that doesn't fit its container's schema (IllegalInsertSlot).
        // `is_pinned_execution_defect` allowlists exactly those variants; each
        // is pinned by its own deterministic test (see that function's doc
        // comment for the 1:1 mapping). (Under judge=true a real plan exists,
        // so a clean execution is never `Ok(false)`; erroring executions are
        // the only residual, and skipping them cannot mask an
        // under-prediction, which needs a clean `Ok(true)`.) Any Err outside
        // the allowlist is a distinct, potentially new latent execution bug
        // and must not be silently swallowed by this gate.
        Err(err) => {
            if is_pinned_execution_defect(&err) {
                let variant = pinned_defect_variant(&err).expect(
                    "is_pinned_execution_defect(err) implies pinned_defect_variant(err).is_some()",
                );
                KNOWN_INSERT_ERROR_BUCKET.with(|b| b.record(variant));
                assert!(
                    !run_changed,
                    "pinned execution defect must not mutate state"
                );
                return;
            }
            panic!(
                "drop execution returned an Err outside the known insert-error bucket \
                 (new latent execution bug?): position={position:?} payload={payload:?} \
                 error={err}"
            );
        }
    };

    assert_eq!(
        judged, executed,
        "judge must match execution exactly (no under- or over-prediction): \
         position={position:?} payload={payload:?}"
    );
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig { cases: 192, ..proptest::prelude::ProptestConfig::default() })]
    #[test]
    fn drop_judgment_matches_execution(
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

        assert_drop_parity(&mut editor, position, &payload, modifiers, source);
    }
}

// Pins the input that `assert_drop_parity`'s 4096-case sweep once surfaced as a
// `judge_apply_drop` under-prediction (fixture 3, InternalSelection move,
// pos_off=13/src_a=12/src_b=3): `resolve_slice_insertion` against the raw
// pre-delete position gets `NoFit`, yet the real drop execution changes state
// because the move's delete-then-remap (`drop_internal_selection_at`) re-anchors
// the drop through a `StableSelection` captured before the delete and resolved
// after it — landing on a different node than the raw position — where the slice
// does fit. The judge now routes every move unconditionally through that same
// delete-then-remap sequence (`move_insertion_fits_after_delete`) — the raw
// pre-delete position is never consulted for a move — so judge follows the
// execution here and reports `true`. This test pins that the two agree; it is
// also protected by the sweep's exact-equality assertion (the input is pinned in
// proptest-regressions/tests/dnd_judgment_parity.txt).
#[test]
fn move_bullet_list_child_range_drop_judge_follows_delete_then_remap() {
    let state = fixtures().swap_remove(3);
    let mut editor = Editor::new_test(state);
    let source = source_range(&mut editor, 3, 12).expect("bullet-list child-range source");
    let position = position_at_flat(&mut editor, 13).expect("position past the source's end");
    editor.apply(Message::Selection {
        op: SelectionOp::SetFlat { start: 3, end: 12 },
    });
    let modifiers = InputModifiers::default();

    {
        let view = editor.state().view();
        assert!(
            !position_inside_selection(&view, position, &source),
            "position lies outside the moved selection; not covered by the interior filter"
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
        "judge's move simulation mirrors the delete-then-remap and finds the slice fits the re-anchored target"
    );

    let (run_result, run_changed) = {
        let mut scratch = Editor::new_test(editor.state().clone());
        let before = crate::test_utils::EditorSnapshot::capture(&scratch);
        let result = apply_drop_for_test(
            &mut scratch,
            position,
            DndDropPayload::InternalSelection,
            modifiers,
            Some(source),
        );
        let after = crate::test_utils::EditorSnapshot::capture(&scratch);
        (result, before != after)
    };
    assert!(
        matches!(run_result, Ok(())) && run_changed,
        "the move's delete-then-remap re-anchors the insert past the raw judge position, changing state: {run_result:?}"
    );
}

// Pins the seed-51ac09 input (fixture 1, InternalSelection move,
// pos_off=16/src_a=25/src_b=26) that surfaced the "plan-then-no-op" class. The
// drop position (flat 16, inside the fold) lies outside the source (flat 25..26,
// the document tail) and off its boundary, so neither `position_inside_selection`
// nor the move boundary no-op filters it — production-reachable. The judge routes
// this move through `move_insertion_fits_after_delete`, which deletes the source
// and re-anchors to the SAME target the execution reaches (Dot{1,17}@6). The
// extracted slice there is a single EMPTY paragraph; at the textblock's end that
// splice is structurally valid (`can_splice_textblock` = true) but emits zero ops
// (the block is consumed by the start-edge join and contributes no inline), so the
// execution's `insert_slice_at` returns `Ok(None)` — a clean no-op. Previously
// `resolve_slice_insertion` still reported `Some(SpliceBlocks)` there, violating
// its own contract ("Some(plan) ⇒ observable change") and over-predicting. The
// contract is now upheld at the source: `resolve_slice_insertion_outcome`'s
// SpliceBlocks branch is guarded by `splice_emits_change` (an exact mirror of
// `insert_blocks_in_textblock`'s Ok(Some) condition), so resolve returns NoFit and
// the move simulation reports `false`. judge and execution now agree exactly (both
// no-op); the fix benefits every `resolve_slice_insertion` consumer, not just DnD.
#[test]
fn move_empty_splice_judged_as_noop_matching_execution() {
    let state = fixtures().swap_remove(1);
    let mut editor = Editor::new_test(state);
    let source = source_range(&mut editor, 25, 26).expect("document-tail source");
    let position = position_at_flat(&mut editor, 16).expect("position inside the fold");
    editor.apply(Message::Selection {
        op: SelectionOp::SetFlat { start: 25, end: 26 },
    });
    let modifiers = InputModifiers::default();

    {
        let view = editor.state().view();
        assert!(
            !position_inside_selection(&view, position, &source),
            "the drop position lies outside the moved selection; production does not filter it"
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
        !judged,
        "resolve now returns NoFit for the empty splice, so the move simulation predicts the no-op"
    );

    let (run_result, run_changed) = {
        let mut scratch = Editor::new_test(editor.state().clone());
        let before = crate::test_utils::EditorSnapshot::capture(&scratch);
        let result = apply_drop_for_test(
            &mut scratch,
            position,
            DndDropPayload::InternalSelection,
            modifiers,
            Some(source),
        );
        let after = crate::test_utils::EditorSnapshot::capture(&scratch);
        (result, before != after)
    };
    assert!(
        matches!(run_result, Ok(())) && !run_changed,
        "the move's actual insert_slice_at at the same target returns Ok(None), rolling back to a clean no-op: {run_result:?}"
    );
}

// Characterizes an input that lives OUTSIDE the production-reachable domain: an
// internal-selection move (alt=false) whose drop position lands inside the moved
// selection. The Over/Drop handlers filter this via `position_inside_selection`,
// so judge and execution are never both consulted in production. Recorded to keep
// the domain-exclusion property visible. Because the judge now routes every move
// through the same delete-then-remap simulation the execution runs — deleting the
// source and re-anchoring the insert against a stable target — both change state
// and judge agrees with execution.
#[test]
fn move_drop_inside_own_fold_is_domain_excluded_yet_execution_reanchors() {
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
    let (run_result, run_changed) = {
        let mut scratch = Editor::new_test(editor.state().clone());
        let before = crate::test_utils::EditorSnapshot::capture(&scratch);
        let result = apply_drop_for_test(
            &mut scratch,
            position,
            DndDropPayload::InternalSelection,
            modifiers,
            Some(source),
        );
        let after = crate::test_utils::EditorSnapshot::capture(&scratch);
        (result, before != after)
    };

    assert!(
        judged,
        "judge's move simulation models the delete-then-remap and agrees with the execution"
    );
    assert!(
        matches!(run_result, Ok(())) && run_changed,
        "drop execution deletes the source then re-anchors the insert, changing state: {run_result:?}"
    );
}

// Reconstructs a latent execution bug that schema-shape judging previously
// masked (also outside the production-reachable domain per
// `position_inside_selection`): copying the whole bullet list into a block
// position inside its own first list item. judge predicts a valid insertion
// from schema-shape feasibility, but executing the block-boundary plan inserts
// the nested BulletList first — which forces the projection to synthesize a
// leading Paragraph scaffold to keep the ListItem valid — and the following
// Paragraph insert then resolves its sequence anchor against that synthetic
// child, which has no CRDT identity, surfacing StepError::NodeNotFound. When
// the execution path stops over-predicting (or the insert is made robust to
// the intermediate scaffold), this test must be updated.
#[test]
fn copy_whole_list_into_its_own_item_execution_hits_synthetic_scaffold_anchor() {
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

    let (run_result, _run_changed) = {
        let mut scratch = Editor::new_test(editor.state().clone());
        let before = crate::test_utils::EditorSnapshot::capture(&scratch);
        let result = apply_drop_for_test(
            &mut scratch,
            position,
            DndDropPayload::InternalSelection,
            modifiers,
            Some(source),
        );
        let after = crate::test_utils::EditorSnapshot::capture(&scratch);
        (result, before != after)
    };
    assert!(
        matches!(
            run_result,
            Err(EditorError::Command(editor_commands::CommandError::Step(
                editor_transaction::StepError::NodeNotFound(_)
            )))
        ),
        "drop execution anchors the follow-up insert against a synthetic scaffold with no CRDT identity: {run_result:?}"
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

    let (run_result, _run_changed) = {
        let mut scratch = Editor::new_test(editor.state().clone());
        let before = crate::test_utils::EditorSnapshot::capture(&scratch);
        let result = apply_drop_for_test(
            &mut scratch,
            position,
            DndDropPayload::InternalSelection,
            modifiers,
            Some(source),
        );
        let after = crate::test_utils::EditorSnapshot::capture(&scratch);
        (result, before != after)
    };
    assert!(
        matches!(
            run_result,
            Err(EditorError::Command(editor_commands::CommandError::Step(
                editor_transaction::StepError::NodeNotFound(_)
            )))
        ),
        "drop execution hits the same synthetic-scaffold anchor from a boundary-touching source: {run_result:?}"
    );
}

// A second variant of the same over-prediction class, through a different code
// path and error: copying a fold-spanning selection into a block position past a
// fold's fixed slots (title, content). judge predicts a valid block-boundary
// insertion, but executing it inserts past the fold's two children, so the step
// layer raises IndexOutOfBounds. Pinned so this residual — which the gate skips as
// an erroring execution — stays visible and forces an update when the path is fixed.
#[test]
fn copy_into_fold_past_fixed_slots_execution_hits_index_out_of_bounds() {
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

    let (run_result, _run_changed) = {
        let mut scratch = Editor::new_test(editor.state().clone());
        let before = crate::test_utils::EditorSnapshot::capture(&scratch);
        let result = apply_drop_for_test(
            &mut scratch,
            position,
            DndDropPayload::InternalSelection,
            modifiers,
            Some(source),
        );
        let after = crate::test_utils::EditorSnapshot::capture(&scratch);
        (result, before != after)
    };
    assert!(
        matches!(
            run_result,
            Err(EditorError::Command(editor_commands::CommandError::Step(
                editor_transaction::StepError::IndexOutOfBounds { .. }
            )))
        ),
        "drop execution inserts past the fold's fixed slots: {run_result:?}"
    );
}

// Reconstructs discrepancy #4 from the 4096-case exploration: a move (alt=false)
// of the bullet list's full two-item child range to a root-level position past
// the list's end — outside the source (18 < 19), so the strict interior filter
// does not cover it. The delete half of move's delete-then-insert
// (`delete_selection`, well before `insert_slice_at` ever runs) empties the
// bullet_list to zero children, and the cursor-repair fallback in
// `ensure_selection_after_child_range_delete`
// (crates/editor-commands/src/helpers/deletion.rs) unconditionally inserts a
// bare Paragraph filler into the emptied container — schema-illegal for a
// BulletList, whose content model only accepts ListItem — surfacing
// StepError::IllegalInsertSlot. Because the judge now routes every move through
// the same delete-then-remap simulation, its own `delete_selection` hits this
// same error; `move_insertion_fits_after_delete` swallows the delete error and
// returns `false`, so judge correctly REFUSES (no over-prediction) while the
// execution still errors. The gate skips the erroring execution via the pinned
// `IllegalInsertSlot` allowlist. Pinned so this latent execution defect stays
// visible until the repair fallback accounts for list-type (as opposed to
// textblock-hosting) block containers.
#[test]
fn move_bullet_list_items_outside_source_execution_hits_illegal_insert_slot() {
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
    assert!(
        !judged,
        "the move simulation's own delete_selection hits the same error, so judge refuses"
    );

    let (run_result, _run_changed) = {
        let mut scratch = Editor::new_test(editor.state().clone());
        let before = crate::test_utils::EditorSnapshot::capture(&scratch);
        let result = apply_drop_for_test(
            &mut scratch,
            position,
            DndDropPayload::InternalSelection,
            modifiers,
            Some(source),
        );
        let after = crate::test_utils::EditorSnapshot::capture(&scratch);
        (result, before != after)
    };
    assert!(
        matches!(
            run_result,
            Err(EditorError::Command(editor_commands::CommandError::Step(
                editor_transaction::StepError::IllegalInsertSlot { .. }
            )))
        ),
        "move's delete half empties the bullet_list, and the cursor-repair \
         fallback's bare-Paragraph filler is schema-illegal for a list container: {run_result:?}"
    );
}

#[test]
fn drop_judgment_matches_execution_at_block_boundaries() {
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
            assert_drop_parity(&mut editor, position, &payload, modifiers, None);
        }
    }
    KNOWN_INSERT_ERROR_BUCKET.with(|b| println!("known_insert_error_bucket: {}", b.count()));
}

// Reconstructs the same judged=true/execution=Err(NodeNotFound) input as
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
