//! Differential A: the restructured lift helpers (`lift_list_items` /
//! `lift_list_item_inner`) must behave identically whether or not their
//! caller wraps them in `Transaction::batched_projection` — defer only
//! changes *when* the projection runs, never *what* it produces. Both arms
//! run the exact same helper call against clones of the exact same initial
//! `State` (so `Dot` allocation is byte-identical going in), then compare
//! every observable: emitted op sequence (`RecordedOp`, `prior` included),
//! `State::selection`, `ProjectedDoc`, flat-index consistency, and an
//! op-level undo→redo round trip back to `PlainDoc` equality.

use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{
    Modifier, ModifierType, PlainBulletListNode, PlainDoc, PlainListItemNode, PlainNode,
    PlainNodeEntry, PlainParagraphNode, PlainRootNode, PlainTextNode,
};
use editor_state::test_utils::build_state_from_plain;
use editor_state::undo::{RecordedOp, capture_prior, invert};
use editor_state::{Position, Selection, State};
use editor_transaction::Transaction;

fn text(t: &str) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Text(PlainTextNode {
            text: t.to_string(),
        }),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![],
    }
}

fn bold_text(t: &str) -> PlainNodeEntry {
    let mut e = text(t);
    e.modifiers.insert(ModifierType::Bold, Modifier::Bold);
    e
}

fn para(children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Paragraph(PlainParagraphNode {}),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children,
    }
}

fn list_item(children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::ListItem(PlainListItemNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children,
    }
}

fn bullet_list(items: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::BulletList(PlainBulletListNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: items,
    }
}

fn root(children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Root(PlainRootNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children,
    }
}

const ITEM_TEXT_LEN: usize = 4; // "item".chars().count()

fn item_entry(styled: bool, has_sub: bool) -> PlainNodeEntry {
    let content = if styled {
        bold_text("item")
    } else {
        text("item")
    };
    let mut children = vec![para(vec![content])];
    if has_sub {
        children.push(bullet_list(vec![list_item(vec![para(vec![text("sub")])])]));
    }
    list_item(children)
}

/// Builds a document with `n` selected list items (each optionally styled,
/// each optionally carrying its own pre-existing nested sublist) followed by
/// `trailing` unselected sibling items in the same list — either directly
/// under `root` (`top_level`) or nested one level under an outer list item
/// (exercising the `NestedUnderListItem` plan kind and its `existing_sublist`
/// branch). Returns the initial `State` (selection spanning all `n` selected
/// items' own paragraphs) and the selected items' `Dot`s in document order.
fn build_fixture(
    n: usize,
    top_level: bool,
    trailing: usize,
    styled: &[bool],
    has_sub: &[bool],
) -> (State, Vec<Dot>) {
    let total = n + trailing;
    let list_items: Vec<PlainNodeEntry> = (0..total)
        .map(|i| {
            if i < n {
                item_entry(styled[i], has_sub[i])
            } else {
                item_entry(false, false)
            }
        })
        .collect();
    let list_entry = bullet_list(list_items);

    let (root_entry, item_paths, para_paths): (PlainNodeEntry, Vec<Vec<usize>>, Vec<Vec<usize>>) =
        if top_level {
            let item_paths = (0..n).map(|i| vec![0usize, i]).collect();
            let para_paths = (0..n).map(|i| vec![0usize, i, 0usize]).collect();
            (root(vec![list_entry]), item_paths, para_paths)
        } else {
            let outer_item = list_item(vec![para(vec![text("outer")]), list_entry]);
            let outer_list = bullet_list(vec![outer_item]);
            let item_paths = (0..n).map(|i| vec![0usize, 0usize, 1usize, i]).collect();
            let para_paths = (0..n)
                .map(|i| vec![0usize, 0usize, 1usize, i, 0usize])
                .collect();
            (root(vec![outer_list]), item_paths, para_paths)
        };

    let (mut state, handles) = build_state_from_plain(PlainDoc { root: root_entry });
    let items: Vec<Dot> = item_paths.iter().map(|p| handles[p]).collect();
    let first_para = handles[&para_paths[0]];
    let last_para = handles[&para_paths[n - 1]];
    state.selection = Some(Selection::new(
        Position::new(first_para, 0),
        Position::new(last_para, ITEM_TEXT_LEN),
    ));
    (state, items)
}

