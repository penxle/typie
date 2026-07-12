use std::sync::Arc;

use editor_common::{Direction, Movement};
use editor_crdt::{Changeset, Dot, ListOp};
use editor_macros::state;
use editor_model::{ChildView, EditOp, NodeType, SeqItem};
use editor_state::{Affinity, Position, Selection, State};
use hashbrown::HashSet;

use crate::editor::Editor;
use crate::message::*;

fn insert_root_paragraph_before_first_child(base: &State) -> Changeset<EditOp> {
    let mut projected = base.projected.as_ref().clone();
    let baseline: HashSet<Dot> = projected.graph().current_heads().copied().collect();
    projected
        .apply_batch(vec![EditOp::Seq(ListOp::Ins {
            pos: 0,
            item: SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![Dot::ROOT],
                attrs: vec![],
            },
        })])
        .unwrap();
    projected.commit();
    projected
        .graph()
        .local_changesets_since(&baseline)
        .unwrap()
        .remove(0)
}

#[test]
fn empty_and_selection_only_ticks_do_not_clone_the_projected_document() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("ab") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(initial);
    let projected = Arc::as_ptr(&editor.state.projected);

    editor.tick().unwrap();
    assert_eq!(Arc::as_ptr(&editor.state.projected), projected);

    editor.enqueue(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(Position::new(p1, 1)),
        },
    });
    editor.tick().unwrap();
    assert_eq!(Arc::as_ptr(&editor.state.projected), projected);
}

#[test]
fn set_at_same_tick_as_remote_structure_selects_visible_atom_identity() {
    let (initial, image, ..) = state! {
        doc { root { image: image p1: paragraph { text("b") } } }
        selection: none
    };
    let mut editor = Editor::new_test(initial.clone());
    let external = editor
        .view
        .external_elements(&editor.state, None)
        .into_iter()
        .find(|element| element.node == image)
        .expect("image must be laid out");

    editor.enqueue(Message::Remote {
        changeset: insert_root_paragraph_before_first_child(&initial),
    });
    editor.enqueue(Message::Selection {
        op: SelectionOp::SetAt {
            page: external.page_idx,
            x: external.bounds.x + external.bounds.width / 2.0,
            y: external.bounds.y + external.bounds.height / 2.0,
        },
    });
    editor.tick().unwrap();

    let selection = editor.state.selection.expect("tap must select the image");
    assert_eq!(selection.anchor.node, Dot::ROOT);
    assert_eq!((selection.anchor.offset, selection.head.offset), (1, 2));
    assert!(matches!(
        editor.state.view().root().unwrap().child_at(1),
        Some(ChildView::Leaf(leaf)) if leaf.dot() == image
    ));
}

#[test]
fn select_unit_at_same_tick_as_remote_structure_selects_visible_atom_identity() {
    let (initial, image, ..) = state! {
        doc { root { image: image p1: paragraph { text("b") } } }
        selection: none
    };
    let mut editor = Editor::new_test(initial.clone());
    let external = editor
        .view
        .external_elements(&editor.state, None)
        .into_iter()
        .find(|element| element.node == image)
        .expect("image must be laid out");

    editor.enqueue(Message::Remote {
        changeset: insert_root_paragraph_before_first_child(&initial),
    });
    editor.enqueue(Message::Selection {
        op: SelectionOp::SelectUnitAt {
            page: external.page_idx,
            x: external.bounds.x + external.bounds.width / 2.0,
            y: external.bounds.y + external.bounds.height / 2.0,
            unit: SelectionPointUnit::Word,
        },
    });
    editor.tick().unwrap();

    let selection = editor.state.selection.expect("tap must select the image");
    assert_eq!((selection.anchor.offset, selection.head.offset), (1, 2));
}

#[test]
fn extend_to_same_tick_as_remote_structure_tracks_visible_atom_identity() {
    let (initial, image, p1, ..) = state! {
        doc { root { image: image p1: paragraph { text("b") } } }
        selection: (p1, 0)
    };
    let layout_editor = Editor::new_test(initial.clone());
    let external = layout_editor
        .view
        .external_elements(&layout_editor.state, None)
        .into_iter()
        .find(|element| element.node == image)
        .expect("image must be laid out");

    let mut editor = Editor::new_test(initial.clone());
    editor.enqueue(Message::Remote {
        changeset: insert_root_paragraph_before_first_child(&initial),
    });
    editor.enqueue(Message::Selection {
        op: SelectionOp::ExtendTo {
            anchor: Position::new(p1, 0),
            head_page: external.page_idx,
            head_x: external.bounds.x + external.bounds.width / 2.0,
            head_y: external.bounds.y + external.bounds.height / 2.0,
            base_selection: None,
            allow_collapse: true,
        },
    });
    editor.tick().unwrap();

    assert_eq!(
        editor.state.selection,
        Some(Selection::new(
            Position {
                node: p1,
                offset: 0,
                affinity: Affinity::Upstream,
            },
            Position {
                node: Dot::ROOT,
                offset: 1,
                affinity: Affinity::Downstream,
            },
        ))
    );
}

