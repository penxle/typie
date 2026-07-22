use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use editor_crdt::Dot;
use editor_model::{
    ChildView, PlainBlockquoteNode, PlainBulletListNode, PlainDoc, PlainFoldContentNode,
    PlainFoldNode, PlainFoldTitleNode, PlainListItemNode, PlainNode, PlainNodeEntry,
    PlainParagraphNode, PlainRootNode, PlainTextNode,
};
use editor_state::test_utils::build_state_from_plain;
use editor_state::{
    Position, ResolvedPosition, ResolvedPositionFlatExt, Selection, StableSelection, State,
};
use editor_transaction::Transaction;

use crate::editor::Editor;
use crate::message::*;
use crate::tracked_range::{TrackedRange, TrackedRangeId, TrackedRangeRegistry};

#[cfg(feature = "resolve-stats")]
use editor_state::resolve_stats;

// ── shared fixture plumbing ─────────────────────────────────────────────────

fn filler_text(n: usize) -> String {
    "abcdefghij ".chars().cycle().take(n).collect()
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
    PlainNodeEntry {
        node: PlainNode::Root(PlainRootNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children,
    }
}

fn bullet_list_wrapping(paras: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    let items: Vec<PlainNodeEntry> = paras
        .into_iter()
        .map(|p| PlainNodeEntry {
            node: PlainNode::ListItem(PlainListItemNode::default()),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children: vec![p],
        })
        .collect();
    PlainNodeEntry {
        node: PlainNode::BulletList(PlainBulletListNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: items,
    }
}

fn editor_from_state(state: &State) -> Editor {
    Editor::new_test(state.clone())
}

fn resolve_selection(editor: &Editor) -> Option<Selection> {
    editor.state().selection
}

fn resolved_selection_caret<'a>(
    editor: &Editor,
    view: &'a editor_model::DocView<'a>,
) -> ResolvedPosition<'a> {
    let sel = resolve_selection(editor).expect("builder must install a selection");
    sel.head
        .resolve(view)
        .expect("caret must resolve against the fresh view")
}

fn nearest_insertable_probe(editor: &Editor, flat: usize) -> Option<usize> {
    let view = editor.state().view();
    let total = editor_state::flat_size(&view);
    Some(crate::editor::nearest_insertable_flat_probe(
        &view, total, flat,
    ))
}

// ── Step 2: flat decomposition fixtures ─────────────────────────────────────

fn shape_children(
    shape: &str,
    paras: usize,
    chars_per_para: usize,
) -> (Vec<PlainNodeEntry>, Vec<usize>) {
    let half = paras / 2;
    let flat = paras - half;
    let before = flat / 2;
    let after = flat - before;
    match shape {
        "flat" => {
            let children: Vec<PlainNodeEntry> = (0..paras)
                .map(|_| plain_para(filler_text(chars_per_para)))
                .collect();
            let target = vec![paras - 1];
            (children, target)
        }
        "nested-list" => {
            let mut out: Vec<PlainNodeEntry> = (0..before)
                .map(|_| plain_para(filler_text(chars_per_para)))
                .collect();
            let list_index = out.len();
            let items: Vec<PlainNodeEntry> = (0..half)
                .map(|_| plain_para(filler_text(chars_per_para)))
                .collect();
            out.push(bullet_list_wrapping(items));
            out.extend((0..after).map(|_| plain_para(filler_text(chars_per_para))));
            let target = vec![list_index, half - 1, 0];
            (out, target)
        }
        "nested-fold" => {
            let mut out: Vec<PlainNodeEntry> = (0..before)
                .map(|_| plain_para(filler_text(chars_per_para)))
                .collect();
            let fold_index = out.len();
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
                        children: (0..half)
                            .map(|_| plain_para(filler_text(chars_per_para)))
                            .collect(),
                    },
                ],
            };
            out.push(fold);
            out.extend((0..after).map(|_| plain_para(filler_text(chars_per_para))));
            let target = vec![fold_index, 1, half - 1];
            (out, target)
        }
        _ => unreachable!("unknown shape {shape}"),
    }
}

fn assert_recursion_shape(view: &editor_model::DocView, shape: &str) {
    let root = view.root().expect("root exists");
    let recurses = root.children().any(|c| match c {
        ChildView::Block(b) => b.child_count() != b.leaf_child_count(),
        ChildView::Leaf(_) => false,
    });
    match shape {
        "flat" => assert!(
            !recurses,
            "flat fixture must stay on block_flat_size's O(1) leaf-only path"
        ),
        "nested-list" | "nested-fold" => assert!(
            recurses,
            "{shape} fixture must exercise block_flat_size's recursive path"
        ),
        _ => unreachable!("unknown shape {shape}"),
    }
}

