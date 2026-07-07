use std::ops::Range;

use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeView};

use crate::{Position, ResolvedPosition};

/// Flat-text sentinel for a block boundary's leading edge (mirrors `collect_chars`).
pub const FLAT_OPEN: char = '\u{2028}';
/// Flat-text sentinel for a block boundary's trailing edge (mirrors `collect_chars`).
pub const FLAT_CLOSE: char = '\u{2029}';

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlatSegment {
    Open { block: Dot },
    Close { block: Dot },
    Text { block: Dot, leaves: Vec<Dot> },
    Break { leaf: Dot },
    Atom { leaf: Dot },
}

enum ChildClass {
    Char(Dot),
    Break(Dot),
    Atom(Dot),
    Block,
}

fn classify(c: &ChildView) -> ChildClass {
    match c {
        ChildView::Block(_) => ChildClass::Block,
        ChildView::Leaf(l) => {
            if l.as_char().is_some() {
                ChildClass::Char(l.dot())
            } else if l.node_type().spec().inline {
                ChildClass::Break(l.dot())
            } else {
                ChildClass::Atom(l.dot())
            }
        }
    }
}

fn block_flat_size(b: &NodeView) -> usize {
    // A block contributes its two boundary sentinels plus one flat unit per direct
    // leaf; nested blocks recurse. The overwhelmingly common block (a paragraph with
    // only inline leaves) is `O(1)` via the leaf-count summary — no child walk — which
    // keeps `to_flat`/`from_flat` off the per-character path.
    let total = b.child_count();
    let leaves = b.leaf_child_count();
    if leaves == total {
        return 2 + total;
    }
    2 + b.children().map(|c| child_flat_size(&c)).sum::<usize>()
}

fn child_flat_size(c: &ChildView) -> usize {
    match c {
        ChildView::Leaf(_) => 1,
        ChildView::Block(b) => block_flat_size(b),
    }
}

pub fn flat_size(view: &DocView) -> usize {
    match view.root() {
        Some(root) => root.children().map(|c| child_flat_size(&c)).sum(),
        None => 0,
    }
}

pub trait ResolvedPositionFlatExt<'a>: Sized {
    fn to_flat(&self) -> usize;
    fn from_flat(view: &'a DocView<'a>, flat: usize) -> Option<Self>;
}

impl<'a> ResolvedPositionFlatExt<'a> for ResolvedPosition<'a> {
    fn to_flat(&self) -> usize {
        let view = self.view();
        let Some(root) = view.root() else {
            return 0;
        };
        let target = self.node();
        // Ancestor chain of the target block (inclusive). The walk descends only into
        // blocks on this chain and skips every other subtree via the `O(1)`
        // `block_flat_size`, so `to_flat` is `O(blocks traversed)` — no per-character
        // DFS through sibling subtrees. Depth is shallow, so a `Vec` membership test is
        // cheaper than hashing.
        let mut ancestors: Vec<Dot> = Vec::new();
        let mut cur = view.node(target);
        while let Some(n) = cur {
            ancestors.push(n.id());
            cur = n.parent();
        }
        to_flat_walk(&root, target, self.offset(), &ancestors).unwrap_or(0)
    }

    fn from_flat(view: &'a DocView<'a>, flat: usize) -> Option<Self> {
        let root = view.root()?;
        let pos = from_flat_walk(&root, 0, flat)?;
        pos.resolve(view)
    }
}

fn to_flat_walk(
    current: &NodeView,
    target: Dot,
    target_offset: usize,
    ancestors: &[Dot],
) -> Option<usize> {
    if current.id() == target {
        let mut acc = 0usize;
        for (i, c) in current.children().enumerate() {
            if i == target_offset {
                return Some(acc);
            }
            acc += child_flat_size(&c);
        }
        return Some(acc);
    }
    let mut acc = 0usize;
    for c in current.children() {
        match c {
            ChildView::Block(b) => {
                acc += 1;
                // Only descend into a block on the target's ancestor chain; every other
                // subtree is skipped by its `O(1)` flat size, never DFS-searched.
                if ancestors.contains(&b.id())
                    && let Some(inner) = to_flat_walk(&b, target, target_offset, ancestors)
                {
                    return Some(acc + inner);
                }
                acc += block_flat_size(&b) - 2;
                acc += 1;
            }
            ChildView::Leaf(_) => acc += 1,
        }
    }
    None
}