#[test]
fn dnd_drop_same_tick_as_remote_structure_uses_visible_atom_boundary() {
    let (initial, image, ..) = state! {
        doc { root { image: image p1: paragraph { text("b") } } }
        selection: none
    };
    let mut editor = Editor::new_test(initial.clone());
    let external = editor
        .view
        .external_elements(&editor.state, None)
        .into_iter()
        .find(|element| element.node == image)
        .expect("image must be laid out");

    editor.enqueue(Message::Dnd {
        op: DndOp::EnterExternal {
            payload: ExternalDndPayloadKind::Text,
        },
    });
    editor.enqueue(Message::Remote {
        changeset: insert_root_paragraph_before_first_child(&initial),
    });
    editor.enqueue(Message::Dnd {
        op: DndOp::Over {
            page: external.page_idx,
            x: external.bounds.x + external.bounds.width / 2.0,
            y: external.bounds.y + external.bounds.height / 2.0,
            modifiers: InputModifiers::default(),
        },
    });
    editor.enqueue(Message::Dnd {
        op: DndOp::Drop {
            page: external.page_idx,
            x: external.bounds.x + external.bounds.width / 2.0,
            y: external.bounds.y + external.bounds.height / 2.0,
            payload: DndDropPayload::Text {
                text: "X".into(),
                html: None,
            },
            modifiers: InputModifiers::default(),
        },
    });
    editor.tick().unwrap();

    let view = editor.state.view();
    let root = view.root().unwrap();
    let image_index = root
        .children()
        .position(|child| matches!(child, ChildView::Leaf(leaf) if leaf.dot() == image));
    assert_eq!(
        image_index,
        Some(1),
        "drop must stay after the visible image"
    );
    assert!(
        matches!(root.child_at(2), Some(ChildView::Block(block)) if block.inline_text() == "X")
    );
}

#[test]
fn navigation_same_tick_as_remote_structure_moves_from_visible_selection() {
    let (initial, ..) = state! {
        doc { root: root { image p1: paragraph { text("b") } } }
        selection: (root, 0, >) -> (root, 1, <)
    };
    let change = insert_root_paragraph_before_first_child(&initial);
    let movement = Movement::Grapheme {
        direction: Direction::Forward,
    };

    let mut coalesced = Editor::new_test(initial.clone());
    coalesced.enqueue(Message::Remote {
        changeset: change.clone(),
    });
    coalesced.enqueue(Message::Navigation {
        op: NavigationOp::Move {
            movement,
            extend: false,
        },
    });
    coalesced.tick().unwrap();

    let mut sequential = Editor::new_test(initial);
    sequential.apply(Message::Remote { changeset: change });
    sequential.apply(Message::Navigation {
        op: NavigationOp::Move {
            movement,
            extend: false,
        },
    });

    assert_eq!(coalesced.state.selection, sequential.state.selection);
}

#[test]
fn deletion_move_same_tick_as_remote_structure_uses_visible_selection() {
    let (initial, ..) = state! {
        doc { root: root { image p1: paragraph { text("b") } } }
        selection: (root, 0, >) -> (root, 1, <)
    };
    let change = insert_root_paragraph_before_first_child(&initial);
    let movement = Movement::Word {
        direction: Direction::Forward,
    };

    let mut coalesced = Editor::new_test(initial.clone());
    coalesced.enqueue(Message::Remote {
        changeset: change.clone(),
    });
    coalesced.enqueue(Message::Deletion {
        op: DeletionOp::Move { movement },
    });
    coalesced.tick().unwrap();

    let mut sequential = Editor::new_test(initial);
    sequential.apply(Message::Remote { changeset: change });
    sequential.apply(Message::Deletion {
        op: DeletionOp::Move { movement },
    });

    editor_state::assert_state_eq!(coalesced.state(), sequential.state());
}