fn build_state_with_shape(shape: &str, paras: usize, chars_per_para: usize) -> State {
    let (children, target_path) = shape_children(shape, paras, chars_per_para);
    let (mut state, handles) = build_state_from_plain(PlainDoc {
        root: root_entry(children),
    });
    assert_recursion_shape(&state.view(), shape);
    let target = *handles
        .get(&target_path)
        .expect("target path must resolve to a live block");
    let offset = chars_per_para.min(10);
    state.selection = Some(Selection::collapsed(Position::new(target, offset)));
    state
}

const WEB_IME: (usize, usize) = (64, 64);
const MOBILE_IME: (usize, usize) = (4096, 4096);

fn batch_median<T>(batches: usize, calls: usize, mut f: impl FnMut() -> T) -> Duration {
    for _ in 0..calls {
        std::hint::black_box(f());
    }
    let mut samples: Vec<Duration> = (0..batches)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..calls {
                std::hint::black_box(f());
            }
            t.elapsed() / calls as u32
        })
        .collect();
    samples.sort();
    samples[batches / 2]
}

#[test]
#[ignore]
fn perf_flat_decomposition() {
    for (shape, paras) in [
        ("flat", 100usize),
        ("flat", 1000),
        ("flat", 5000),
        ("nested-list", 1000),
        ("nested-fold", 1000),
        ("nested-list", 5000),
        ("nested-fold", 5000),
    ] {
        let base_state = build_state_with_shape(shape, paras, 50);
        let base = editor_from_state(&base_state);
        let view = base.state().view();
        let total = editor_state::flat_size(&view);
        let fs = batch_median(5, 100, || {
            editor_state::flat_size(std::hint::black_box(&view))
        });
        let caret = resolved_selection_caret(&base, &view);
        let caret_flat = caret.to_flat();
        assert_eq!(
            nearest_insertable_probe(&base, caret_flat),
            Some(caret_flat),
            "gate caret must be immediately insertable (n=1)"
        );
        let tf = batch_median(5, 100, || std::hint::black_box(&caret).to_flat());
        let ff_total = batch_median(5, 100, || {
            editor_state::ResolvedPosition::from_flat(
                std::hint::black_box(&view),
                std::hint::black_box(caret_flat),
            )
        });
        let ff_walk = batch_median(5, 100, || {
            editor_state::from_flat_walk_probe(
                std::hint::black_box(&view),
                std::hint::black_box(caret_flat),
            )
        });
        let walk_pos = editor_state::from_flat_walk_probe(&view, caret_flat).expect("in range");
        let ff_terminal = batch_median(5, 100, || {
            std::hint::black_box(&walk_pos).resolve(std::hint::black_box(&view))
        });
        let sel = resolve_selection(&base).expect("builder must install a selection");
        assert!(
            sel.is_collapsed(),
            "gate case is the collapsed caret (n=1 path)"
        );
        for (label, (before, after)) in [("web", WEB_IME), ("mobile", MOBILE_IME)] {
            {
                let mut warmup: Vec<Editor> =
                    (0..5).map(|_| editor_from_state(&base_state)).collect();
                for e in &mut warmup {
                    e.ime(before, after)
                        .expect("no error")
                        .expect("ime present");
                }
            }
            let fresh = {
                let mut samples: Vec<Duration> = (0..5)
                    .map(|_| {
                        let mut pool: Vec<Editor> =
                            (0..20).map(|_| editor_from_state(&base_state)).collect();
                        let t = Instant::now();
                        for e in &mut pool {
                            let r =
                                e.ime(std::hint::black_box(before), std::hint::black_box(after));
                            std::hint::black_box(r.expect("no error").expect("ime present"));
                        }
                        t.elapsed() / 20
                    })
                    .collect();
                samples.sort();
                samples[2]
            };
            let mut warm = editor_from_state(&base_state);
            warm.ime(before, after)
                .expect("no error")
                .expect("ime present");
            let steady = batch_median(5, 20, || {
                warm.ime(std::hint::black_box(before), std::hint::black_box(after))
                    .expect("no error")
                    .expect("ime present")
            });
            eprintln!("[{shape} paras={paras} {label}] ime fresh {fresh:?} steady {steady:?}");
        }
        eprintln!(
            "[{shape} paras={paras}] flat_size {fs:?} to_flat {tf:?} from_flat total {ff_total:?} walk {ff_walk:?} terminal {ff_terminal:?} (doc flat_size={total})"
        );
    }

    // nearest_insertable_flat at a non-insertable representative site (a block
    // open sentinel), measured directly against a flat fixture as a reference point.
    {
        let base_state = build_state_with_shape("nested-list", 1000, 50);
        let base = editor_from_state(&base_state);
        let view = base.state().view();
        let total = editor_state::flat_size(&view);
        let non_insertable = std::iter::successors(Some(0usize), |o| Some(o + 1))
            .take(total)
            .find(|&o| nearest_insertable_probe(&base, o) != Some(o))
            .expect("a nested shape has a non-insertable boundary offset");
        let n = batch_median(5, 100, || {
            nearest_insertable_probe(
                std::hint::black_box(&base),
                std::hint::black_box(non_insertable),
            )
        });
        eprintln!("[nested-list paras=1000] nearest_insertable_flat (non-insertable site) {n:?}");
    }
}

