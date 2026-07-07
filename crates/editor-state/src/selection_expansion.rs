use editor_common::StrExt;
use editor_model::{AtomLeaf, ChildView, DocView, NodeType, NodeView};
use editor_resource::Resource;

use crate::affinity::Affinity;

use crate::Position;
use crate::paragraph_break::paragraph_break_at_end;
use crate::selection::Selection;
use crate::traversal::{TextRun, text_run_around};

pub fn resolve_word_selection_expansion<'a>(
    sel: &Selection,
    view: &'a DocView<'a>,
    resource: &Resource,
) -> Option<Selection> {
    resolve_selection_expansion(sel, view, |pos, view| {
        word_selection_at(pos, view, resource)
    })
}

pub fn resolve_sentence_selection_expansion<'a>(
    sel: &Selection,
    view: &'a DocView<'a>,
    resource: &Resource,
) -> Option<Selection> {
    resolve_selection_expansion(sel, view, |pos, view| {
        sentence_selection_at(pos, view, resource)
    })
}

pub fn resolve_paragraph_selection_expansion<'a>(
    sel: &Selection,
    view: &'a DocView<'a>,
) -> Option<Selection> {
    resolve_selection_expansion(sel, view, paragraph_selection_at)
}

fn resolve_selection_expansion<'a>(
    sel: &Selection,
    view: &'a DocView<'a>,
    mut selection_at: impl FnMut(&Position, &'a DocView<'a>) -> Option<Selection>,
) -> Option<Selection> {
    if sel.is_collapsed() {
        return selection_at(&sel.head, view);
    }
    let rs = sel.resolve(view)?;
    let (from, to) = (rs.from().position(), rs.to().position());
    let to_lookup = range_end_lookup_position(&from, &to, view);
    let (a, b) = (selection_at(&from, view)?, selection_at(&to_lookup, view)?);
    (a == b).then_some(a)
}

fn is_inline_unit_atom(l: &editor_model::LeafView) -> bool {
    matches!(
        l.as_atom(),
        Some(AtomLeaf::HardBreak | AtomLeaf::Tab | AtomLeaf::PageBreak)
    )
}

fn atom_at(host: &NodeView, i: usize) -> bool {
    matches!(host.child_at(i), Some(ChildView::Leaf(l)) if is_inline_unit_atom(&l))
}

fn inline_unit_selection(host: Dot, index: usize) -> Selection {
    Selection::new(
        Position {
            node: host,
            offset: index,
            affinity: Affinity::Downstream,
        },
        Position {
            node: host,
            offset: index + 1,
            affinity: Affinity::Upstream,
        },
    )
}

use editor_crdt::Dot;

fn inline_selection_at<'a>(
    pos: &Position,
    view: &'a DocView<'a>,
    text_selection: impl FnOnce(&TextRun, usize) -> Option<Selection>,
) -> Option<Selection> {
    let host = view.node(pos.node)?;
    if !host.spec().is_textblock() {
        return None;
    }
    let run = text_run_around(pos, view)?;
    if !run.text.is_empty() {
        if pos.offset == run.end && atom_at(&host, run.end) {
            return Some(inline_unit_selection(host.id(), run.end));
        }
        return text_selection(&run, pos.offset - run.start);
    }
    let index = match pos.affinity {
        Affinity::Downstream => pos.offset,
        Affinity::Upstream => pos.offset.checked_sub(1)?,
    };
    atom_at(&host, index).then(|| inline_unit_selection(host.id(), index))
}

fn word_selection_at<'a>(
    pos: &Position,
    view: &'a DocView<'a>,
    resource: &Resource,
) -> Option<Selection> {
    inline_selection_at(pos, view, |run, target| {
        let (s, e) = word_range_in_text(&run.text, target, resource)?;
        (s < e).then(|| {
            Selection::new(
                Position::new(run.host, run.start + s),
                Position {
                    node: run.host,
                    offset: run.start + e,
                    affinity: Affinity::Upstream,
                },
            )
        })
    })
    .or_else(|| empty_textblock_selection_at(pos, view))
}

fn sentence_selection_at<'a>(
    pos: &Position,
    view: &'a DocView<'a>,
    resource: &Resource,
) -> Option<Selection> {
    inline_selection_at(pos, view, |run, target| {
        let (s, e) = sentence_range_in_text(&run.text, target, resource)?;
        (s < e).then(|| {
            Selection::new(
                Position::new(run.host, run.start + s),
                Position {
                    node: run.host,
                    offset: run.start + e,
                    affinity: Affinity::Upstream,
                },
            )
        })
    })
    .or_else(|| empty_textblock_selection_at(pos, view))
}

