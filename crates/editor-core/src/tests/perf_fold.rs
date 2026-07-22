//! Perf baseline for Enter inside a fold vs at root.
//! Run: cargo test -p editor-core --release perf_fold -- --ignored --nocapture

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use editor_model::{
    Modifier, ModifierType, NodeType, PlainBulletListNode, PlainDoc, PlainFoldContentNode,
    PlainFoldNode, PlainFoldTitleNode, PlainListItemNode, PlainNode, PlainNodeEntry,
    PlainParagraphNode, PlainRootNode, PlainTextNode,
};
use editor_resource::{FontFamily, FontFamilySource, FontManifest, FontWeight, Resource};
use editor_state::test_utils::build_state_from_plain;
use editor_state::{Position, Selection, State};

use crate::editor::Editor;
use crate::message::*;

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

fn build_root_state(in_list: bool, count: usize, chars_per_para: usize) -> State {
    let (state, _handles) = build_state_from_plain(PlainDoc {
        root: root_entry(content_blocks(in_list, count, chars_per_para)),
    });
    state
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

fn fold_dot(state: &State) -> editor_crdt::Dot {
    let view = state.view();
    view.root()
        .unwrap()
        .child_blocks()
        .find(|b| b.node_type() == NodeType::Fold)
        .expect("fold")
        .id()
}

fn nth_para_dot(state: &State, in_fold: bool, idx: usize) -> editor_crdt::Dot {
    let view = state.view();
    let root = view.root().unwrap();
    let container = if in_fold {
        view.node(fold_dot(state))
            .unwrap()
            .child_blocks()
            .find(|b| b.node_type() == NodeType::FoldContent)
            .expect("fold content")
            .id()
    } else {
        root.id()
    };
    view.node(container)
        .unwrap()
        .child_blocks()
        .filter(|b| b.node_type() == NodeType::Paragraph)
        .nth(idx)
        .expect("target paragraph")
        .id()
}

fn median_enter(
    base: &State,
    resource: &Arc<Mutex<Resource>>,
    fold: Option<editor_crdt::Dot>,
    target: editor_crdt::Dot,
    offset: usize,
) -> Duration {
    let mut v = Vec::new();
    for _ in 0..20 {
        let mut editor = Editor::new_test_with_resource(base.clone(), resource.clone());
        if let Some(id) = fold {
            editor.apply(Message::View {
                op: ViewOp::ToggleFold { id },
            });
        }
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::collapsed(Position::new(target, offset)),
            },
        });
        let t = Instant::now();
        editor.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });
        v.push(t.elapsed());
    }
    v.sort();
    v[v.len() / 2]
}

#[test]
#[ignore]
fn perf_enter_fold_vs_root() {
    let resource = Arc::new(Mutex::new(make_resource()));
    for &count in &[50usize, 200] {
        let root_state = build_root_state(false, count, 200);
        let rt = nth_para_dot(&root_state, false, count / 2);
        let root_med = median_enter(&root_state, &resource, None, rt, 200);

        let fold_state = build_fold_state(false, count, 200);
        let fd = fold_dot(&fold_state);
        let ft = nth_para_dot(&fold_state, true, count / 2);
        let fold_med = median_enter(&fold_state, &resource, Some(fd), ft, 200);

        eprintln!("paras={count}: root enter {root_med:?}, fold enter {fold_med:?}");
    }
}

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