// ── synthetic-wrapper typing fixture ────────────────────────────────────────

#[test]
#[ignore]
fn perf_synthetic_wrapper_typing() {
    // Each bare `TableCell`'s sequence (all real ops):
    //   TableCell { parents: [ROOT] }
    //   Paragraph { parents: [ROOT, cell] }   ← the real typing host
    //   Char('a')                              ← inside the paragraph
    // Projection wraps the cell in a synthetic Table > TableRow — the host
    // paragraph is a real block, so incremental insertion is accepted and its
    // flat-width delta propagates through the synthetic ancestor chain.
    fn build(fanout: usize, mixed: bool) -> (State, Dot) {
        use editor_crdt::{Changeset, Op};
        use editor_model::{EditOp, NodeType, SeqItem};
        let mut ops: Vec<Op<EditOp>> = Vec::new();
        let mut pos = 0usize;
        let mut prev: Option<Dot> = None;
        let mut clock = 1u64;
        let push = |ops: &mut Vec<Op<EditOp>>,
                    pos: &mut usize,
                    prev: &mut Option<Dot>,
                    clock: &mut u64,
                    item: SeqItem|
         -> Dot {
            let id = Dot::new(1, *clock);
            *clock += 1;
            ops.push(Op {
                id,
                parents: (*prev).into_iter().collect(),
                payload: EditOp::Seq(editor_crdt::ListOp::Ins { pos: *pos, item }),
            });
            *pos += 1;
            *prev = Some(id);
            id
        };
        let para = |parents: Vec<Dot>| SeqItem::Block {
            node_type: NodeType::Paragraph,
            parents,
            attrs: vec![],
        };
        let mut host = Dot::ROOT;
        for i in 0..fanout {
            let is_cell_slot = if mixed { i % 2 == 1 } else { i == fanout - 1 };
            if is_cell_slot {
                let cell = push(
                    &mut ops,
                    &mut pos,
                    &mut prev,
                    &mut clock,
                    SeqItem::Block {
                        node_type: NodeType::TableCell,
                        parents: vec![Dot::ROOT],
                        attrs: vec![],
                    },
                );
                let p = push(
                    &mut ops,
                    &mut pos,
                    &mut prev,
                    &mut clock,
                    para(vec![Dot::ROOT, cell]),
                );
                push(
                    &mut ops,
                    &mut pos,
                    &mut prev,
                    &mut clock,
                    SeqItem::Char('a'),
                );
                host = p;
            } else {
                push(
                    &mut ops,
                    &mut pos,
                    &mut prev,
                    &mut clock,
                    para(vec![Dot::ROOT]),
                );
            }
        }
        let selection = Some(Selection::collapsed(Position::new(host, 1)));
        let state = State::from_changesets(vec![Changeset { ops }], selection).unwrap();
        (state, host)
    }
    for mixed in [false, true] {
        let (s10, host10) = build(10, mixed);
        let (s1000, host1000) = build(1000, mixed);
        for (state, host, fanout) in [(&s10, host10, 10usize), (&s1000, host1000, 1000)] {
            let view = state.view();
            let h = view.node(host).expect("typing host is live");
            assert!(h.dot().is_some(), "typing host must be a real block");
            assert!(
                h.parent().is_some_and(
                    |p| p.dot().is_some() && p.node_type() == editor_model::NodeType::TableCell
                ),
                "typing host's direct parent must be the real cell"
            );
            assert!(
                h.ancestors().any(|a| a.dot().is_none()),
                "typing host must sit under a synthetic ancestor chain"
            );
            let root = view.root().expect("root");
            assert_eq!(
                root.child_count(),
                fanout + 1,
                "root fan-out must match the cell plus Root's synthesized trailing paragraph"
            );
            let synthetic = root.child_blocks().filter(|b| b.dot().is_none()).count();
            let expected = (if mixed { fanout / 2 } else { 1 }) + 1;
            assert_eq!(
                synthetic, expected,
                "synthetic wrapper density must match the cell plus the trailing paragraph"
            );
            let mut probe = editor_from_state(state);
            let before = visible_len(probe.state());
            type_one_char(&mut probe);
            assert_eq!(
                visible_len(probe.state()),
                before + 1,
                "typing must land as one visible unit"
            );
        }
        let mut ratios: Vec<f64> = Vec::new();
        for _ in 0..20 {
            let mut e10 = editor_from_state(&s10);
            let k10 = batch_median(3, 20, || type_one_char(&mut e10));
            let mut e1000 = editor_from_state(&s1000);
            let k1000 = batch_median(3, 20, || type_one_char(&mut e1000));
            if k10.as_secs_f64() > 0.0 {
                ratios.push(k1000.as_secs_f64() / k10.as_secs_f64());
            }
        }
        eprintln!(
            "[synthetic-wrapper mixed={mixed}] per-key slope {:.2}",
            median_f64(&mut ratios)
        );
    }
}