fn range_end_lookup_position(from: &Position, to: &Position, _view: &DocView) -> Position {
    if to.offset > 0 {
        Position {
            node: to.node,
            offset: to.offset - 1,
            affinity: Affinity::Downstream,
        }
    } else {
        *from
    }
}

fn paragraph_at<'a>(pos: &Position, view: &'a DocView<'a>) -> Option<NodeView<'a>> {
    view.node(pos.node)?
        .ancestors()
        .find(|n| n.node_type() == NodeType::Paragraph)
}

fn paragraph_selection<'a>(p: &NodeView, view: &'a DocView<'a>) -> Option<Selection> {
    let count = p.children().count();
    if count == 0 {
        return paragraph_break_at_end(
            &Position {
                node: p.id(),
                offset: 0,
                affinity: Affinity::Downstream,
            },
            view,
        );
    }
    Some(Selection::new(
        Position {
            node: p.id(),
            offset: 0,
            affinity: Affinity::Downstream,
        },
        Position {
            node: p.id(),
            offset: count,
            affinity: Affinity::Upstream,
        },
    ))
}

fn paragraph_selection_at<'a>(pos: &Position, view: &'a DocView<'a>) -> Option<Selection> {
    paragraph_selection(&paragraph_at(pos, view)?, view)
}

fn empty_textblock_selection_at<'a>(pos: &Position, view: &'a DocView<'a>) -> Option<Selection> {
    let node = view.node(pos.node)?;
    let tb = if node.spec().is_textblock() {
        node
    } else {
        node.ancestors().find(|n| n.spec().is_textblock())?
    };
    if tb.children().count() != 0 {
        return None;
    }
    paragraph_break_at_end(pos, view)
}

fn word_range_in_text(
    text: &str,
    char_offset: usize,
    resource: &Resource,
) -> Option<(usize, usize)> {
    if text.is_empty() {
        return None;
    }

    let char_count = text.char_count();
    let target = char_offset.min(char_count);
    let boundaries = text_boundaries(
        text,
        resource.segmenters.word.as_borrowed().segment_str(text),
    );
    let mut range = range_from_boundaries(&boundaries, target, char_count);

    if range.0 == range.1 && target > 0 {
        range = range_from_boundaries(&boundaries, target - 1, char_count);
    }

    if range.0 > 0
        && text
            .chars()
            .skip(range.0)
            .take(range.1 - range.0)
            .all(char::is_whitespace)
    {
        range = range_from_boundaries(&boundaries, range.0 - 1, char_count);
    }

    (range.0 < range.1).then_some(range)
}

fn sentence_range_in_text(
    text: &str,
    char_offset: usize,
    resource: &Resource,
) -> Option<(usize, usize)> {
    if text.is_empty() {
        return None;
    }

    let char_count = text.char_count();
    let target = char_offset.min(char_count);
    let boundaries = text_boundaries(
        text,
        resource.segmenters.sentence.as_borrowed().segment_str(text),
    );
    let (start, mut end) = {
        let range = range_from_boundaries(&boundaries, target, char_count);
        if range.0 == range.1 && target > 0 {
            range_from_boundaries(&boundaries, target - 1, char_count)
        } else {
            range
        }
    };

    let chars: Vec<_> = text.chars().collect();
    while end > start && chars.get(end - 1).is_some_and(|c| c.is_whitespace()) {
        end -= 1;
    }

    (start < end).then_some((start, end))
}

fn text_boundaries(text: &str, byte_boundaries: impl Iterator<Item = usize>) -> Vec<usize> {
    let mut boundaries = vec![0];
    for boundary in byte_boundaries {
        let char_boundary = text.nth_byte_char_offset(boundary);
        if boundaries.last().copied() != Some(char_boundary) {
            boundaries.push(char_boundary);
        }
    }
    let char_count = text.char_count();
    if boundaries.last().copied() != Some(char_count) {
        boundaries.push(char_count);
    }
    boundaries
}

