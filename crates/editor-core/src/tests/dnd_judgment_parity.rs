use editor_state::{Position, Selection, State, flat_size};

use crate::editor::Editor;
use crate::handle::{apply_drop_for_test, judge_apply_drop, position_inside_selection};
use crate::message::*;

use editor_macros::state;

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
                request_id: "parity-image".into(),
                kinds: vec![AttachmentPlaceholderKind::Image],
                reuse_node_id: None,
            },
            plain,
        ),
        (
            DndDropPayload::Files {
                request_id: "parity-mixed".into(),
                kinds: vec![
                    AttachmentPlaceholderKind::Image,
                    AttachmentPlaceholderKind::File,
                ],
                reuse_node_id: None,
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
        Err(err) => {
            assert!(
                !run_changed,
                "an erroring drop execution must not mutate state (transact atomicity): {err}"
            );
            panic!(
                "drop execution returned an Err (latent execution bug): \
                 position={position:?} payload={payload:?} error={err}"
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

// Copying the whole bullet list into a block position inside its own first
// list item. The sequential block-boundary insert used to anchor its follow-up
// insert on a normalization-synthesized scaffold with no CRDT identity
// (StepError::NodeNotFound); `insert_blocks_at_block_boundary` now re-derives
// each slot from the sibling it just inserted, so the judgment and the
// execution agree on a successful, observable drop.
#[test]
fn copy_whole_list_into_its_own_item_succeeds_observably() {
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
        run_result.is_ok(),
        "copying the list into its own item must execute cleanly: {run_result:?}"
    );
    assert!(
        run_changed,
        "a judged-true copy must observably change state"
    );
}

// The interior filter (`position_inside_selection`) uses a strict interval, so a
// copy whose drop position sits ON the source's `from` boundary is not excluded —
// and copy, unlike move, has no boundary no-op. This used to reach the same
// synthetic-scaffold anchor bug as the interior case; with the anchor-relative
// re-derivation it now lands cleanly, keeping judgment and execution in
// agreement from a source that only touches (not contains) the position.
#[test]
fn copy_drop_at_list_item_source_boundary_succeeds_observably() {
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
        run_result.is_ok(),
        "the boundary-touching copy must execute cleanly: {run_result:?}"
    );
    assert!(
        run_changed,
        "a judged-true copy must observably change state"
    );
}

// Copying a fold-spanning selection into the gap past the fold's fixed slots
// (title, content). A fixed-arity container can never absorb an extra child —
// the fragments would stay projection-suppressed misfits — so
// `resolve_slice_insertion` refuses (NoFit) and the execution, consuming the
// same resolve, is a clean no-op: judgment and execution agree at `false`.
// (This used to over-predict and then die on IndexOutOfBounds once mid-batch
// normalization suppressed the first inserted child.)
#[test]
fn copy_into_fold_past_fixed_slots_is_judged_no_fit() {
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
    assert!(
        !judged,
        "a fixed-arity container admits no extra child, so the judge refuses"
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
        run_result.is_ok(),
        "the refused drop must degrade to a clean no-op, not an error: {run_result:?}"
    );
    assert!(
        !run_changed,
        "a judged-false drop must not observably change state"
    );
}

// Pins the input the re-expanded gate (all execution errors fatal) surfaced
// after the erroring-execution allowlist was removed (fixture 3, move,
// pos_off=21/src=3..20): moving a document-spanning selection to the root's
// trailing gap leaves the root with only a normalization-synthesized trailing
// paragraph scaffold after the delete half, so the re-anchored block-boundary
// insert's FIRST slot anchored on a synthetic sibling with no CRDT identity
// (StepError::NodeNotFound). `child_seq_insert_pos` now retreats to the
// nearest real left sibling (container start when none precedes), so the move
// lands cleanly and judgment matches execution.
#[test]
fn move_document_spanning_selection_to_root_end_gap_succeeds() {
    let state = fixtures().swap_remove(3);
    let size = flat_size(&state.view());
    let mut editor = Editor::new_test(state);
    let source =
        source_range(&mut editor, 3.min(size), 20.min(size)).expect("document-spanning source");
    let position =
        position_at_flat(&mut editor, 21.min(size)).expect("position at the root's trailing gap");
    editor.apply(Message::Selection {
        op: SelectionOp::SetFlat {
            start: 3.min(size),
            end: 20.min(size),
        },
    });
    assert_drop_parity(
        &mut editor,
        position,
        &DndDropPayload::InternalSelection,
        InputModifiers::default(),
        Some(source),
    );
}

// Reconstructs discrepancy #4 from the 4096-case exploration: a move (alt=false)
// of the bullet list's full two-item child range to a root-level position past
// the list's end — outside the source (18 < 19), so the strict interior filter
// does not cover it. The delete half of move's delete-then-insert empties the
// bullet_list; the cursor-repair path in
// `ensure_selection_after_child_range_delete`
// (crates/editor-commands/src/helpers/deletion.rs) now prunes the emptied
// list container instead of inserting a schema-illegal bare Paragraph filler
// (formerly pinned as StepError::IllegalInsertSlot), so the judgment and the
// execution agree on a successful, observable drop.
#[test]
fn move_bullet_list_items_outside_source_succeeds_observably() {
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
        judged,
        "the move simulation's delete half now succeeds (emptied list pruned), \
         and the re-anchored insertion resolves"
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
        run_result.is_ok(),
        "moving the whole child range out of a bullet list must execute cleanly: {run_result:?}"
    );
    assert!(
        run_changed,
        "a judged-true move must observably change state"
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
}