// ── Step 3: tracked resolve matrix ──────────────────────────────────────────

const RANGE_SPAN: usize = 3;

#[cfg(feature = "resolve-stats")]
type BranchSnapshot = [resolve_stats::SiteStats; 6];
#[cfg(not(feature = "resolve-stats"))]
type BranchSnapshot = ();

fn branch_reset() {
    #[cfg(feature = "resolve-stats")]
    resolve_stats::reset();
}

#[cfg(feature = "resolve-stats")]
fn branch_snapshot_normalized(b: u32) -> BranchSnapshot {
    let snap = resolve_stats::snapshot();
    std::array::from_fn(|i| resolve_stats::SiteStats {
        calls: snap[i].calls / b as u64,
        elapsed: snap[i].elapsed / b,
        fanout_sum: snap[i].fanout_sum / b as u64,
        fast_calls: snap[i].fast_calls,
    })
}
#[cfg(not(feature = "resolve-stats"))]
fn branch_snapshot_normalized(_b: u32) -> BranchSnapshot {}

struct PairedRow {
    k10: [(Duration, BranchSnapshot); 3],
    k1000: [(Duration, BranchSnapshot); 3],
}

fn visible_len(state: &State) -> usize {
    editor_state::flat_size(&state.view())
}

fn type_one_char(e: &mut Editor) {
    e.apply(Message::Insertion {
        op: InsertionOp::Text {
            text: "x".to_string(),
        },
    });
}

fn measured_paragraphs(count: usize, chars: usize) -> Vec<PlainNodeEntry> {
    (0..count).map(|_| plain_para(filler_text(chars))).collect()
}

fn filler_paragraphs(count: usize) -> Vec<PlainNodeEntry> {
    (0..count).map(|_| plain_para(filler_text(5))).collect()
}

fn build_live_ranges(
    shape: &str,
    k: usize,
    paras: usize,
    range_count: usize,
) -> (State, TrackedRangeRegistry, Vec<TrackedRangeId>) {
    let filler_count = paras.saturating_sub(range_count).max(1);
    let measured = measured_paragraphs(range_count, k);
    let filler = filler_paragraphs(filler_count);

    let mut root_children: Vec<PlainNodeEntry> = Vec::new();
    let measured_paths: Vec<Vec<usize>> = match shape {
        "top" => {
            let paths = (0..range_count).map(|i| vec![i]).collect();
            root_children.extend(measured);
            paths
        }
        "list" => {
            let paths = (0..range_count).map(|i| vec![0usize, i, 0usize]).collect();
            root_children.push(bullet_list_wrapping(measured));
            paths
        }
        _ => unreachable!("unknown shape {shape}"),
    };
    root_children.extend(filler);
    let caret_path = vec![root_children.len() - 1];

    let (mut state, handles) = build_state_from_plain(PlainDoc {
        root: root_entry(root_children),
    });

    let mut reg = TrackedRangeRegistry::new();
    let mut ids: Vec<TrackedRangeId> = Vec::new();
    {
        let pre_view = state.view();
        for (i, path) in measured_paths.iter().enumerate() {
            let para = *handles.get(path).expect("measured paragraph path resolves");
            assert_eq!(
                pre_view
                    .node(para)
                    .expect("measured host is live")
                    .child_count(),
                k,
                "live fixture measured host must have exactly K children"
            );
            // Anchored at the host's tail (not offset 0): `index_of`'s linear scan
            // must walk nearly the full child list before matching, so K's effect
            // on the dominant per-key site is actually exercised.
            let sel = Selection::new(Position::new(para, k - RANGE_SPAN), Position::new(para, k));
            let stable = StableSelection::capture(&sel, &pre_view);
            let id: TrackedRangeId = format!("r{i}");
            reg.add(TrackedRange::new(
                id.clone(),
                "g".into(),
                stable,
                String::new(),
                &state,
            ));
            ids.push(id);
        }
    }

    let caret = *handles.get(&caret_path).expect("caret path resolves");
    state.selection = Some(Selection::collapsed(Position::new(caret, 0)));
    (state, reg, ids)
}