fn from_flat_walk(container: &NodeView, start_flat: usize, target: usize) -> Option<Position> {
    let mut acc = start_flat;
    for (i, c) in container.children().enumerate() {
        match c {
            ChildView::Block(b) => {
                let content = block_flat_size(&b) - 2;
                if target == acc {
                    return Some(Position::new(container.id(), i));
                }
                acc += 1;
                if target >= acc && target <= acc + content {
                    return from_flat_walk(&b, acc, target);
                }
                acc += content;
                if target == acc {
                    return Some(Position::new(b.id(), b.child_count()));
                }
                acc += 1;
            }
            ChildView::Leaf(_) => {
                if target == acc {
                    return Some(Position::new(container.id(), i));
                }
                acc += 1;
            }
        }
    }
    if target == acc {
        return Some(Position::new(container.id(), container.child_count()));
    }
    None
}

pub fn flat_segments(view: &DocView) -> Vec<FlatSegment> {
    let mut out = Vec::new();
    if let Some(root) = view.root() {
        emit_children(&root, &mut out);
    }
    out
}

fn emit_children(block: &NodeView, out: &mut Vec<FlatSegment>) {
    let mut run: Vec<Dot> = Vec::new();
    let block_id = block.id();
    for c in block.children() {
        match classify(&c) {
            ChildClass::Char(d) => run.push(d),
            other => {
                flush_text(block_id, &mut run, out);
                match other {
                    ChildClass::Break(d) => out.push(FlatSegment::Break { leaf: d }),
                    ChildClass::Atom(d) => out.push(FlatSegment::Atom { leaf: d }),
                    ChildClass::Block => {
                        if let ChildView::Block(b) = c {
                            out.push(FlatSegment::Open { block: b.id() });
                            emit_children(&b, out);
                            out.push(FlatSegment::Close { block: b.id() });
                        }
                    }
                    ChildClass::Char(..) => unreachable!(),
                }
            }
        }
    }
    flush_text(block_id, &mut run, out);
}

fn flush_text(block: Dot, run: &mut Vec<Dot>, out: &mut Vec<FlatSegment>) {
    if !run.is_empty() {
        out.push(FlatSegment::Text {
            block,
            leaves: std::mem::take(run),
        });
    }
}

pub fn flat_chars(view: &DocView, range: Range<usize>) -> Vec<char> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    if let Some(root) = view.root() {
        collect_chars(&root, &mut idx, &range, &mut out);
    }
    out
}

pub fn flat_text(view: &DocView, range: Range<usize>) -> String {
    flat_chars(view, range).into_iter().collect()
}

fn collect_chars(block: &NodeView, idx: &mut usize, range: &Range<usize>, out: &mut Vec<char>) {
    for c in block.children() {
        if *idx >= range.end {
            return;
        }
        match c {
            ChildView::Block(b) => {
                // Skip whole subtrees before the window via their O(1) flat size,
                // so a small window never walks the rest of the document.
                let size = block_flat_size(&b);
                if *idx + size <= range.start {
                    *idx += size;
                    continue;
                }
                push_unit(FLAT_OPEN, idx, range, out);
                collect_chars(&b, idx, range, out);
                push_unit(FLAT_CLOSE, idx, range, out);
            }
            ChildView::Leaf(l) => {
                let ch = match l.as_char() {
                    Some(c) => c,
                    None if l.node_type().spec().inline => '\n',
                    None => '\u{fffc}',
                };
                push_unit(ch, idx, range, out);
            }
        }
    }
}

fn push_unit(ch: char, idx: &mut usize, range: &Range<usize>, out: &mut Vec<char>) {
    if range.contains(idx) {
        out.push(ch);
    }
    *idx += 1;
}

pub fn flat_segments_in_range(view: &DocView, range: Range<usize>) -> Vec<FlatSegment> {
    flat_segments_in_range_with_pos(view, range)
        .into_iter()
        .map(|(_, seg)| seg)
        .collect()
}

/// Segments overlapping `range`, each with its absolute flat start position.
/// Text segments are trimmed to the range. Skips whole subtrees outside the
/// range via their O(1) flat size and stops at `range.end`, so the walk is
/// O(blocks before the range + range) rather than O(document).
pub fn flat_segments_in_range_with_pos(
    view: &DocView,
    range: Range<usize>,
) -> Vec<(usize, FlatSegment)> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    if let Some(root) = view.root() {
        collect_segments_in_range(&root, &mut idx, &range, &mut out);
    }
    out
}

