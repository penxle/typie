use editor_clipboard::Slice;
use editor_crdt::Dot;
use editor_model::{Fragment, NodeType, PlainNode, PlainParagraphNode};
use editor_state::Position;
use editor_transaction::Transaction;

use super::{fragments_are_inline, position_in_textblock, top_level_fragments};
use crate::CommandResult;
use crate::helpers::{find_ancestor_textblock, insert_terminal_page_break_into_root_paragraph};

pub(super) fn prepare_page_breaks_for_position(
    tr: &Transaction,
    position: &Position,
    slice: Slice,
) -> (Slice, Option<Fragment>) {
    if !contains_page_break(&slice.content) {
        return (slice, None);
    }

    let in_textblock = position_in_textblock(tr, position);
    let root_textblock = in_textblock && position_is_in_root_paragraph(tr, position);
    let root_boundary = !in_textblock && position.node == Dot::ROOT;
    let inserts_root_paragraph = root_textblock || root_boundary;
    let top_level_inline = fragments_are_inline(&top_level_fragments(&slice));

    let sanitized = if top_level_inline {
        sanitize_page_break_siblings(slice.content, inserts_root_paragraph)
    } else {
        sanitize_structural_page_break_roots(slice.content, inserts_root_paragraph)
    };
    let content = sanitized.fragments;

    if content.is_empty() {
        return (Slice::new(vec![], 0, 0), None);
    }

    let terminal_bare_page_break =
        top_level_inline && content.last().is_some_and(is_page_break_fragment);
    if root_textblock && terminal_bare_page_break {
        return (
            Slice::new(
                vec![paragraph_with_children(content), empty_paragraph_fragment()],
                1,
                1,
            ),
            None,
        );
    }

    let mut prepared = if root_boundary && top_level_inline {
        Slice::new(vec![paragraph_with_children(content)], 0, 0)
    } else {
        Slice::new(
            content,
            slice.open_start.min(sanitized.open_start),
            slice.open_end.min(sanitized.open_end),
        )
    };

    if root_textblock
        && prepared.open_end > 0
        && prepared
            .content
            .last()
            .is_some_and(paragraph_ends_with_page_break)
    {
        prepared.content.push(empty_paragraph_fragment());
        prepared.open_end = 1;
    }

    let trailing_block_context = if root_boundary
        && root_boundary_is_at_end(tr, position)
        && prepared
            .content
            .last()
            .is_some_and(paragraph_ends_with_page_break)
    {
        Some(empty_paragraph_fragment())
    } else {
        None
    };

    (prepared, trailing_block_context)
}

pub(super) fn insert_terminal_page_break_from_edge(
    tr: &mut Transaction,
    paragraph_id: Dot,
    fragments: &[Fragment],
) -> CommandResult {
    if !fragments.last().is_some_and(is_page_break_fragment) {
        return Ok(false);
    }
    insert_terminal_page_break_into_root_paragraph(tr, paragraph_id)
}

struct SanitizedPageBreaks {
    fragments: Vec<Fragment>,
    open_start: u32,
    open_end: u32,
}

struct SanitizedPageBreakFragment {
    fragment: Fragment,
    open_start: u32,
    open_end: u32,
}

fn sanitize_page_break_fragment(
    mut fragment: Fragment,
    root_paragraph: bool,
) -> Option<SanitizedPageBreakFragment> {
    let allow_terminal_page_break =
        root_paragraph && fragment.node.as_type() == NodeType::Paragraph;
    let had_children = !fragment.children.is_empty();
    let sanitized = sanitize_page_break_siblings(fragment.children, allow_terminal_page_break);
    fragment.children = sanitized.fragments;
    (!had_children || !fragment.children.is_empty()).then_some(SanitizedPageBreakFragment {
        fragment,
        open_start: sanitized.open_start.saturating_add(1),
        open_end: sanitized.open_end.saturating_add(1),
    })
}

fn sanitize_page_break_siblings(
    fragments: Vec<Fragment>,
    allow_terminal_page_break: bool,
) -> SanitizedPageBreaks {
    let original_len = fragments.len();
    let terminal_page_break = fragments
        .len()
        .checked_sub(1)
        .filter(|&index| allow_terminal_page_break && is_page_break_fragment(&fragments[index]));

    let retained: Vec<_> = fragments
        .into_iter()
        .enumerate()
        .filter_map(|(index, fragment)| {
            if is_page_break_fragment(&fragment) {
                return (Some(index) == terminal_page_break).then_some((
                    index,
                    SanitizedPageBreakFragment {
                        fragment,
                        open_start: 1,
                        open_end: 1,
                    },
                ));
            }
            sanitize_page_break_fragment(fragment, false).map(|fragment| (index, fragment))
        })
        .collect();
    finish_page_break_sanitization(retained, original_len)
}

fn sanitize_structural_page_break_roots(
    fragments: Vec<Fragment>,
    allow_root_paragraph: bool,
) -> SanitizedPageBreaks {
    let original_len = fragments.len();
    let retained = fragments
        .into_iter()
        .enumerate()
        .filter_map(|(index, fragment)| {
            if is_page_break_fragment(&fragment) {
                return None;
            }
            let root_paragraph =
                allow_root_paragraph && fragment.node.as_type() == NodeType::Paragraph;
            sanitize_page_break_fragment(fragment, root_paragraph).map(|fragment| (index, fragment))
        })
        .collect();
    finish_page_break_sanitization(retained, original_len)
}

fn finish_page_break_sanitization(
    retained: Vec<(usize, SanitizedPageBreakFragment)>,
    original_len: usize,
) -> SanitizedPageBreaks {
    let open_start = retained
        .first()
        .filter(|(index, _)| *index == 0)
        .map_or(0, |(_, fragment)| fragment.open_start);
    let open_end = retained
        .last()
        .filter(|(index, _)| *index + 1 == original_len)
        .map_or(0, |(_, fragment)| fragment.open_end);
    SanitizedPageBreaks {
        fragments: retained
            .into_iter()
            .map(|(_, fragment)| fragment.fragment)
            .collect(),
        open_start,
        open_end,
    }
}

fn is_page_break_fragment(fragment: &Fragment) -> bool {
    fragment.node.as_type() == NodeType::PageBreak
}

fn contains_page_break(fragments: &[Fragment]) -> bool {
    fragments
        .iter()
        .any(|fragment| is_page_break_fragment(fragment) || contains_page_break(&fragment.children))
}

fn paragraph_ends_with_page_break(fragment: &Fragment) -> bool {
    fragment.node.as_type() == NodeType::Paragraph
        && fragment.children.last().is_some_and(is_page_break_fragment)
}

fn paragraph_with_children(children: Vec<Fragment>) -> Fragment {
    Fragment {
        node: PlainNode::Paragraph(PlainParagraphNode::default()),
        modifiers: vec![],
        carry: vec![],
        children,
    }
}

fn empty_paragraph_fragment() -> Fragment {
    paragraph_with_children(vec![])
}

fn root_boundary_is_at_end(tr: &Transaction, position: &Position) -> bool {
    tr.state()
        .view()
        .root()
        .is_some_and(|root| position.offset >= root.children().count())
}

fn position_is_in_root_paragraph(tr: &Transaction, position: &Position) -> bool {
    let view = tr.state().view();
    find_ancestor_textblock(&view, position.node).is_some_and(|id| {
        view.node(id).is_some_and(|node| {
            node.node_type() == NodeType::Paragraph
                && node.parent().is_some_and(|parent| parent.id() == Dot::ROOT)
        })
    })
}
