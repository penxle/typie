use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{
    AtomLeaf, ChildView, DocView, LeafView, Marker, Modifier, ModifierType, NodeType, NodeView,
};
use editor_state::Position;
use editor_state::State;

#[derive(Debug, PartialEq)]
enum Shape {
    Block {
        ty: NodeType,
        modifiers: BTreeMap<ModifierType, Modifier>,
        style: Option<String>,
        marker: Option<Marker>,
        children: Vec<Shape>,
    },
    Char {
        ch: char,
        modifiers: BTreeMap<ModifierType, Modifier>,
        style: Option<String>,
    },
    Atom {
        leaf: AtomLeaf,
        modifiers: BTreeMap<ModifierType, Modifier>,
        style: Option<String>,
    },
}

fn leaf_own(l: &LeafView) -> BTreeMap<ModifierType, Modifier> {
    l.own_modifiers()
        .iter()
        .filter(|(_, o)| !o.from_style)
        .map(|(t, o)| (*t, o.value.clone()))
        .collect()
}

fn node_style(state: &State, id: Dot) -> Option<String> {
    state.projected.node_styles().value_of(id)
}

fn node_marker(state: &State, id: Dot) -> Option<Marker> {
    state.projected.node_markers().value_of(id)
}

fn block_modifiers(state: &State, id: Dot) -> BTreeMap<ModifierType, Modifier> {
    state.projected.block_modifiers().modifiers_of(id)
}

fn shape_of(state: &State, nv: &NodeView) -> Shape {
    let mut children = Vec::new();
    for c in nv.children() {
        match c {
            ChildView::Block(b) => children.push(shape_of(state, &b)),
            ChildView::Leaf(l) => {
                let modifiers = leaf_own(&l);
                let style = node_style(state, l.dot());
                if let Some(ch) = l.as_char() {
                    children.push(Shape::Char {
                        ch,
                        modifiers,
                        style,
                    });
                } else if let Some(atom) = l.as_atom() {
                    children.push(Shape::Atom {
                        leaf: atom.clone(),
                        modifiers,
                        style,
                    });
                }
            }
        }
    }
    Shape::Block {
        ty: nv.node_type(),
        modifiers: block_modifiers(state, nv.id()),
        style: node_style(state, nv.id()),
        marker: node_marker(state, nv.id()),
        children,
    }
}

fn doc_shape(state: &State) -> Vec<Shape> {
    let view = state.view();
    view.roots().map(|r| shape_of(state, &r)).collect()
}

fn position_path(view: &DocView, pos: &Position) -> Option<(Vec<usize>, editor_state::Affinity)> {
    let rp = pos.resolve(view)?;
    Some((rp.path().to_vec(), rp.affinity()))
}

type PathWithAffinity = (Vec<usize>, editor_state::Affinity);

fn selection_shape(state: &State) -> Option<(PathWithAffinity, PathWithAffinity)> {
    let view = state.view();
    let sel = state.selection.as_ref()?;
    let anchor = position_path(&view, &sel.anchor)?;
    let head = position_path(&view, &sel.head)?;
    Some((anchor, head))
}

pub(crate) fn assert_state_eq_impl(actual: &State, expected: &State) {
    let a = doc_shape(actual);
    let e = doc_shape(expected);
    assert_eq!(a, e, "projected document trees differ");

    let a_sel = selection_shape(actual);
    let e_sel = selection_shape(expected);
    assert_eq!(
        actual.selection.is_some(),
        expected.selection.is_some(),
        "selection presence differs"
    );
    assert_eq!(a_sel, e_sel, "selection structural paths differ");

    assert_eq!(
        actual.pending_modifiers, expected.pending_modifiers,
        "pending_modifiers differ"
    );
    assert_eq!(
        actual.pending_style, expected.pending_style,
        "pending_style differs"
    );
}

macro_rules! assert_state_eq {
    ($actual:expr, $expected:expr) => {
        $crate::test_utils::assert_state_eq_impl(&$actual, &$expected)
    };
}

macro_rules! transact {
    ($initial:expr, |$tr:ident| $body:expr) => {{
        let initial: editor_state::State = $initial;
        #[allow(unused_mut)]
        let mut $tr = editor_transaction::Transaction::new(&initial);
        assert!($body.unwrap(), "command returned Ok(false)");
        $tr.commit()
    }};
}

macro_rules! transact_err {
    ($initial:expr, |$tr:ident| $body:expr) => {{
        let initial: editor_state::State = $initial;
        #[allow(unused_mut)]
        let mut $tr = editor_transaction::Transaction::new(&initial);
        $body.unwrap_err()
    }};
}

macro_rules! transact_fail {
    ($initial:expr, |$tr:ident| $body:expr) => {{
        let initial: editor_state::State = $initial;
        #[allow(unused_mut)]
        let mut $tr = editor_transaction::Transaction::new(&initial);
        assert!(
            !$body.unwrap(),
            "command returned Ok(true), expected Ok(false)"
        );
        $tr.commit()
    }};
}

pub(crate) use assert_state_eq;
pub(crate) use transact;
pub(crate) use transact_err;
pub(crate) use transact_fail;
