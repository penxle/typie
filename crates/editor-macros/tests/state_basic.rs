use editor_macros::state;
use editor_model::*;
use editor_state::*;

#[test]
fn state_collapsed_selection() {
    let (state, _, t) = state! {
        doc {
            root {
                p: paragraph {
                    t: text("Hello World")
                }
            }
        }
        selection: (t, 0)
    };

    // Verify selection is collapsed at (t, 0)
    assert!(state.selection.is_collapsed());
    assert_eq!(state.selection.anchor.node_id, t);
    assert_eq!(state.selection.anchor.offset, 0);
    assert_eq!(state.selection.anchor.affinity, Affinity::Downstream);

    // Verify doc content
    let t_entry = state.doc.get_entry(t).unwrap();
    if let Node::Text(ref text_node) = t_entry.node {
        assert_eq!(text_node.text, "Hello World");
    } else {
        panic!("expected Text node");
    }
}

#[test]
fn state_range_selection() {
    let (state, t1, t2) = state! {
        doc {
            root {
                paragraph {
                    t1: text("Hello")
                }
                paragraph {
                    t2: text("World")
                }
            }
        }
        selection: (t1, 2) -> (t2, 3)
    };
    assert!(!state.selection.is_collapsed());
    assert_eq!(state.selection.anchor.node_id, t1);
    assert_eq!(state.selection.anchor.offset, 2);
    assert_eq!(state.selection.head.node_id, t2);
    assert_eq!(state.selection.head.offset, 3);
}

#[test]
fn state_affinity_upstream() {
    let (state, t) = state! {
        doc {
            root {
                paragraph {
                    t: text("Hello")
                }
            }
        }
        selection: (t, 5, <)
    };
    assert!(state.selection.is_collapsed());
    assert_eq!(state.selection.anchor.node_id, t);
    assert_eq!(state.selection.anchor.offset, 5);
    assert_eq!(state.selection.anchor.affinity, Affinity::Upstream);
}

#[test]
fn state_affinity_downstream_explicit() {
    let (state, t) = state! {
        doc {
            root {
                paragraph {
                    t: text("Hello")
                }
            }
        }
        selection: (t, 5, >)
    };
    assert!(state.selection.is_collapsed());
    assert_eq!(state.selection.anchor.node_id, t);
    assert_eq!(state.selection.anchor.offset, 5);
    assert_eq!(state.selection.anchor.affinity, Affinity::Downstream);
}

#[test]
fn state_range_with_affinity() {
    let (state, t1, t2) = state! {
        doc {
            root {
                paragraph {
                    t1: text("Hello")
                }
                paragraph {
                    t2: text("World")
                }
            }
        }
        selection: (t1, 0, <) -> (t2, 5, >)
    };
    assert!(!state.selection.is_collapsed());
    assert_eq!(state.selection.anchor.node_id, t1);
    assert_eq!(state.selection.anchor.offset, 0);
    assert_eq!(state.selection.anchor.affinity, Affinity::Upstream);
    assert_eq!(state.selection.head.node_id, t2);
    assert_eq!(state.selection.head.offset, 5);
    assert_eq!(state.selection.head.affinity, Affinity::Downstream);
}