fn build_deleted_marker_ranges(
    shape: &str,
    k: usize,
    paras: usize,
    range_count: usize,
) -> (State, TrackedRangeRegistry, Vec<TrackedRangeId>) {
    let filler_count = paras.saturating_sub(range_count).max(1);
    let markers = measured_paragraphs(range_count, k + 1);
    let filler = filler_paragraphs(filler_count);

    let mut root_children: Vec<PlainNodeEntry> = Vec::new();
    let marker_paths: Vec<Vec<usize>> = match shape {
        "top" => {
            let paths = (0..range_count).map(|i| vec![i]).collect();
            root_children.extend(markers);
            paths
        }
        "list" => {
            let paths = (0..range_count).map(|i| vec![0usize, i, 0usize]).collect();
            root_children.push(bullet_list_wrapping(markers));
            paths
        }
        _ => unreachable!("unknown shape {shape}"),
    };
    root_children.extend(filler);
    let caret_path = vec![root_children.len() - 1];

    let (state, handles) = build_state_from_plain(PlainDoc {
        root: root_entry(root_children),
    });

    let marker_dots: Vec<Dot> = marker_paths
        .iter()
        .map(|p| *handles.get(p).expect("marker path resolves"))
        .collect();

    let mut reg = TrackedRangeRegistry::new();
    let mut ids: Vec<TrackedRangeId> = Vec::new();
    {
        let pre_view = state.view();
        for (i, &marker) in marker_dots.iter().enumerate() {
            let sel = Selection::new(Position::new(marker, 0), Position::new(marker, k + 1));
            let stable = StableSelection::capture(&sel, &pre_view);
            let id: TrackedRangeId = format!("r{i}");
            reg.add(TrackedRange::new(
                id.clone(),
                "g".into(),
                stable,
                String::new(),
                &state,
            ));
            ids.push(id);
        }
    }

    let mut tr = Transaction::new(&state);
    for &marker in &marker_dots {
        tr.remove_text(marker, k, 1)
            .expect("marker char removal applies");
    }
    let (mut state, ..) = tr.commit();

    {
        let post_view = state.view();
        for &marker in &marker_dots {
            assert_eq!(
                post_view
                    .node(marker)
                    .expect("marker host survives deletion")
                    .child_count(),
                k,
                "deleted-marker fixture host must have exactly K children after deletion"
            );
        }
    }

    let caret = *handles.get(&caret_path).expect("caret path resolves");
    state.selection = Some(Selection::collapsed(Position::new(caret, 0)));
    (state, reg, ids)
}

