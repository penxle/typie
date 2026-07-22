//! Temporary perf baselines for span-boundary typing and list-toolbar probes.
//! Run: cargo test -p editor-core --release perf_span_boundary -- --ignored --nocapture

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use editor_model::{
    Anchor, Bias, ChildView, EditOp, Modifier, ModifierType, NodeType, PlainBulletListNode,
    PlainDoc, PlainFoldContentNode, PlainFoldNode, PlainFoldTitleNode, PlainListItemNode,
    PlainNode, PlainNodeEntry, PlainParagraphNode, PlainRootNode, PlainTextNode, SeqItem, SpanOp,
};
use editor_resource::{FontFamily, FontFamilySource, FontManifest, FontWeight, Resource};
use editor_state::test_utils::build_state_from_plain;
use editor_state::{Position, Selection, State};

use crate::editor::Editor;
use crate::message::*;

fn make_resource() -> Resource {
    let mut resource = Resource::new_test();
    let families = [
        ("Pretendard", vec![400u16, 700]),
        ("Paperlogy", vec![400u16, 700]),
    ];
    resource.set_fonts(
        families
            .iter()
            .map(|(name, weights)| FontFamily {
                name: name.to_string(),
                source: FontFamilySource::Default,
                weights: weights
                    .iter()
                    .map(|&value| FontWeight {
                        value,
                        hash: format!("{name}-{value}"),
                    })
                    .collect(),
            })
            .collect(),
    );
    for (name, weights) in &families {
        let id = resource.font_registry.intern_id(name).unwrap();
        for &value in weights {
            resource.font_registry.set_manifest(
                id,
                value,
                FontManifest::from_coverages(&[vec![0x0000, 0xFFFF]]),
            );
        }
    }
    resource
}

fn plain_text(text: String) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Text(PlainTextNode { text }),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![],
    }
}

fn plain_para(text: String) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Paragraph(PlainParagraphNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![plain_text(text)],
    }
}

fn root_entry(children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    let mut root_modifiers = BTreeMap::new();
    root_modifiers.insert(
        ModifierType::FontFamily,
        Modifier::FontFamily {
            value: "Pretendard".to_string(),
        },
    );
    root_modifiers.insert(
        ModifierType::FontWeight,
        Modifier::FontWeight { value: 400 },
    );
    PlainNodeEntry {
        node: PlainNode::Root(PlainRootNode::default()),
        modifiers: root_modifiers,
        carry: Vec::new(),
        children,
    }
}

fn paras(count: usize, chars_per_para: usize) -> Vec<PlainNodeEntry> {
    let text: String = "가나다라마 hello world 바사아자차 "
        .chars()
        .cycle()
        .take(chars_per_para)
        .collect();
    (0..count).map(|_| plain_para(text.clone())).collect()
}