fn range_from_boundaries(
    boundaries: &[usize],
    char_offset: usize,
    char_count: usize,
) -> (usize, usize) {
    let start = boundaries
        .iter()
        .rev()
        .find(|&&boundary| boundary <= char_offset)
        .copied()
        .unwrap_or(0);
    let end = boundaries
        .iter()
        .find(|&&boundary| boundary > char_offset)
        .copied()
        .unwrap_or(char_count);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, AtomLeaf, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeType, ProjectedDoc,
        SeqItem, SpanLog, project_document,
    };
    use editor_resource::Resource;

    use crate::{Affinity, Position};

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
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
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
            aliases: AliasLog::new(),
        }
    }

    fn pos(node: Dot, offset: usize) -> Position {
        Position::new(node, offset)
    }

    fn pos_aff(node: Dot, offset: usize, aff: Affinity) -> Position {
        Position {
            node,
            offset,
            affinity: aff,
        }
    }

    // root > p('hello world')
    fn hello_world_doc() -> (ProjectedDoc, Dot, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let text = "hello world";
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        for (i, c) in text.chars().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), SeqItem::Char(c)));
        }
        (project_document(&logs(&items)).unwrap(), root, para)
    }

    // root > p1('') p2('next')   (empty first para)
    fn empty_then_nonempty() -> (ProjectedDoc, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let p1 = Dot::new(2, 1);
        let p2 = Dot::new(2, 2);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(2, 3), SeqItem::Char('n')),
            (Dot::new(2, 4), SeqItem::Char('e')),
            (Dot::new(2, 5), SeqItem::Char('x')),
            (Dot::new(2, 6), SeqItem::Char('t')),
        ];
        (project_document(&logs(&items)).unwrap(), root, p1, p2)
    }

    // root > p('Hello.  Next.')
    fn sentence_doc() -> (ProjectedDoc, Dot, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(3, 1);
        let text = "Hello.  Next.";
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        for (i, c) in text.chars().enumerate() {
            items.push((Dot::new(3, 2 + i as u64), SeqItem::Char(c)));
        }
        (project_document(&logs(&items)).unwrap(), root, para)
    }

    // root > p('ab' + HardBreak + 'cd')
    fn ab_break_cd_doc() -> (ProjectedDoc, Dot, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(4, 1);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(4, 2), SeqItem::Char('a')),
            (Dot::new(4, 3), SeqItem::Char('b')),
            (Dot::new(4, 4), SeqItem::Atom(AtomLeaf::HardBreak)),
            (Dot::new(4, 5), SeqItem::Char('c')),
            (Dot::new(4, 6), SeqItem::Char('d')),
        ];
        (project_document(&logs(&items)).unwrap(), root, para)
    }

    // root > p(Tab + Tab)  (two inline atoms, no text)
    fn two_tabs_doc() -> (ProjectedDoc, Dot, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(5, 1);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(5, 2), SeqItem::Atom(AtomLeaf::Tab)),
            (Dot::new(5, 3), SeqItem::Atom(AtomLeaf::Tab)),
        ];
        (project_document(&logs(&items)).unwrap(), root, para)
    }

    // root > p('hello' + Tab)  text-run END before Tab
    fn hello_tab_doc() -> (ProjectedDoc, Dot, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(6, 1);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(6, 2), SeqItem::Char('h')),
            (Dot::new(6, 3), SeqItem::Char('e')),
            (Dot::new(6, 4), SeqItem::Char('l')),
            (Dot::new(6, 5), SeqItem::Char('l')),
            (Dot::new(6, 6), SeqItem::Char('o')),
            (Dot::new(6, 7), SeqItem::Atom(AtomLeaf::Tab)),
        ];
        (project_document(&logs(&items)).unwrap(), root, para)
    }

    // root > p('hello' + HardBreak)  text-run END before HardBreak
    fn hello_hardbreak_doc() -> (ProjectedDoc, Dot, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(7, 1);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(7, 2), SeqItem::Char('h')),
            (Dot::new(7, 3), SeqItem::Char('e')),
            (Dot::new(7, 4), SeqItem::Char('l')),
            (Dot::new(7, 5), SeqItem::Char('l')),
            (Dot::new(7, 6), SeqItem::Char('o')),
            (Dot::new(7, 7), SeqItem::Atom(AtomLeaf::HardBreak)),
        ];
        (project_document(&logs(&items)).unwrap(), root, para)
    }

    // root > p('Hello') + p('World')  non-empty paragraph
    fn two_nonempty_paras() -> (ProjectedDoc, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let p1 = Dot::new(8, 1);
        let p2 = Dot::new(8, 7);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(8, 2), SeqItem::Char('H')),
            (Dot::new(8, 3), SeqItem::Char('e')),
            (Dot::new(8, 4), SeqItem::Char('l')),
            (Dot::new(8, 5), SeqItem::Char('l')),
            (Dot::new(8, 6), SeqItem::Char('o')),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(8, 8), SeqItem::Char('W')),
            (Dot::new(8, 9), SeqItem::Char('o')),
            (Dot::new(8, 10), SeqItem::Char('r')),
            (Dot::new(8, 11), SeqItem::Char('l')),
            (Dot::new(8, 12), SeqItem::Char('d')),
        ];
        (project_document(&logs(&items)).unwrap(), root, p1, p2)
    }

    // §4.1 word — collapsed caret inside word
    #[test]
    fn test_1_word_collapsed_inside_word() {
        let resource = Resource::new_test();
        let (pd, _root, para) = hello_world_doc();
        let view = DocView::new(&pd);

        // caret at offset 2 (inside 'hello')
        let sel = Selection::collapsed(pos(para, 2));
        let result = resolve_word_selection_expansion(&sel, &view, &resource);

        assert_eq!(
            result,
            Some(Selection::new(
                pos(para, 0),
                pos_aff(para, 5, Affinity::Upstream),
            ))
        );
    }

    // §4.1 word — collapsed caret inside second word
    #[test]
    fn test_1_word_collapsed_inside_second_word() {
        let resource = Resource::new_test();
        let (pd, _root, para) = hello_world_doc();
        let view = DocView::new(&pd);

        // caret at offset 8 (inside 'world')
        let sel = Selection::collapsed(pos(para, 8));
        let result = resolve_word_selection_expansion(&sel, &view, &resource);

        assert_eq!(
            result,
            Some(Selection::new(
                pos(para, 6),
                pos_aff(para, 11, Affinity::Upstream),
            ))
        );
    }

    // §4.2 word — range within same word expands
    #[test]
    fn test_2_word_range_same_word_expands() {
        let resource = Resource::new_test();
        let (pd, _root, para) = hello_world_doc();
        let view = DocView::new(&pd);

        // range inside 'hello' (offsets 1..3)
        let sel = Selection::new(pos(para, 1), pos(para, 3));
        let result = resolve_word_selection_expansion(&sel, &view, &resource);

        assert_eq!(
            result,
            Some(Selection::new(
                pos(para, 0),
                pos_aff(para, 5, Affinity::Upstream),
            ))
        );
    }

    // §4.2 word — range spanning two words → None
    #[test]
    fn test_2_word_range_cross_word_none() {
        let resource = Resource::new_test();
        let (pd, _root, para) = hello_world_doc();
        let view = DocView::new(&pd);

        // range from inside 'hello' to inside 'world'
        let sel = Selection::new(pos(para, 1), pos(para, 8));
        let result = resolve_word_selection_expansion(&sel, &view, &resource);

        assert_eq!(result, None);
    }

    // §4.3 sentence — caret inside a sentence
    #[test]
    fn test_3_sentence_collapsed() {
        let resource = Resource::new_test();
        let (pd, _root, para) = sentence_doc();
        let view = DocView::new(&pd);

        // caret at offset 1 (inside 'Hello.')
        let sel = Selection::collapsed(pos(para, 1));
        let result = resolve_sentence_selection_expansion(&sel, &view, &resource);

        // 'Hello.' with trailing space trimmed → 0..6
        assert_eq!(
            result,
            Some(Selection::new(
                pos(para, 0),
                pos_aff(para, 6, Affinity::Upstream),
            ))
        );
    }

    // §4.4 inline unit — caret at text-run END before HardBreak, Downstream → unit-selects atom
    #[test]
    fn test_4_inline_unit_text_end_before_hardbreak_downstream() {
        let resource = Resource::new_test();
        let (pd, _root, para) = hello_hardbreak_doc();
        let view = DocView::new(&pd);

        // offset 5 = end of 'hello', Downstream affinity
        let sel = Selection::collapsed(pos_aff(para, 5, Affinity::Downstream));
        let result = resolve_word_selection_expansion(&sel, &view, &resource);

        // Must unit-select the HardBreak at index 5: (para,5,Down)..(para,6,Up)
        assert_eq!(
            result,
            Some(Selection::new(
                pos_aff(para, 5, Affinity::Downstream),
                pos_aff(para, 6, Affinity::Upstream),
            ))
        );
    }

    // §4.4 inline unit — caret at text-run END before HardBreak, Upstream → ALSO unit-selects atom (affinity-independent)
    #[test]
    fn test_4_inline_unit_text_end_before_hardbreak_upstream() {
        let resource = Resource::new_test();
        let (pd, _root, para) = hello_hardbreak_doc();
        let view = DocView::new(&pd);

        // offset 5 = end of 'hello', Upstream affinity
        let sel = Selection::collapsed(pos_aff(para, 5, Affinity::Upstream));
        let result = resolve_word_selection_expansion(&sel, &view, &resource);

        // Must ALSO unit-select the HardBreak (not select the preceding word)
        assert_eq!(
            result,
            Some(Selection::new(
                pos_aff(para, 5, Affinity::Downstream),
                pos_aff(para, 6, Affinity::Upstream),
            ))
        );
    }

    // §4.4 inline unit — caret at text-run END before Tab, both affinities
    #[test]
    fn test_4_inline_unit_text_end_before_tab_downstream() {
        let resource = Resource::new_test();
        let (pd, _root, para) = hello_tab_doc();
        let view = DocView::new(&pd);

        let sel = Selection::collapsed(pos_aff(para, 5, Affinity::Downstream));
        let result = resolve_word_selection_expansion(&sel, &view, &resource);

        assert_eq!(
            result,
            Some(Selection::new(
                pos_aff(para, 5, Affinity::Downstream),
                pos_aff(para, 6, Affinity::Upstream),
            ))
        );
    }

    #[test]
    fn test_4_inline_unit_text_end_before_tab_upstream() {
        let resource = Resource::new_test();
        let (pd, _root, para) = hello_tab_doc();
        let view = DocView::new(&pd);

        let sel = Selection::collapsed(pos_aff(para, 5, Affinity::Upstream));
        let result = resolve_word_selection_expansion(&sel, &view, &resource);

        assert_eq!(
            result,
            Some(Selection::new(
                pos_aff(para, 5, Affinity::Downstream),
                pos_aff(para, 6, Affinity::Upstream),
            ))
        );
    }

    // §4.4 inline unit — caret between two atoms (empty run) → affinity-selected atom
    #[test]
    fn test_4_inline_unit_between_atoms_downstream() {
        let resource = Resource::new_test();
        let (pd, _root, para) = two_tabs_doc();
        let view = DocView::new(&pd);

        // offset 1, Downstream → selects atom at index 1 (second Tab)
        let sel = Selection::collapsed(pos_aff(para, 1, Affinity::Downstream));
        let result = resolve_word_selection_expansion(&sel, &view, &resource);

        assert_eq!(
            result,
            Some(Selection::new(
                pos_aff(para, 1, Affinity::Downstream),
                pos_aff(para, 2, Affinity::Upstream),
            ))
        );
    }

    #[test]
    fn test_4_inline_unit_between_atoms_upstream() {
        let resource = Resource::new_test();
        let (pd, _root, para) = two_tabs_doc();
        let view = DocView::new(&pd);

        // offset 1, Upstream → selects atom at index 0 (first Tab)
        let sel = Selection::collapsed(pos_aff(para, 1, Affinity::Upstream));
        let result = resolve_word_selection_expansion(&sel, &view, &resource);

        assert_eq!(
            result,
            Some(Selection::new(
                pos_aff(para, 0, Affinity::Downstream),
                pos_aff(para, 1, Affinity::Upstream),
            ))
        );
    }

    // §4.4 caret at text-run START after an atom → segments the following word (no atom)
    #[test]
    fn test_4_text_run_start_after_atom_segments_word() {
        let resource = Resource::new_test();
        let (pd, _root, para) = ab_break_cd_doc();
        let view = DocView::new(&pd);

        // offset 3 = start of 'cd' (after HardBreak at index 2)
        let sel = Selection::collapsed(pos(para, 3));
        let result = resolve_word_selection_expansion(&sel, &view, &resource);

        // Should select 'cd' (word), not the HardBreak
        assert_eq!(
            result,
            Some(Selection::new(
                pos(para, 3),
                pos_aff(para, 5, Affinity::Upstream),
            ))
        );
    }

    // §4.5 paragraph — caret in non-empty paragraph → full (0..count)
    #[test]
    fn test_5_paragraph_nonempty() {
        let (pd, _root, p1, _p2) = two_nonempty_paras();
        let view = DocView::new(&pd);

        let sel = Selection::collapsed(pos(p1, 2));
        let result = resolve_paragraph_selection_expansion(&sel, &view);

        // p1 has 5 chars
        assert_eq!(
            result,
            Some(Selection::new(
                pos_aff(p1, 0, Affinity::Downstream),
                pos_aff(p1, 5, Affinity::Upstream),
            ))
        );
    }

    // §4.5 paragraph — empty paragraph → trailing paragraph-break
    #[test]
    fn test_5_paragraph_empty_gives_break() {
        let (pd, _root, p1, p2) = empty_then_nonempty();
        let view = DocView::new(&pd);

        let sel = Selection::collapsed(pos(p1, 0));
        let result = resolve_paragraph_selection_expansion(&sel, &view);

        // empty p1 → paragraph_break_at_end
        // Break: p1 end .. p2 start = (p1,0,Down)..(p2,0,Up) based on paragraph_break.rs logic
        assert!(
            result.is_some(),
            "empty paragraph should give trailing break"
        );
        let sel_res = result.unwrap();
        assert_eq!(sel_res.anchor.node, p1);
        let _ = p2;
    }

    // §4.6 empty textblock — caret in empty paragraph → trailing break
    #[test]
    fn test_6_empty_textblock_selection_at() {
        let resource = Resource::new_test();
        let (pd, _root, p1, _p2) = empty_then_nonempty();
        let view = DocView::new(&pd);

        let sel = Selection::collapsed(pos(p1, 0));
        let result = resolve_word_selection_expansion(&sel, &view, &resource);

        assert!(
            result.is_some(),
            "empty textblock should give trailing break via word expansion"
        );
    }

    // §4.6 non-empty textblock → None for empty_textblock_selection_at
    #[test]
    fn test_6_nonempty_textblock_empty_textblock_at_returns_none() {
        let (pd, _root, p1, _p2) = two_nonempty_paras();
        let view = DocView::new(&pd);

        // non-empty paragraph: empty_textblock_selection_at should return None
        let result = empty_textblock_selection_at(&pos(p1, 2), &view);
        assert_eq!(result, None);
    }

    // §4.7 range_end_lookup — to.offset > 0 → offset - 1
    #[test]
    fn test_7_range_end_lookup_nonzero_offset() {
        let (pd, _root, p1, _p2) = two_nonempty_paras();
        let view = DocView::new(&pd);

        let from = pos(p1, 0);
        let to = pos(p1, 3);
        let result = range_end_lookup_position(&from, &to, &view);

        assert_eq!(result.node, p1);
        assert_eq!(result.offset, 2);
        assert_eq!(result.affinity, Affinity::Downstream);
    }

    // §4.7 range_end_lookup — to.offset == 0 → from
    #[test]
    fn test_7_range_end_lookup_zero_offset() {
        let (pd, _root, p1, p2) = two_nonempty_paras();
        let view = DocView::new(&pd);

        let from = pos(p1, 2);
        let to = pos(p2, 0);
        let result = range_end_lookup_position(&from, &to, &view);

        assert_eq!(result, from);
    }

    proptest::proptest! {
        #[test]
        fn test_9_proptest_word_expansion_never_panics_and_invariants(
            offset in 0usize..=11,
            affinity_bit in 0u8..=1,
        ) {
            let resource = Resource::new_test();
            let (pd, _root, para) = hello_world_doc();
            let view = DocView::new(&pd);

            let aff = if affinity_bit == 0 { Affinity::Downstream } else { Affinity::Upstream };
            let offset = offset.min(11);
            let sel = Selection::collapsed(Position { node: para, offset, affinity: aff });
            let result = resolve_word_selection_expansion(&sel, &view, &resource);

            if let Some(s) = result {
                let rs = s.resolve(&view);
                proptest::prop_assert!(rs.is_some(), "result selection must be resolvable");
                if let Some(rs) = rs {
                    proptest::prop_assert!(rs.from() <= rs.to(), "from <= to");
                    proptest::prop_assert_eq!(rs.from().node(), rs.to().node(), "same host for word expansion");
                }
            }
        }

        #[test]
        fn test_9_proptest_sentence_expansion_never_panics(
            offset in 0usize..=13,
            affinity_bit in 0u8..=1,
        ) {
            let resource = Resource::new_test();
            let (pd, _root, para) = sentence_doc();
            let view = DocView::new(&pd);

            let aff = if affinity_bit == 0 { Affinity::Downstream } else { Affinity::Upstream };
            let offset = offset.min(13);
            let sel = Selection::collapsed(Position { node: para, offset, affinity: aff });
            let _ = resolve_sentence_selection_expansion(&sel, &view, &resource);
        }
    }
}