fn build_wrapper_move_ranges(
    shape: &str,
    k: usize,
    paras: usize,
    range_count: usize,
) -> (State, TrackedRangeRegistry, Vec<TrackedRangeId>) {
    fn fold_of(content_children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
        PlainNodeEntry {
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
                    children: content_children,
                },
            ],
        }
    }

    let targets: Vec<PlainNodeEntry> = (0..k).map(|_| plain_para(filler_text(1))).collect();
    // `Blockquote`'s content spec is `(Paragraph | BulletList | OrderedList)+` — it
    // cannot nest another `Blockquote` directly. `wrap_host` is instead a Fold's
    // `FoldContent`, whose spec allows both a flat run of Paragraphs and a nested
    // Blockquote sibling, so `k` flat targets plus one nested wrapper is valid.
    let nested = PlainNodeEntry {
        node: PlainNode::Blockquote(PlainBlockquoteNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![plain_para(filler_text(1))],
    };
    let mut wrap_children = targets;
    wrap_children.push(nested);

    let filler = filler_paragraphs(paras.max(1));

    let (wrap_path, nested_path, mut root_children): (Vec<usize>, Vec<usize>, Vec<PlainNodeEntry>) =
        match shape {
            "top" => (vec![0, 1], vec![0, 1, k], vec![fold_of(wrap_children)]),
            "list" => (
                vec![0, 1, 0, 1],
                vec![0, 1, 0, 1, k],
                vec![fold_of(vec![fold_of(wrap_children)])],
            ),
            _ => unreachable!("unknown shape {shape}"),
        };
    let mut target_path = wrap_path.clone();
    target_path.push(0);

    root_children.extend(filler);
    let caret_path = vec![root_children.len() - 1];

    let (state, handles) = build_state_from_plain(PlainDoc {
        root: root_entry(root_children),
    });

    let wrap_host_dot = *handles.get(&wrap_path).expect("wrap host path resolves");
    let target_dot = *handles
        .get(&target_path)
        .expect("target paragraph path resolves");
    let nested_dot = *handles
        .get(&nested_path)
        .expect("nested wrapper path resolves");

    let mut reg = TrackedRangeRegistry::new();
    let mut ids: Vec<TrackedRangeId> = Vec::new();
    {
        let pre_view = state.view();
        assert_eq!(
            pre_view
                .node(wrap_host_dot)
                .expect("wrap host is live")
                .child_count(),
            k + 1,
            "wrapper-move fixture host must have exactly K+1 children before the move"
        );
        let sel = Selection::new(
            Position::new(wrap_host_dot, 0),
            Position::new(wrap_host_dot, 1),
        );
        let stable = StableSelection::capture(&sel, &pre_view);
        for i in 0..range_count {
            let id: TrackedRangeId = format!("r{i}");
            reg.add(TrackedRange::new(
                id.clone(),
                "g".into(),
                stable.clone(),
                String::new(),
                &state,
            ));
            ids.push(id);
        }
    }

    let mut tr = Transaction::new(&state);
    tr.move_node(target_dot, nested_dot, 1)
        .expect("target paragraph moves into the nested wrapper");
    let (mut state, ..) = tr.commit();

    {
        let post_view = state.view();
        assert_eq!(
            post_view
                .node(wrap_host_dot)
                .expect("wrap host survives the move")
                .child_count(),
            k,
            "wrapper-move fixture host must have exactly K children after the move"
        );
    }

    let caret = *handles.get(&caret_path).expect("caret path resolves");
    state.selection = Some(Selection::collapsed(Position::new(caret, 0)));
    (state, reg, ids)
}

fn build_state_with_ranges(
    shape: &str,
    fixture: &str,
    k: usize,
    paras: usize,
    range_count: usize,
) -> (State, TrackedRangeRegistry, Vec<TrackedRangeId>) {
    match fixture {
        "live" => build_live_ranges(shape, k, paras, range_count),
        "deleted-marker" => build_deleted_marker_ranges(shape, k, paras, range_count),
        "wrapper-move" => build_wrapper_move_ranges(shape, k, paras, range_count),
        _ => unreachable!("unknown fixture {fixture}"),
    }
}

#[cfg(feature = "resolve-stats")]
fn assert_branch_fires(
    fixture: &str,
    state: &State,
    reg: &TrackedRangeRegistry,
    ids: &[TrackedRangeId],
) {
    resolve_stats::reset();
    let hits: usize = ids
        .iter()
        .filter_map(|id| reg.get(id).expect("registered").locate(state))
        .count();
    assert_eq!(hits, ids.len(), "diagnostic fixture ranges must all locate");
    let snap = resolve_stats::snapshot();
    match fixture {
        "live" => assert!(
            snap.iter().map(|s| s.calls).sum::<u64>() > 0,
            "live fixture must exercise some resolve site"
        ),
        "deleted-marker" => assert!(
            snap[resolve_stats::DEAD_CHILD_OFFSET_WITHIN].calls > 0,
            "deleted-marker fixture must hit DEAD_CHILD_OFFSET_WITHIN"
        ),
        "wrapper-move" => assert!(
            snap[resolve_stats::CONTAINING_INDEX_OF].calls > 0
                || snap[resolve_stats::CONTAINING_INDEX_OF].fast_calls > 0,
            "wrapper-move fixture must reach the containing-child branch (fast or fallback)"
        ),
        _ => unreachable!("unknown fixture {fixture}"),
    }
}

