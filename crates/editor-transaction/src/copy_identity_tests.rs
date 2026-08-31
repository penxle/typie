use proptest::prelude::*;

use editor_crdt::Dot;
use editor_macros::state;
use editor_state::State;

use crate::{Transaction, build_revert_transaction};

#[derive(Clone, Debug)]
enum Act {
    Move { para: u8, to: u8 },
    Type { para: u8, offset: u8, ch: char },
    Delete { para: u8 },
    Revert,
}

fn arb_act() -> impl Strategy<Value = Act> {
    prop_oneof![
        (0u8..10, 0u8..=10).prop_map(|(para, to)| Act::Move { para, to }),
        (0u8..10, 0u8..4, prop::sample::select(vec!['q', 'r']))
            .prop_map(|(para, offset, ch)| Act::Type { para, offset, ch }),
        (0u8..10).prop_map(|para| Act::Delete { para }),
        Just(Act::Revert),
    ]
}

fn base() -> State {
    let (state, _p1) = state! {
        doc { root {
            p1: paragraph { text("aaa") }
            paragraph { text("bbb") }
            paragraph { text("ccc") }
            paragraph { text("ddd") }
            paragraph { text("eee") }
            paragraph { text("fff") }
            paragraph { text("ggg") }
            paragraph { text("hhh") }
            paragraph { text("iii") }
            paragraph { text("jjj") }
            paragraph { text("kkk") }
            paragraph { text("lll") }
            paragraph { text("mmm") }
            paragraph { text("nnn") }
            paragraph { text("ooo") }
            paragraph { text("ppp") }
            paragraph { text("qqq") }
            paragraph { text("rrr") }
            paragraph { text("sss") }
            paragraph { text("ttt") }
        } }
        selection: (p1, 0)
    };
    state
}

fn para_at(state: &State, i: u8) -> Option<Dot> {
    state
        .view()
        .root()?
        .child_blocks()
        .nth(i as usize)
        .map(|b| b.id())
}

fn apply_act(state: &State, origin: &State, act: &Act) -> State {
    match act {
        Act::Move { para, to } => {
            let Some(p) = para_at(state, *para) else {
                return state.clone();
            };
            let n = state.view().root().unwrap().child_blocks().count();
            let mut tr = Transaction::new(state);
            if tr
                .move_node(p, Dot::ROOT, (*to as usize).min(n.saturating_sub(1)))
                .is_err()
            {
                return state.clone();
            }
            tr.commit().0
        }
        Act::Type { para, offset, ch } => {
            let Some(p) = para_at(state, *para) else {
                return state.clone();
            };
            let len = state.view().node(p).map_or(0, |n| n.children().count());
            let mut tr = Transaction::new(state);
            if tr
                .insert_text(p, (*offset as usize).min(len), &ch.to_string())
                .is_err()
            {
                return state.clone();
            }
            tr.commit().0
        }
        Act::Delete { para } => {
            let Some(p) = para_at(state, *para) else {
                return state.clone();
            };
            let mut tr = Transaction::new(state);
            if tr.remove_subtree(p).is_err() {
                return state.clone();
            }
            tr.commit().0
        }
        Act::Revert => match build_revert_transaction(state, origin) {
            Ok(tr) => tr.commit().0,
            Err(_) => state.clone(),
        },
    }
}

fn sync(dst: &State, src: &State) -> State {
    let heads: hashbrown::HashSet<Dot> = dst.graph().current_heads().copied().collect();
    let css = src.missing_changesets_tolerant(&heads);
    dst.receive_remote_changesets(css).unwrap().0
}

fn fork(state: &State) -> State {
    State::from_changesets(state.graph().changesets_as_vec(), None).unwrap()
}

fn check_oracles(state: &State) -> Result<(), TestCaseError> {
    let doc = state.projected.projected();
    let view = state.view();
    for members in doc.alias_classes.classes() {
        let shown = members
            .iter()
            .filter(|d| view.node(**d).is_some() || view.leaf(**d).is_some())
            .count();
        prop_assert!(shown <= 1, "class {:?} shows {} copies", members, shown);
    }
    for d in doc.hidden.roots() {
        prop_assert!(
            view.node(d).is_none() && view.leaf(d).is_none(),
            "hidden root {:?} is in the tree",
            d
        );
    }
    prop_assert_eq!(state.projected.repair_stats().totality_violations, 0);
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    #[test]
    fn replicas_converge_and_show_each_class_at_most_once(
        a_acts in prop::collection::vec(arb_act(), 0..4),
        b_acts in prop::collection::vec(arb_act(), 0..4),
        b_first in any::<bool>(),
    ) {
        let origin = base();
        let mut a = origin.clone();
        let mut b = fork(&origin);
        for act in &a_acts {
            a = apply_act(&a, &origin, act);
        }
        for act in &b_acts {
            b = apply_act(&b, &origin, act);
        }
        let (ma, mb) = if b_first {
            (sync(&a, &b), sync(&b, &a))
        } else {
            let mb = sync(&b, &a);
            (sync(&a, &mb), mb)
        };
        prop_assert_eq!(ma.projected.projected(), mb.projected.projected());
        check_oracles(&ma)?;
        check_oracles(&mb)?;
        let cold = State::from_changesets(ma.graph().changesets_as_vec(), None).unwrap();
        prop_assert_eq!(cold.projected.projected(), ma.projected.projected());
    }
}

fn paragraph_texts(state: &State) -> Vec<String> {
    state
        .view()
        .root()
        .unwrap()
        .child_blocks()
        .map(|b| b.inline_text())
        .collect()
}

#[test]
fn incremental_remote_insert_onto_a_reissued_copy_keeps_replicas_converged() {
    let origin = base();
    let mut a = origin.clone();
    for act in [
        Act::Type {
            para: 0,
            offset: 2,
            ch: 'q',
        },
        Act::Type {
            para: 0,
            offset: 0,
            ch: 'q',
        },
    ] {
        a = apply_act(&a, &origin, &act);
    }
    let b = apply_act(&fork(&origin), &origin, &Act::Move { para: 0, to: 2 });
    let mb = sync(&b, &a);
    let ma = sync(&a, &mb);
    assert_eq!(paragraph_texts(&mb), paragraph_texts(&ma));
    assert_eq!(mb.projected.projected(), ma.projected.projected());
}
