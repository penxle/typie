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

/// Walk-recursive flat width oracle — superseded by [`NodeView::flat_width`]'s
/// `O(1)` maintained index. Kept only as the O3 consistency oracle for the two
/// production subtree-skip sites (`flat_chars`/`flat_segments_in_range`, which
/// call `flat_width` instead) and for probes/tests.
#[cfg(any(test, feature = "test-utils"))]
fn block_flat_size(b: &NodeView) -> usize {
    let total = b.child_count();
    let leaves = b.leaf_child_count();
    if leaves == total {
        return 2 + total;
    }
    2 + b.children().map(|c| child_flat_size(&c)).sum::<usize>()
}

#[cfg(any(test, feature = "test-utils"))]
fn child_flat_size(c: &ChildView) -> usize {
    match c {
        ChildView::Leaf(_) => 1,
        ChildView::Block(b) => block_flat_size(b),
    }
}

/// The document's total flat width. Reads the maintained flat index — `O(1)`.
pub fn flat_size(view: &DocView) -> usize {
    view.root_flat_total() as usize
}

/// Walk-recursive `flat_size` oracle, independent of the maintained flat index
/// — an O3 total-length cross-check (`flat_chars(0..n)` cannot serve this role:
/// it truncates at `range.end`, so it can't detect an inflated total).
#[cfg(any(test, feature = "test-utils"))]
pub fn flat_size_walk_probe(view: &DocView) -> usize {
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
        let Some(mut container) = view.root() else {
            return 0;
        };
        let path = self.path();
        let mut acc = 0u64;
        for (i, &slot) in path.iter().enumerate() {
            acc += container.flat_offset_before(slot);
            if i + 1 == path.len() {
                break;
            }
            let Some(ChildView::Block(b)) = container.child_at(slot) else {
                return acc as usize;
            };
            acc += 1;
            container = b;
        }
        acc as usize
    }

    fn from_flat(view: &'a DocView<'a>, flat: usize) -> Option<Self> {
        from_flat_index(view, flat)?.resolve(view)
    }
}

/// `from_flat`'s index descent, open/close-equivalent to the walk oracle:
/// `within == 0` is the child's open boundary; `within == width - 1` descends
/// to `target == content_total`, the child's own end.
fn from_flat_index(view: &DocView, flat: usize) -> Option<Position> {
    let mut container = view.root()?;
    let mut target = flat as u64;
    loop {
        if target == container.flat_content_total() {
            return Some(Position::new(container.id(), container.child_count()));
        }
        let (slot, within) = container.child_at_flat_offset(target)?;
        match container.child_at(slot)? {
            ChildView::Leaf(_) => return Some(Position::new(container.id(), slot)),
            ChildView::Block(b) => {
                if within == 0 {
                    return Some(Position::new(container.id(), slot));
                }
                container = b;
                target = within - 1;
            }
        }
    }
}

/// Walk-recursive `to_flat` oracle, independent of the maintained flat index —
/// the O3 index-vs-walk cross-check. Superseded in production by
/// [`ResolvedPositionFlatExt::to_flat`]'s path-prefix summation.
#[cfg(any(test, feature = "test-utils"))]
pub fn to_flat_walk_probe(view: &DocView, target: Dot, target_offset: usize) -> Option<usize> {
    let root = view.root()?;
    let mut ancestors: Vec<Dot> = Vec::new();
    let mut cur = view.node(target);
    while let Some(n) = cur {
        ancestors.push(n.id());
        cur = n.parent();
    }
    to_flat_walk(&root, target, target_offset, &ancestors)
}

#[cfg(any(test, feature = "test-utils"))]
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

#[cfg(any(test, feature = "test-utils"))]
pub fn from_flat_walk_probe(view: &DocView, flat: usize) -> Option<Position> {
    let root = view.root()?;
    from_flat_walk(&root, 0, flat)
}