fn measure_row(
    base_state: &State,
    reg: &TrackedRangeRegistry,
    ids: &[TrackedRangeId],
) -> [(Duration, BranchSnapshot); 3] {
    const B: u32 = 20;
    let mut e = editor_from_state(base_state);
    let before_len = visible_len(e.state());
    type_one_char(&mut e);
    assert_eq!(visible_len(e.state()), before_len + 1);
    let subsets: [Vec<&TrackedRange>; 3] = [0usize, 50, 200].map(|n| {
        ids[..n]
            .iter()
            .map(|id| reg.get(id).expect("registered"))
            .collect()
    });
    subsets.map(|subset| {
        let hits: usize = subset.iter().filter_map(|r| r.locate(e.state())).count();
        assert_eq!(hits, subset.len());
        branch_reset();
        let t = Instant::now();
        for _ in 0..B {
            let hits: usize = subset
                .iter()
                .filter_map(|r| std::hint::black_box(r).locate(std::hint::black_box(e.state())))
                .count();
            std::hint::black_box(hits);
        }
        (t.elapsed() / B, branch_snapshot_normalized(B))
    })
}

fn median_duration(vals: &mut [Duration]) -> Duration {
    vals.sort();
    vals[vals.len() / 2]
}

fn median_f64(vals: &mut [f64]) -> f64 {
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    vals[vals.len() / 2]
}

fn report_cell(shape: &str, fixture: &str, paras: usize, rows: &[PairedRow]) {
    let mut per_range_k10: Vec<Duration> = Vec::new();
    let mut per_range_k1000: Vec<Duration> = Vec::new();
    let mut ratios: Vec<f64> = Vec::new();
    let mut raw_r0_k10: Vec<Duration> = Vec::new();
    let mut raw_r200_k10: Vec<Duration> = Vec::new();
    let mut raw_r0_k1000: Vec<Duration> = Vec::new();
    let mut raw_r200_k1000: Vec<Duration> = Vec::new();
    for row in rows {
        let r10 = row.k10[2].0.saturating_sub(row.k10[0].0) / 200;
        let r1000 = row.k1000[2].0.saturating_sub(row.k1000[0].0) / 200;
        let r10_secs = r10.as_secs_f64();
        if r10_secs > 0.0 {
            ratios.push(r1000.as_secs_f64() / r10_secs);
        }
        per_range_k10.push(r10);
        per_range_k1000.push(r1000);
        raw_r0_k10.push(row.k10[0].0);
        raw_r200_k10.push(row.k10[2].0);
        raw_r0_k1000.push(row.k1000[0].0);
        raw_r200_k1000.push(row.k1000[2].0);
    }
    eprintln!(
        "[shape={shape} fixture={fixture} paras={paras}] per-range K=10 {:?} K=1000 {:?} ratio(median-of-ratios) {:.2}",
        median_duration(&mut per_range_k10),
        median_duration(&mut per_range_k1000),
        if ratios.is_empty() {
            0.0
        } else {
            median_f64(&mut ratios)
        },
    );
    eprintln!(
        "  raw locate wall: r0 K=10 {:?} K=1000 {:?} | r200 K=10 {:?} K=1000 {:?}",
        median_duration(&mut raw_r0_k10),
        median_duration(&mut raw_r0_k1000),
        median_duration(&mut raw_r200_k10),
        median_duration(&mut raw_r200_k1000),
    );

    #[cfg(feature = "resolve-stats")]
    {
        for site in 0..6 {
            let mut occ_k10: Vec<f64> = Vec::new();
            let mut occ_k1000: Vec<f64> = Vec::new();
            let mut fanout_k10: Vec<f64> = Vec::new();
            let mut fanout_k1000: Vec<f64> = Vec::new();
            for row in rows {
                let wall10 = row.k10[2].0.as_secs_f64();
                let wall1000 = row.k1000[2].0.as_secs_f64();
                let s10 = row.k10[2].1[site];
                let s1000 = row.k1000[2].1[site];
                if wall10 > 0.0 {
                    occ_k10.push(s10.elapsed.as_secs_f64() / wall10);
                }
                if wall1000 > 0.0 {
                    occ_k1000.push(s1000.elapsed.as_secs_f64() / wall1000);
                }
                if s10.calls > 0 {
                    fanout_k10.push(s10.fanout_sum as f64 / s10.calls as f64);
                }
                if s1000.calls > 0 {
                    fanout_k1000.push(s1000.fanout_sum as f64 / s1000.calls as f64);
                }
            }
            eprintln!(
                "  site={site} occupancy K=10 {:.4} K=1000 {:.4} avg_fanout K=10 {:.1} K=1000 {:.1}",
                if occ_k10.is_empty() {
                    0.0
                } else {
                    median_f64(&mut occ_k10)
                },
                if occ_k1000.is_empty() {
                    0.0
                } else {
                    median_f64(&mut occ_k1000)
                },
                if fanout_k10.is_empty() {
                    0.0
                } else {
                    median_f64(&mut fanout_k10)
                },
                if fanout_k1000.is_empty() {
                    0.0
                } else {
                    median_f64(&mut fanout_k1000)
                },
            );
        }
    }
}

