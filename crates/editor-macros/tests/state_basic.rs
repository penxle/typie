use editor_macros::state;
use editor_state::*;

#[test]
fn state_collapsed_selection() {
    let (state, p1) = state! {
        doc {
            root {
                p1: paragraph {
                    text("Hello World")
                }
            }
        }
        selection: (p1, 0)
    };

    let sel = state.selection.as_ref().unwrap();
    assert!(sel.is_collapsed());
    assert_eq!(sel.anchor.node, p1);
    assert_eq!(sel.anchor.offset, 0);
    assert_eq!(sel.anchor.affinity, Affinity::Downstream);

    assert_eq!(state.view().node(p1).unwrap().inline_text(), "Hello World");
}

#[test]
fn state_range_selection() {
    let (state, p1, p2) = state! {
        doc {
            root {
                p1: paragraph {
                    text("Hello")
                }
                p2: paragraph {
                    text("World")
                }
            }
        }
        selection: (p1, 2) -> (p2, 3)
    };
    let sel = state.selection.as_ref().unwrap();
    assert!(!sel.is_collapsed());
    assert_eq!(sel.anchor.node, p1);
    assert_eq!(sel.anchor.offset, 2);
    assert_eq!(sel.head.node, p2);
    assert_eq!(sel.head.offset, 3);
}

#[test]
fn state_affinity_upstream() {
    let (state, p1) = state! {
        doc {
            root {
                p1: paragraph {
                    text("Hello")
                }
            }
        }
        selection: (p1, 5, <)
    };
    let sel = state.selection.as_ref().unwrap();
    assert!(sel.is_collapsed());
    assert_eq!(sel.anchor.node, p1);
    assert_eq!(sel.anchor.offset, 5);
    assert_eq!(sel.anchor.affinity, Affinity::Upstream);
}

#[test]
fn state_affinity_downstream_explicit() {
    let (state, p1) = state! {
        doc {
            root {
                p1: paragraph {
                    text("Hello")
                }
            }
        }
        selection: (p1, 5, >)
    };
    let sel = state.selection.as_ref().unwrap();
    assert!(sel.is_collapsed());
    assert_eq!(sel.anchor.node, p1);
    assert_eq!(sel.anchor.offset, 5);
    assert_eq!(sel.anchor.affinity, Affinity::Downstream);
}

#[test]
fn state_range_with_affinity() {
    let (state, p1, p2) = state! {
        doc {
            root {
                p1: paragraph {
                    text("Hello")
                }
                p2: paragraph {
                    text("World")
                }
            }
        }
        selection: (p1, 0, <) -> (p2, 5, >)
    };
    let sel = state.selection.as_ref().unwrap();
    assert!(!sel.is_collapsed());
    assert_eq!(sel.anchor.node, p1);
    assert_eq!(sel.anchor.offset, 0);
    assert_eq!(sel.anchor.affinity, Affinity::Upstream);
    assert_eq!(sel.head.node, p2);
    assert_eq!(sel.head.offset, 5);
    assert_eq!(sel.head.affinity, Affinity::Downstream);
}

#[test]
fn macro_selection_none_yields_none() {
    let (state, ..) = state! {
        doc { root { p1: paragraph { text("Hello") } } }
        selection: none
    };
    assert!(state.selection.is_none());
}

#[test]
fn macro_selection_collapsed_yields_some() {
    let (state, ..) = state! {
        doc { root { p1: paragraph { text("Hello") } } }
        selection: (p1, 0)
    };
    let sel = state
        .selection
        .as_ref()
        .expect("selection: (p1, 0) must yield Some");
    assert!(sel.is_collapsed());
}