/// Applies the op-level inverse (`editor_state::undo::{capture_prior,
/// invert}` — the same primitives `UndoHistory` uses) of `ops` to `state`, in
/// reverse order. Returns the resulting ops as fresh `RecordedOp`s, so
/// calling this a second time on its own output redoes what the first call
/// undid. A `Step`-level `inverse()`/`apply()` round trip cannot survive a
/// lift's re-minted dots (a moved subtree's old dots are tombstoned), so the
/// oracle here is the dot-based primitive undo/redo actually builds on.
fn invert_recorded_ops(state: &mut State, ops: &[RecordedOp]) -> Vec<RecordedOp> {
    let mut out = Vec::new();
    for ro in ops.iter().rev() {
        for payload in invert(&state.projected, ro) {
            let prior = capture_prior(&state.projected, &payload);
            let op = state.projected_mut().apply(payload).unwrap();
            out.push(RecordedOp { op, prior });
        }
    }
    out
}

fn assert_defer_parity(
    initial: &State,
    ok_off: bool,
    ok_on: bool,
    state_off: &State,
    state_on: &State,
    recorded_off: &[RecordedOp],
    recorded_on: &[RecordedOp],
) {
    assert!(ok_off, "off arm must apply");
    assert_eq!(
        ok_off, ok_on,
        "batched_projection must not change whether the command applies"
    );
    assert_eq!(
        recorded_off, recorded_on,
        "emitted op sequence (incl. prior) must be defer-invariant"
    );
    assert_eq!(
        state_off.selection, state_on.selection,
        "selection must be defer-invariant"
    );
    assert_eq!(
        state_off.projected.projected(),
        state_on.projected.projected(),
        "ProjectedDoc must be defer-invariant"
    );
    editor_model::assert_flat_index_consistent(&state_off.projected.projected().tree);
    editor_model::assert_flat_index_consistent(&state_on.projected.projected().tree);

    for (state, recorded) in [(state_off, recorded_off), (state_on, recorded_on)] {
        let mut undone = state.clone();
        let undo_ops = invert_recorded_ops(&mut undone, recorded);
        assert_eq!(
            undone.to_plain(),
            initial.to_plain(),
            "undo must restore the initial PlainDoc"
        );
        invert_recorded_ops(&mut undone, &undo_ops);
        assert_eq!(
            undone.to_plain(),
            state.to_plain(),
            "redo must restore the post-lift PlainDoc"
        );
    }
}

fn cases() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(64)
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig { cases: cases(), ..proptest::prelude::ProptestConfig::default() })]
    #[test]
    fn lift_defer_parity_multi_item(
        n in 2usize..=6,
        top_level in proptest::bool::ANY,
        trailing in 0usize..=2,
        styled_bits in proptest::collection::vec(proptest::bool::ANY, 6),
        sub_bits in proptest::collection::vec(proptest::bool::ANY, 6),
    ) {
        let styled = &styled_bits[..n];
        let has_sub = &sub_bits[..n];
        let (initial, items) = build_fixture(n, top_level, trailing, styled, has_sub);

        let mut tr_off = Transaction::new(&initial);
        let ok_off = crate::helpers::lift_list_items_planned(&mut tr_off, items.clone()).expect("off arm applies");
        let (state_off, _steps_off, recorded_off, ..) = tr_off.commit();

        let mut tr_on = Transaction::new(&initial);
        let ok_on = tr_on
            .batched_projection(|tr| crate::helpers::lift_list_items_planned(tr, items.clone()))
            .expect("on arm applies");
        let (state_on, _steps_on, recorded_on, ..) = tr_on.commit();

        assert_defer_parity(&initial, ok_off, ok_on, &state_off, &state_on, &recorded_off, &recorded_on);
    }
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig { cases: cases(), ..proptest::prelude::ProptestConfig::default() })]
    #[test]
    fn lift_defer_parity_single_item(
        top_level in proptest::bool::ANY,
        trailing in 0usize..=2,
        styled in proptest::bool::ANY,
        has_sub in proptest::bool::ANY,
    ) {
        let (initial, items) = build_fixture(1, top_level, trailing, &[styled], &[has_sub]);
        let item = items[0];

        let mut tr_off = Transaction::new(&initial);
        let ok_off = crate::helpers::lift_list_item_inner(&mut tr_off, item).expect("off arm applies");
        let (state_off, _steps_off, recorded_off, ..) = tr_off.commit();

        let mut tr_on = Transaction::new(&initial);
        let ok_on = tr_on
            .batched_projection(|tr| crate::helpers::lift_list_item_inner(tr, item))
            .expect("on arm applies");
        let (state_on, _steps_on, recorded_on, ..) = tr_on.commit();

        assert_defer_parity(&initial, ok_off, ok_on, &state_off, &state_on, &recorded_off, &recorded_on);
    }
}