#[test]
#[ignore]
fn perf_tracked_decomposition() {
    for shape in ["top", "list"] {
        for fixture in ["live", "deleted-marker", "wrapper-move"] {
            for paras in [1000usize, 5000] {
                let (state_k10, reg_k10, ids_k10) =
                    build_state_with_ranges(shape, fixture, 10, paras, 200);
                let (state_k1000, reg_k1000, ids_k1000) =
                    build_state_with_ranges(shape, fixture, 1000, paras, 200);

                #[cfg(feature = "resolve-stats")]
                assert_branch_fires(fixture, &state_k10, &reg_k10, &ids_k10);

                let mut rows: Vec<PairedRow> = Vec::new();
                for _ in 0..20 {
                    let row_k10 = measure_row(&state_k10, &reg_k10, &ids_k10);
                    let row_k1000 = measure_row(&state_k1000, &reg_k1000, &ids_k1000);
                    rows.push(PairedRow {
                        k10: row_k10,
                        k1000: row_k1000,
                    });
                }
                report_cell(shape, fixture, paras, &rows);
            }
        }
    }
}

// ── Step 8: ancestor-index (A-axis) scaling ─────────────────────────────────

#[test]
#[ignore]
fn perf_ancestor_index_scaling() {
    fn build(a: usize) -> (State, Position) {
        let children: Vec<PlainNodeEntry> = (0..a).map(|_| plain_para(filler_text(1))).collect();
        let (state, handles) = build_state_from_plain(PlainDoc {
            root: root_entry(children),
        });
        let target = *handles.get(&vec![a - 1]).expect("last child resolves");
        (state, Position::new(target, 0))
    }
    fn measure_unordered(state: &State, pos: &Position) -> Duration {
        let view = editor_model::DocView::new(state.projected.projected());
        batch_median(3, 50, || {
            std::hint::black_box(pos).resolve(std::hint::black_box(&view))
        })
    }
    fn measure_ordered(state: &State, pos: &Position) -> Duration {
        let view = state.view();
        batch_median(3, 50, || {
            std::hint::black_box(pos).resolve(std::hint::black_box(&view))
        })
    }

    let (s10, p10) = build(10);
    let (s1000, p1000) = build(1000);

    #[cfg(feature = "resolve-stats")]
    {
        let unordered = editor_model::DocView::new(s1000.projected.projected());
        resolve_stats::reset();
        let _ = p1000.resolve(&unordered);
        let snap = resolve_stats::snapshot();
        let s = snap[resolve_stats::ANCESTOR_INDEX];
        assert!(s.calls > 0, "A-axis fixture must hit ANCESTOR_INDEX");
        let avg = s.fanout_sum as f64 / s.calls as f64;
        assert!(
            (avg / 1000.0 - 1.0).abs() < 0.1,
            "avg fanout must track A, got {avg}"
        );
    }

    let mut baseline_ratios: Vec<f64> = Vec::new();
    let mut fast_ratios: Vec<f64> = Vec::new();
    for _ in 0..20 {
        let b10 = measure_unordered(&s10, &p10);
        let b1000 = measure_unordered(&s1000, &p1000);
        let f10 = measure_ordered(&s10, &p10);
        let f1000 = measure_ordered(&s1000, &p1000);
        if b10.as_secs_f64() > 0.0 {
            baseline_ratios.push(b1000.as_secs_f64() / b10.as_secs_f64());
        }
        if f10.as_secs_f64() > 0.0 {
            fast_ratios.push(f1000.as_secs_f64() / f10.as_secs_f64());
        }
    }
    eprintln!(
        "[A-axis] baseline slope {:.2} fast slope {:.2}",
        median_f64(&mut baseline_ratios),
        median_f64(&mut fast_ratios),
    );
}

// ── Step 4: character_counts decomposition ──────────────────────────────────

fn build_editor_with_paragraphs(paras: usize, chars_per_para: usize) -> Editor {
    let state = build_state_with_shape("flat", paras, chars_per_para);
    editor_from_state(&state)
}

#[test]
#[ignore]
fn perf_character_counts() {
    for paras in [100usize, 1000, 5000] {
        let editor = build_editor_with_paragraphs(paras, 50);
        let c = batch_median(5, 10, || std::hint::black_box(&editor).character_counts());
        eprintln!("[paras={paras}] character_counts {c:?}");
    }
}