fn collect_segments_in_range(
    block: &NodeView,
    idx: &mut usize,
    range: &Range<usize>,
    out: &mut Vec<(usize, FlatSegment)>,
) -> bool {
    let block_id = block.id();
    let mut run: Vec<Dot> = Vec::new();
    let mut run_start = 0usize;
    fn flush(
        block: Dot,
        run: &mut Vec<Dot>,
        run_start: usize,
        out: &mut Vec<(usize, FlatSegment)>,
    ) {
        if !run.is_empty() {
            out.push((
                run_start,
                FlatSegment::Text {
                    block,
                    leaves: std::mem::take(run),
                },
            ));
        }
    }
    for c in block.children() {
        if *idx >= range.end {
            flush(block_id, &mut run, run_start, out);
            return false;
        }
        match classify(&c) {
            ChildClass::Char(d) => {
                if *idx >= range.start {
                    if run.is_empty() {
                        run_start = *idx;
                    }
                    run.push(d);
                }
                *idx += 1;
            }
            other => {
                flush(block_id, &mut run, run_start, out);
                match other {
                    ChildClass::Break(d) => {
                        if *idx >= range.start {
                            out.push((*idx, FlatSegment::Break { leaf: d }));
                        }
                        *idx += 1;
                    }
                    ChildClass::Atom(d) => {
                        if *idx >= range.start {
                            out.push((*idx, FlatSegment::Atom { leaf: d }));
                        }
                        *idx += 1;
                    }
                    ChildClass::Block => {
                        let ChildView::Block(b) = c else {
                            unreachable!()
                        };
                        let size = block_flat_size(&b);
                        if *idx + size <= range.start {
                            *idx += size;
                            continue;
                        }
                        if *idx >= range.start {
                            out.push((*idx, FlatSegment::Open { block: b.id() }));
                        }
                        *idx += 1;
                        if !collect_segments_in_range(&b, idx, range, out) {
                            return false;
                        }
                        if *idx >= range.start && *idx < range.end {
                            out.push((*idx, FlatSegment::Close { block: b.id() }));
                        }
                        *idx += 1;
                    }
                    ChildClass::Char(..) => unreachable!(),
                }
            }
        }
    }
    flush(block_id, &mut run, run_start, out);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        Anchor, AtomLeaf, Bias, DocLogs, HorizontalRuleVariant, Modifier, ModifierAttrLog,
        ModifierType, NodeAttrLog, NodeType, ProjectedDoc, SeqItem, SpanLog, SpanOp,
        project_document,
    };

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
        logs_with_spans(items, SpanLog::new())
    }

    fn logs_with_spans(items: &[(Dot, SeqItem)], spans: SpanLog) -> DocLogs {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        DocLogs {
            seq: build_oplog(&ev),
            spans,
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
        }
    }

    fn para_doc(children: &[SeqItem]) -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
            },
        )];
        for (i, c) in children.iter().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), c.clone()));
        }
        (project_document(&logs(&items)).unwrap(), para)
    }

    // Reference flat conversions: the original always-recursive block size and the
    // unguarded DFS walk. The optimized production versions (O(1) leaf-block size +
    // ancestor-guarded descent) must agree with these for every position.
    fn block_flat_size_ref(b: &NodeView) -> usize {
        2 + b.children().map(|c| child_flat_size_ref(&c)).sum::<usize>()
    }
    fn child_flat_size_ref(c: &ChildView) -> usize {
        match c {
            ChildView::Leaf(_) => 1,
            ChildView::Block(b) => block_flat_size_ref(b),
        }
    }
    fn to_flat_ref(view: &DocView, target: Dot, offset: usize) -> usize {
        fn walk(current: &NodeView, target: Dot, target_offset: usize) -> Option<usize> {
            if current.id() == target {
                let mut acc = 0usize;
                for (i, c) in current.children().enumerate() {
                    if i == target_offset {
                        return Some(acc);
                    }
                    acc += child_flat_size_ref(&c);
                }
                return Some(acc);
            }
            let mut acc = 0usize;
            for c in current.children() {
                match c {
                    ChildView::Block(b) => {
                        acc += 1;
                        if let Some(inner) = walk(&b, target, target_offset) {
                            return Some(acc + inner);
                        }
                        acc += block_flat_size_ref(&b) - 2;
                        acc += 1;
                    }
                    ChildView::Leaf(_) => acc += 1,
                }
            }
            None
        }
        match view.root() {
            Some(root) => walk(&root, target, offset).unwrap_or(0),
            None => 0,
        }
    }

    #[derive(Clone, Debug)]
    enum BlockSpec {
        Para(usize),
        Quote(Vec<usize>),
    }

    fn build_specced(specs: &[BlockSpec]) -> (ProjectedDoc, Vec<Dot>) {
        let root = Dot::ROOT;
        let mut items: Vec<(Dot, SeqItem)> = Vec::new();
        let mut blocks: Vec<Dot> = Vec::new();
        let mut clock = 1u64;
        let mut next = |items: &mut Vec<(Dot, SeqItem)>, item: SeqItem| {
            let d = Dot::new(1, clock);
            clock += 1;
            items.push((d, item));
            d
        };
        for spec in specs {
            match spec {
                BlockSpec::Para(k) => {
                    let p = next(
                        &mut items,
                        SeqItem::Block {
                            node_type: NodeType::Paragraph,
                            parents: vec![root],
                        },
                    );
                    blocks.push(p);
                    for _ in 0..*k {
                        next(&mut items, SeqItem::Char('a'));
                    }
                }
                BlockSpec::Quote(inner) => {
                    let bq = next(
                        &mut items,
                        SeqItem::Block {
                            node_type: NodeType::Blockquote,
                            parents: vec![root],
                        },
                    );
                    blocks.push(bq);
                    for k in inner {
                        let p = next(
                            &mut items,
                            SeqItem::Block {
                                node_type: NodeType::Paragraph,
                                parents: vec![root, bq],
                            },
                        );
                        blocks.push(p);
                        for _ in 0..*k {
                            next(&mut items, SeqItem::Char('a'));
                        }
                    }
                }
            }
        }
        (project_document(&logs(&items)).unwrap(), blocks)
    }

    proptest::proptest! {
        /// Optimized `to_flat`/`from_flat` must agree with the reference DFS for every
        /// position in random flat *and* nested documents, and `from_flat ∘ to_flat`
        /// must round-trip.
        #[test]
        fn flat_conversions_match_reference(
            specs in proptest::collection::vec(
                proptest::prop_oneof![
                    (0usize..6).prop_map(BlockSpec::Para),
                    proptest::collection::vec(0usize..5, 1..4).prop_map(BlockSpec::Quote),
                ],
                1..6,
            ),
        ) {
            let (pd, blocks) = build_specced(&specs);
            let view = DocView::new(&pd);
            for &b in &blocks {
                let node = view.node(b).unwrap();
                for off in 0..=node.child_count() {
                    let pos = Position::new(b, off);
                    let Some(rp) = pos.resolve(&view) else { continue };
                    let got = rp.to_flat();
                    let expected = to_flat_ref(&view, b, off);
                    proptest::prop_assert_eq!(got, expected, "to_flat({:?},{}) ", b, off);
                    // Round-trip: from_flat lands on a position with the same flat offset.
                    let back = ResolvedPosition::from_flat(&view, got)
                        .expect("from_flat resolves");
                    proptest::prop_assert_eq!(back.to_flat(), got, "round-trip at flat {}", got);
                }
            }
        }
    }

    #[test]
    fn flat_size_root_not_wrapped() {
        let (pd, _p) = para_doc(&[SeqItem::Char('h'), SeqItem::Char('i')]);
        let view = DocView::new(&pd);
        assert_eq!(flat_size(&view), 4);
    }

    #[test]
    fn to_from_flat_roundtrip() {
        let (pd, p) = para_doc(&[SeqItem::Char('h'), SeqItem::Char('i')]);
        let view = DocView::new(&pd);
        for off in 0..=2usize {
            let pos = Position::new(p, off).resolve(&view).unwrap();
            let flat = pos.to_flat();
            let back = ResolvedPosition::from_flat(&view, flat).unwrap();
            assert_eq!(back.node(), p, "off {off}");
            assert_eq!(back.offset(), off, "off {off}");
        }
    }

    #[test]
    fn from_flat_close_token_maps_to_content_end() {
        let (pd, p) = para_doc(&[SeqItem::Char('h'), SeqItem::Char('i')]);
        let view = DocView::new(&pd);
        let r = ResolvedPosition::from_flat(&view, 3).unwrap();
        assert_eq!(r.node(), p);
        assert_eq!(r.offset(), 2);
    }

    #[test]
    fn from_flat_zero_is_root_start() {
        let (pd, _p) = para_doc(&[SeqItem::Char('h'), SeqItem::Char('i')]);
        let view = DocView::new(&pd);
        let root_id = view.root().unwrap().id();
        let r = ResolvedPosition::from_flat(&view, 0).unwrap();
        assert_eq!(r.node(), root_id);
        assert_eq!(r.offset(), 0);
    }

    fn mixed_doc() -> (ProjectedDoc, Dot, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let hr = Dot::new(1, 1);
        let para = Dot::new(1, 2);
        let a = Dot::new(1, 3);
        let hb = Dot::new(1, 4);
        let b = Dot::new(1, 5);
        let items = vec![
            (
                hr,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HorizontalRule {
                        variant: HorizontalRuleVariant::Line,
                    },
                    parents: vec![root],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (a, SeqItem::Char('a')),
            (hb, SeqItem::Atom(AtomLeaf::HardBreak)),
            (b, SeqItem::Char('b')),
        ];
        (project_document(&logs(&items)).unwrap(), hr, para, a, b)
    }

    #[test]
    fn flat_segments_mixed_doc() {
        let (pd, hr, para, a, b) = mixed_doc();
        let view = DocView::new(&pd);
        let hb = view
            .node(para)
            .unwrap()
            .children()
            .find_map(|c| match c {
                ChildView::Leaf(l) if l.as_char().is_none() => Some(l.dot()),
                _ => None,
            })
            .unwrap();
        let segs = flat_segments(&view);
        assert_eq!(
            segs,
            vec![
                FlatSegment::Atom { leaf: hr },
                FlatSegment::Open { block: para },
                FlatSegment::Text {
                    block: para,
                    leaves: vec![a]
                },
                FlatSegment::Break { leaf: hb },
                FlatSegment::Text {
                    block: para,
                    leaves: vec![b]
                },
                FlatSegment::Close { block: para },
            ]
        );
    }

    #[test]
    fn flat_text_and_size_mixed_doc() {
        let (pd, _hr, _para, _a, _b) = mixed_doc();
        let view = DocView::new(&pd);
        let size = flat_size(&view);
        assert_eq!(size, 6);
        assert_eq!(flat_text(&view, 0..size), "\u{fffc}\u{2028}a\nb\u{2029}");
    }

    #[test]
    fn flat_segments_in_range_splits_text() {
        let (pd, p) = para_doc(&[SeqItem::Char('a'), SeqItem::Char('b'), SeqItem::Char('c')]);
        let view = DocView::new(&pd);
        let leaves: Vec<Dot> = view
            .node(p)
            .unwrap()
            .children()
            .filter_map(|c| match c {
                ChildView::Leaf(l) => l.as_char().map(|_| l.dot()),
                _ => None,
            })
            .collect();
        let segs = flat_segments_in_range(&view, 2..3);
        assert_eq!(
            segs,
            vec![FlatSegment::Text {
                block: p,
                leaves: vec![leaves[1]]
            }]
        );
    }

    #[test]
    fn deep_container_atom() {
        use editor_model::ImageNode;
        let root = Dot::ROOT;
        let img = Dot::new(1, 1);
        let bq = Dot::new(1, 2);
        let para = Dot::new(1, 3);
        let x = Dot::new(1, 4);
        let items = vec![
            (
                img,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image {
                        node: ImageNode::default(),
                    },
                    parents: vec![root],
                },
            ),
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![root],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                },
            ),
            (x, SeqItem::Char('x')),
        ];
        let pd = project_document(&logs(&items)).unwrap();
        let view = DocView::new(&pd);
        let segs = flat_segments(&view);
        assert_eq!(segs.first(), Some(&FlatSegment::Atom { leaf: img }));
        assert!(
            segs.iter().any(|s| matches!(
                s,
                FlatSegment::Open { block } if *block == bq
            )),
            "blockquote opens as a container: {segs:?}"
        );
        let flat = flat_size(&view);
        assert_eq!(
            flat_text(&view, 0..flat),
            "\u{fffc}\u{2028}\u{2028}x\u{2029}\u{2029}\u{2028}\u{2029}"
        );
    }

    #[test]
    fn mixed_modifier_text_is_one_segment() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let b = Dot::new(1, 3);
        let c = Dot::new(1, 4);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (a, SeqItem::Char('a')),
            (b, SeqItem::Char('b')),
            (c, SeqItem::Char('c')),
        ];
        let spans = SpanLog::new()
            .apply(
                Dot::new(3, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: a,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: a,
                        bias: Bias::After,
                    },
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let pd = project_document(&logs_with_spans(&items, spans)).unwrap();
        let view = DocView::new(&pd);

        let segs = flat_segments(&view);
        assert_eq!(
            segs,
            vec![
                FlatSegment::Open { block: para },
                FlatSegment::Text {
                    block: para,
                    leaves: vec![a, b, c]
                },
                FlatSegment::Close { block: para },
            ]
        );

        assert_eq!(
            view.leaf_state_by_dot_slow(a)
                .unwrap()
                .eff
                .get(&ModifierType::Bold),
            Some(&Modifier::Bold)
        );
        assert_eq!(
            view.leaf_state_by_dot_slow(b)
                .unwrap()
                .eff
                .get(&ModifierType::Bold),
            None
        );
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn roundtrip_arbitrary(seq in arb_doc()) {
            let pd = project_document(&logs(&seq)).unwrap();
            let view = DocView::new(&pd);
            let size = flat_size(&view);

            prop_assert_eq!(flat_chars(&view, 0..size).len(), size);
            prop_assert_eq!(flat_segments(&view).iter().map(seg_size).sum::<usize>(), size);

            for flat in 0..=size {
                if let Some(r) = ResolvedPosition::from_flat(&view, flat) {
                    let back = r.to_flat();
                    let r2 = ResolvedPosition::from_flat(&view, back).unwrap();
                    prop_assert_eq!(r.node(), r2.node());
                    prop_assert_eq!(r.offset(), r2.offset());
                }
            }

            for block in all_block_ids(&view) {
                let count = view.node(block).unwrap().children().count();
                for offset in 0..=count {
                    let pos = Position::new(block, offset).resolve(&view).unwrap();
                    let flat = pos.to_flat();
                    let back = ResolvedPosition::from_flat(&view, flat).unwrap();
                    prop_assert_eq!(back.node(), pos.node());
                    prop_assert_eq!(back.offset(), pos.offset());
                }
            }
        }
    }

    fn seg_size(s: &FlatSegment) -> usize {
        match s {
            FlatSegment::Text { leaves, .. } => leaves.len(),
            _ => 1,
        }
    }

    fn all_block_ids(view: &DocView) -> Vec<Dot> {
        fn walk(node: &NodeView, out: &mut Vec<Dot>) {
            out.push(node.id());
            for c in node.children() {
                if let ChildView::Block(b) = c {
                    walk(&b, out);
                }
            }
        }
        let mut out = Vec::new();
        if let Some(root) = view.root() {
            walk(&root, &mut out);
        }
        out
    }

    fn arb_doc() -> impl Strategy<Value = Vec<(Dot, SeqItem)>> {
        (any::<bool>(), prop::collection::vec(0u8..4, 0..6)).prop_map(|(nest, kinds)| {
            let root = Dot::ROOT;
            let mut items: Vec<(Dot, SeqItem)> = vec![];
            let para = Dot::new(1, 1);
            if nest {
                let bq = Dot::new(1, 100);
                items.push((
                    bq,
                    SeqItem::Block {
                        node_type: NodeType::Blockquote,
                        parents: vec![root],
                    },
                ));
                items.push((
                    para,
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![root, bq],
                    },
                ));
            } else {
                items.push((
                    para,
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![root],
                    },
                ));
            }
            for (i, k) in kinds.into_iter().enumerate() {
                let d = Dot::new(1, 2 + i as u64);
                let item = match k {
                    0 => SeqItem::Char('a'),
                    1 => SeqItem::Char('b'),
                    2 => SeqItem::Atom(AtomLeaf::HardBreak),
                    _ => SeqItem::Char('c'),
                };
                items.push((d, item));
            }
            items
        })
    }
}