fn bullet_list(count: usize, chars_per_para: usize) -> Vec<PlainNodeEntry> {
    let items = paras(count, chars_per_para)
        .into_iter()
        .map(|p| PlainNodeEntry {
            node: PlainNode::ListItem(PlainListItemNode::default()),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children: vec![p],
        })
        .collect();
    vec![PlainNodeEntry {
        node: PlainNode::BulletList(PlainBulletListNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: items,
    }]
}

fn content_blocks(in_list: bool, count: usize, chars_per_para: usize) -> Vec<PlainNodeEntry> {
    if in_list {
        bullet_list(count, chars_per_para)
    } else {
        paras(count, chars_per_para)
    }
}

fn build_fold_state(in_list: bool, count: usize, chars_per_para: usize) -> State {
    let fold = PlainNodeEntry {
        node: PlainNode::Fold(PlainFoldNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![
            PlainNodeEntry {
                node: PlainNode::FoldTitle(PlainFoldTitleNode::default()),
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children: vec![plain_text("Title".to_string())],
            },
            PlainNodeEntry {
                node: PlainNode::FoldContent(PlainFoldContentNode::default()),
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children: content_blocks(in_list, count, chars_per_para),
            },
        ],
    };
    let (state, _handles) = build_state_from_plain(PlainDoc {
        root: root_entry(vec![fold, plain_para("tail".to_string())]),
    });
    state
}

fn item_paragraph_leaf_dots(state: &State) -> Vec<Vec<editor_crdt::Dot>> {
    let view = state.view();
    let root = view.root().unwrap();
    let fold = root
        .child_blocks()
        .find(|b| b.node_type() == NodeType::Fold)
        .expect("fold");
    let content = fold
        .child_blocks()
        .find(|b| b.node_type() == NodeType::FoldContent)
        .expect("fold content");
    let list = content
        .child_blocks()
        .find(|b| b.node_type() == NodeType::BulletList)
        .expect("bullet list");
    list.child_blocks()
        .filter(|b| b.node_type() == NodeType::ListItem)
        .map(|item| {
            let para = item
                .child_blocks()
                .find(|b| b.node_type() == NodeType::Paragraph)
                .expect("item paragraph");
            para.children()
                .filter_map(|c| match c {
                    ChildView::Leaf(l) => Some(l.dot()),
                    ChildView::Block(_) => None,
                })
                .collect()
        })
        .collect()
}

// Stress spans draw endpoints ONLY from items 4+, so the measured items
// (0: bold target, 1: plain control, 2: probe caret) keep clean boundaries
// and an uncontested LWW winner; the measured bold spans are applied LAST so
// a newer stress op can never outrank them (winner = max op dot).
fn add_spans(state: State, extra: usize) -> State {
    let per_item = item_paragraph_leaf_dots(&state);
    let stress_leaves: Vec<editor_crdt::Dot> = per_item.iter().skip(4).flatten().copied().collect();
    let (next, _ops) = state
        .batch_with_ops(|b| {
            for k in 0..extra {
                let a = stress_leaves[(k * 7) % stress_leaves.len()];
                let bdot = stress_leaves[(k * 11 + 3) % stress_leaves.len()];
                b.apply(EditOp::Span(SpanOp::AddSpan {
                    start: Anchor {
                        id: a,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: bdot,
                        bias: Bias::After,
                    },
                    modifier: if k % 2 == 0 {
                        Modifier::Italic
                    } else {
                        Modifier::Bold
                    },
                }))?;
            }
            for (i, leaves) in per_item.iter().enumerate() {
                if i % 2 == 0 && leaves.len() >= 2 {
                    b.apply(EditOp::Span(SpanOp::AddSpan {
                        start: Anchor {
                            id: leaves[0],
                            bias: Bias::Before,
                        },
                        end: Anchor {
                            id: *leaves.last().unwrap(),
                            bias: Bias::After,
                        },
                        modifier: Modifier::Bold,
                    }))?;
                }
            }
            Ok::<(), editor_state::StateError>(())
        })
        .expect("span batch applies");
    next
}

// Each sample builds a FRESH editor from a clone of the shared base State
// (setup outside the timer) and measures exactly ONE insert at the pristine
// boundary — a sequential typing loop would only hit the span boundary on the
// first key (the new plain char becomes the neighbor afterwards), and
// compensating CRDT edits would still grow history between samples.
fn median_boundary_type(
    base: &State,
    resource: &Arc<Mutex<Resource>>,
    fold: editor_crdt::Dot,
    target: editor_crdt::Dot,
    offset: usize,
) -> Duration {
    let mut v = Vec::new();
    for _ in 0..20 {
        let mut editor = Editor::new_test_with_resource(base.clone(), resource.clone());
        editor.apply(Message::View {
            op: ViewOp::ToggleFold { id: fold },
        });
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::collapsed(Position::new(target, offset)),
            },
        });
        let t = Instant::now();
        editor.apply(Message::Insertion {
            op: InsertionOp::Text {
                text: "a".to_string(),
            },
        });
        v.push(t.elapsed());
    }
    v.sort();
    v[v.len() / 2]
}

/// Seals the freshly-applied span batch behind one seq op. Without it every
/// measured keystroke's `seq_parents` walk traverses the ENTIRE uninterrupted
/// span-op run (O(run) per key — 40% of mid-typing samples in profiling),
/// polluting the non-boundary baseline. Production histories interleave spans
/// with edits, so the long run is a fixture-only artifact; one trailing seq op
/// terminates the walk in O(1).
fn seal_history(mut state: State) -> State {
    let pos = state.projected_mut().seq_checkout().visible_len();
    let (state, _) = state
        .batch_with_ops(|b| {
            b.apply(EditOp::Seq(editor_crdt::ListOp::Ins {
                pos,
                item: SeqItem::Char('.'),
            }))?;
            Ok::<(), editor_state::StateError>(())
        })
        .expect("seal op applies");
    state
}

fn dense_state(extra: usize) -> State {
    seal_history(add_spans(build_fold_state(true, 200, 30), extra))
}

#[test]
#[ignore]
fn perf_boundary_typing() {
    let resource = Arc::new(Mutex::new(make_resource()));
    for extra in [16000usize, 32000] {
        let state = dense_state(extra);
        let per_item = item_paragraph_leaf_dots(&state);
        let (fold, bold_para, plain_para) = {
            let v = state.view();
            (
                v.root()
                    .unwrap()
                    .child_blocks()
                    .find(|b| b.node_type() == NodeType::Fold)
                    .unwrap()
                    .id(),
                v.leaf(per_item[0][0]).unwrap().parent().unwrap().id(),
                v.leaf(per_item[1][0]).unwrap().parent().unwrap().id(),
            )
        };
        let bold_len = per_item[0].len();
        let plain_len = per_item[1].len();

        eprintln!(
            "[spans={extra}] bold end:   {:?}",
            median_boundary_type(&state, &resource, fold, bold_para, bold_len)
        );
        eprintln!(
            "[spans={extra}] bold mid:   {:?}",
            median_boundary_type(&state, &resource, fold, bold_para, bold_len / 2)
        );
        eprintln!(
            "[spans={extra}] bold start: {:?}",
            median_boundary_type(&state, &resource, fold, bold_para, 0)
        );
        eprintln!(
            "[spans={extra}] plain end:  {:?}",
            median_boundary_type(&state, &resource, fold, plain_para, plain_len)
        );
    }
}