#[cfg(any(test, feature = "test-utils"))]
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
                let size = b.flat_width() as usize;
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
                        let size = b.flat_width() as usize;
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
        AliasLog, Anchor, AtomLeaf, Bias, DocLogs, HorizontalRuleVariant, Modifier,
        ModifierAttrLog, ModifierType, NodeAttrLog, NodeType, ProjectedDoc, SeqItem, SpanLog,
        SpanOp, project_document,
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
            aliases: AliasLog::new(),
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
                attrs: vec![],
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
                            attrs: vec![],
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
                            attrs: vec![],
                        },
                    );
                    blocks.push(bq);
                    for k in inner {
                        let p = next(
                            &mut items,
                            SeqItem::Block {
                                node_type: NodeType::Paragraph,
                                parents: vec![root, bq],
                                attrs: vec![],
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

    #[test]
    fn offset_zero_flat_is_one_for_root_level_paragraph() {
        let (pd, p) = para_doc(&[SeqItem::Char('a')]);
        let view = DocView::new(&pd);
        let rp = Position::new(p, 0).resolve(&view).unwrap();
        assert_eq!(
            rp.to_flat(),
            1,
            "a root-direct paragraph's own open sentinel is the sole flat unit before offset 0"
        );
    }

    #[test]
    fn offset_zero_flat_equals_nesting_depth() {
        let (pd, blocks) = build_specced(&[BlockSpec::Quote(vec![1])]);
        let view = DocView::new(&pd);
        let para = blocks[1];
        let rp = Position::new(para, 0).resolve(&view).unwrap();
        assert_eq!(
            rp.to_flat(),
            2,
            "two open sentinels (blockquote, paragraph) precede the first leaf two levels deep"
        );
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
                    attrs: vec![],
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
                    attrs: vec![],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                    attrs: vec![],
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
                    attrs: vec![],
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
                        attrs: vec![],
                    },
                ));
                items.push((
                    para,
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![root, bq],
                        attrs: vec![],
                    },
                ));
            } else {
                items.push((
                    para,
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![root],
                        attrs: vec![],
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

    fn assert_flat_contract(state: &crate::State) {
        let view = state.view();
        let n = flat_size(&view);
        assert_eq!(
            flat_chars(&view, 0..n).len(),
            n,
            "independent walk (flat_chars) must agree with flat_size"
        );
        for f in 0..=n {
            let Some(rp) = ResolvedPosition::from_flat(&view, f) else {
                panic!("from_flat must resolve every 0..=flat_size, failed at {f}/{n}");
            };
            assert_eq!(rp.to_flat(), f, "to_flat(from_flat({f})) roundtrip");
        }
    }

    /// `from_flat`/`to_flat`/`flat_size` at a single flat offset must agree
    /// between the maintained index and the independent walk oracle.
    fn assert_flat_index_matches_walk_at(view: &DocView, f: usize) {
        let via_index = ResolvedPosition::from_flat(view, f).map(|r| r.position());
        let via_walk = from_flat_walk_probe(view, f);
        assert_eq!(via_index, via_walk, "from_flat index vs walk at {f}");
        if let Some(rp) = ResolvedPosition::from_flat(view, f) {
            let back = to_flat_walk_probe(view, rp.node(), rp.offset());
            assert_eq!(Some(rp.to_flat()), back, "to_flat index vs walk at {f}");
        }
    }

    /// Full-sweep index-vs-walk cross-check, `0..=flat_size` inclusive.
    fn assert_flat_index_matches_walk(state: &crate::State) {
        let view = state.view();
        let n = flat_size(&view);
        assert_eq!(n, flat_size_walk_probe(&view), "flat_size index vs walk");
        for f in 0..=n {
            assert_flat_index_matches_walk_at(&view, f);
        }
    }

    #[test]
    fn flat_contract_holds_on_named_scenarios() {
        for ps in [
            crate::corpus::bold_label_fold_list(),
            crate::corpus::tombstone_cluster_anchors(),
            crate::corpus::concurrent_delete_remote_span(),
            crate::corpus::mixed_atoms(),
        ] {
            let state = crate::State::new(ps, None);
            assert_flat_contract(&state);
            assert_flat_index_matches_walk(&state);
        }
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 64, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn flat_contract_holds_on_corpus_final_states(
            suffix in proptest::collection::vec(proptest::prelude::any::<crate::corpus::CorpusStep>(), 0..40),
        ) {
            let mut steps = crate::corpus::mandatory_prefix();
            steps.extend(suffix);
            let run = crate::corpus::run_corpus(&steps, &mut |_, _, _| {});
            for ps in [run.a, run.b] {
                let state = crate::State::new(ps, None);
                assert_flat_contract(&state);
                assert_flat_index_matches_walk(&state);
            }
        }
    }

    /// Up to 8 boundary-sample flat offsets: the two endpoints, evenly-strided
    /// interior points, and (when the replica's most recent span resolves to a
    /// live leaf) the offset adjacent to that mutation.
    fn boundary_flat_offsets(n: usize, mutation_adjacent: Option<usize>) -> Vec<usize> {
        let mut offsets: Vec<usize> = vec![0, n];
        if n > 0 {
            for k in 1..=4usize {
                offsets.push(n * k / 5);
            }
        }
        if let Some(m) = mutation_adjacent {
            offsets.push(m.min(n));
        }
        offsets.sort_unstable();
        offsets.dedup();
        offsets.truncate(8);
        offsets
    }

    /// The flat offset adjacent to the replica's most recently applied span, if
    /// its start anchor still resolves to a live leaf under a live parent.
    fn mutation_adjacent_flat(view: &DocView, spans: &[(Anchor, Anchor, Dot)]) -> Option<usize> {
        let (start, ..) = spans.last()?;
        let leaf = view.leaf(start.id)?;
        let parent = leaf.parent()?;
        let slot = parent
            .children()
            .position(|c| matches!(&c, ChildView::Leaf(l) if l.dot() == start.id))?;
        let offset = slot + usize::from(matches!(start.bias, Bias::After));
        Position::new(parent.id(), offset)
            .resolve(view)
            .map(|rp| rp.to_flat())
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig {
            cases: std::env::var("PROPTEST_CASES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(64),
            ..proptest::prelude::ProptestConfig::default()
        })]
        #[test]
        fn flat_index_consistent_at_corpus_boundaries(
            suffix in proptest::collection::vec(proptest::prelude::any::<crate::corpus::CorpusStep>(), 0..40),
        ) {
            let mut steps = crate::corpus::mandatory_prefix();
            steps.extend(suffix);
            crate::corpus::run_corpus(&steps, &mut |a, b, spans| {
                for (s, replica_spans) in [(a, &spans.a), (b, &spans.b)] {
                    let tree = &s.projected().tree;
                    editor_model::assert_flat_index_consistent(tree);
                    let view = s.view();
                    let indexed = flat_size(&view) as u64;
                    let walked = flat_size_walk_probe(&view) as u64;
                    assert_eq!(indexed, walked, "root flat total: index vs walk");
                    let adjacent = mutation_adjacent_flat(&view, replica_spans);
                    for f in boundary_flat_offsets(indexed as usize, adjacent) {
                        assert_flat_index_matches_walk_at(&view, f);
                    }
                }
            });
        }
    }
}
