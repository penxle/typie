use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{
    ChildView, DocView, Fragment, LeafView, Modifier, ModifierType, NodeType, NodeView,
    OwnModifier, PlainNode, PlainTextNode,
};
use editor_resource::Resource;
use editor_state::State;
use editor_state::{CellRect, ResolvedSelection, document_content_selection};
use serde::{Deserialize, Serialize};

use crate::html::parse as html_parse;
use crate::html::serialize as html_serialize;
use crate::payload::ClipboardPayload;
use crate::text::parse as text_parse;
use crate::text::serialize as text_serialize;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slice {
    pub content: Vec<Fragment>,
    pub open_start: u32,
    pub open_end: u32,
}

impl Slice {
    pub fn extract(state: &State) -> Option<Slice> {
        let view = state.view();
        let rs = state.selection.as_ref()?.resolve(&view)?;
        if rs.is_collapsed() {
            return None;
        }
        if let Some(rect) = rs.as_cell_rect() {
            return Some(extract_cell_rect(state, &view, &rect));
        }
        let full_document = covers_document_content(&view, &rs);
        let common_nv = if full_document {
            view.root()?
        } else {
            view.node(common_ancestor(&view, &rs)?)?
        };
        let common_depth = block_path_of(&common_nv).len();

        let from = rs.from();
        let to = rs.to();
        let from_block_depth = from.path().len().saturating_sub(1);
        let to_block_depth = to.path().len().saturating_sub(1);
        let open_start = (from_block_depth.saturating_sub(common_depth)) as u32
            + from.is_inline_position() as u32;
        let open_end =
            (to_block_depth.saturating_sub(common_depth)) as u32 + to.is_inline_position() as u32;

        let fragment = build_fragment(state, &common_nv, &rs);

        let mut slice = if can_use_as_slice_roots(&fragment.children) {
            Slice::new(
                fragment.children,
                open_start.saturating_sub(1),
                open_end.saturating_sub(1),
            )
        } else {
            Slice::new(vec![fragment], open_start.max(1), open_end.max(1))
        };
        if full_document {
            clear_open_edge_carry(&mut slice.content, slice.open_start, true);
            clear_open_edge_carry(&mut slice.content, slice.open_end, false);
        }
        Some(slice)
    }

    pub fn to_text(&self) -> String {
        text_serialize::to_text(self)
    }

    pub fn from_text(text: &str) -> Slice {
        text_parse::from_text(text)
    }

    pub fn to_html(&self, resource: &Resource) -> String {
        html_serialize::to_html(self, resource)
    }

    pub fn from_html(html: &str, resource: &Resource) -> Slice {
        html_parse::from_html(html, resource)
    }

    pub fn from_payload(html: Option<&str>, text: &str, resource: &Resource) -> Slice {
        match html {
            Some(h) if !h.is_empty() => Self::from_html(h, resource),
            _ => Self::from_text(text),
        }
    }

    pub fn new(content: Vec<Fragment>, open_start: u32, open_end: u32) -> Self {
        if content.is_empty() {
            Self {
                content,
                open_start: 0,
                open_end: 0,
            }
        } else {
            Self {
                content,
                open_start,
                open_end,
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn to_payload(&self, resource: &Resource) -> ClipboardPayload {
        ClipboardPayload {
            html: self.to_html(resource),
            text: self.to_text(),
        }
    }
}

fn covers_document_content(view: &DocView, selection: &ResolvedSelection) -> bool {
    let Some(document) =
        document_content_selection(view).and_then(|selection| selection.resolve(view))
    else {
        return false;
    };
    selection.from().path() == document.from().path()
        && selection.to().path() == document.to().path()
}

fn clear_open_edge_carry(content: &mut [Fragment], depth: u32, start: bool) {
    if depth == 0 {
        return;
    }
    let fragment = if start {
        content.first_mut()
    } else {
        content.last_mut()
    };
    let Some(fragment) = fragment else {
        return;
    };
    fragment.carry.clear();
    clear_open_edge_carry(&mut fragment.children, depth - 1, start);
}

fn can_use_as_slice_roots(fragments: &[Fragment]) -> bool {
    !fragments.is_empty()
        && (fragments
            .iter()
            .all(|fragment| fragment.node.as_type().spec().inline)
            || fragments.iter().all(|fragment| {
                NodeType::Root
                    .spec()
                    .content
                    .matches(fragment.node.as_type())
            }))
}

fn common_ancestor(view: &DocView, rs: &ResolvedSelection) -> Option<Dot> {
    let a = rs.from().path();
    let b = rs.to().path();
    let an = &a[..a.len().saturating_sub(1)];
    let bn = &b[..b.len().saturating_sub(1)];
    let prefix_len = an.iter().zip(bn).take_while(|(x, y)| x == y).count();
    let prefix = &an[..prefix_len];

    let root = view.root()?;
    let mut node = root;
    for &i in prefix {
        match node.child_at(i) {
            Some(ChildView::Block(b)) => node = b,
            _ => return Some(view.root()?.id()),
        }
    }
    Some(node.id())
}

fn block_path_of(nv: &NodeView) -> Vec<usize> {
    let mut chain: Vec<usize> = nv.ancestors().filter_map(|n| n.index()).collect();
    chain.reverse();
    chain
}

fn is_prefix(prefix: &[usize], full: &[usize]) -> bool {
    full.len() >= prefix.len() && full[..prefix.len()] == *prefix
}

fn block_modifiers(state: &State, nv: &NodeView) -> Vec<Modifier> {
    match nv.dot() {
        Some(dot) => state
            .projected
            .block_modifiers()
            .modifiers_of(dot)
            .into_values()
            .collect(),
        None => vec![],
    }
}

fn node_carry(state: &State, nv: &NodeView) -> Vec<Modifier> {
    match nv.dot() {
        Some(dot) => state.projected.carry_modifiers(dot).into_values().collect(),
        None => vec![],
    }
}

fn leaf_modifiers(own: &BTreeMap<ModifierType, OwnModifier>) -> Vec<Modifier> {
    own.values().map(|o| o.value.clone()).collect()
}

struct RunAccum {
    modifiers: Vec<Modifier>,
    text: String,
}

fn flush_run(run: &mut Option<RunAccum>, out: &mut Vec<Fragment>) {
    if let Some(r) = run.take() {
        out.push(Fragment {
            node: PlainNode::Text(PlainTextNode { text: r.text }),
            modifiers: r.modifiers,
            carry: vec![],
            children: vec![],
        });
    }
}

fn push_leaf(
    leaf: &LeafView,
    own: Option<&BTreeMap<ModifierType, OwnModifier>>,
    run: &mut Option<RunAccum>,
    out: &mut Vec<Fragment>,
) {
    if let Some(ch) = leaf.as_char() {
        let modifiers = own.map(leaf_modifiers).unwrap_or_default();
        match run {
            Some(r) if r.modifiers == modifiers => r.text.push(ch),
            _ => {
                flush_run(run, out);
                *run = Some(RunAccum {
                    modifiers,
                    text: ch.to_string(),
                });
            }
        }
    } else if let Some(node) = leaf.node() {
        flush_run(run, out);
        out.push(Fragment {
            node: node.to_plain(),
            modifiers: own.map(leaf_modifiers).unwrap_or_default(),
            carry: vec![],
            children: vec![],
        });
    }
}

fn node_to_fragment(state: &State, nv: &NodeView) -> Fragment {
    let mut out: Vec<Fragment> = Vec::new();
    let mut run: Option<RunAccum> = None;
    for (slot, c) in nv.children().enumerate() {
        match c {
            ChildView::Block(b) => {
                flush_run(&mut run, &mut out);
                out.push(node_to_fragment(state, &b));
            }
            ChildView::Leaf(l) => {
                let own = nv.leaf_state_at(slot).map(|s| s.own);
                push_leaf(&l, own, &mut run, &mut out);
            }
        }
    }
    flush_run(&mut run, &mut out);
    Fragment {
        node: nv.node().to_plain(),
        modifiers: block_modifiers(state, nv),
        carry: node_carry(state, nv),
        children: out,
    }
}

fn build_fragment(state: &State, nv: &NodeView, rs: &ResolvedSelection) -> Fragment {
    if rs.contains_subtree(nv) {
        return node_to_fragment(state, nv);
    }
    let from = rs.from();
    let to = rs.to();
    let from_block_path = &from.path()[..from.path().len().saturating_sub(1)];
    let to_block_path = &to.path()[..to.path().len().saturating_sub(1)];

    let children: Vec<ChildView> = nv.children().collect();

    let mut out: Vec<Fragment> = Vec::new();
    let mut run: Option<RunAccum> = None;
    for (i, c) in children.iter().enumerate() {
        match c {
            ChildView::Leaf(l) => {
                if !rs.contains_leaf_slot(nv, i) {
                    flush_run(&mut run, &mut out);
                    continue;
                }
                let own = nv.leaf_state_at(i).map(|s| s.own);
                push_leaf(l, own, &mut run, &mut out);
            }
            ChildView::Block(b) => {
                flush_run(&mut run, &mut out);
                let bp = block_path_of(b);
                if rs.contains_subtree(b) {
                    out.push(node_to_fragment(state, b));
                } else if is_prefix(&bp, from_block_path) || is_prefix(&bp, to_block_path) {
                    out.push(build_fragment(state, b, rs));
                }
            }
        }
    }
    flush_run(&mut run, &mut out);
    Fragment {
        node: nv.node().to_plain(),
        modifiers: block_modifiers(state, nv),
        carry: vec![],
        children: out,
    }
}

fn extract_cell_rect(state: &State, view: &DocView, rect: &CellRect) -> Slice {
    let Some(table) = view.node(rect.table_id()) else {
        return Slice::new(vec![], 0, 0);
    };
    let mut rows: Vec<Fragment> = Vec::new();
    for r in rect.rows().clone() {
        let Some(ChildView::Block(row)) = table.child_at(r) else {
            continue;
        };
        let mut cells: Vec<Fragment> = Vec::new();
        for c in rect.cols().clone() {
            if let Some(ChildView::Block(cell)) = row.child_at(c) {
                cells.push(node_to_fragment(state, &cell));
            }
        }
        if cells.is_empty() {
            continue;
        }
        rows.push(Fragment {
            node: row.node().to_plain(),
            modifiers: block_modifiers(state, &row),
            carry: vec![],
            children: cells,
        });
    }
    let table_frag = Fragment {
        node: table.node().to_plain(),
        modifiers: block_modifiers(state, &table),
        carry: vec![],
        children: rows,
    };
    Slice::new(vec![table_frag], 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_doc::DocBuilder;
    use editor_macros::state;
    use editor_model::{AtomLeaf, CalloutVariant, Modifier, NodeType};
    use editor_resource::Resource;
    use editor_state::{Affinity, Position, Selection};

    fn cell_rect_sel(state: &State, anchor_cell: Dot, head_cell: Dot) -> editor_state::Selection {
        use editor_state::{Position, Selection};
        let view = state.view();
        let a = view.node(anchor_cell).unwrap();
        let h = view.node(head_cell).unwrap();
        let a_row = a.parent().unwrap().id();
        let h_row = h.parent().unwrap().id();
        let a_col = a.index().unwrap();
        let h_col = h.index().unwrap();
        let lo = a_col.min(h_col);
        let hi = a_col.max(h_col);
        Selection::new(Position::new(a_row, lo), Position::new(h_row, hi + 1))
    }

    #[test]
    fn extract_collapsed_returns_none() {
        let (s, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        assert!(Slice::extract(&s).is_none());
    }

    #[test]
    fn extract_no_selection_returns_none() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        assert!(Slice::extract(&s).is_none());
    }

    #[test]
    fn is_empty_recognizes_bare_container() {
        let slice = Slice::new(vec![], 0, 0);
        assert!(slice.is_empty());
    }

    #[test]
    fn is_empty_keeps_text_leaf_non_empty() {
        let slice = Slice {
            content: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: "hello".into(),
            }))],
            open_start: 0,
            open_end: 0,
        };
        assert!(!slice.is_empty());
    }

    #[test]
    fn is_empty_keeps_tab_leaf_non_empty() {
        let slice = Slice {
            content: vec![Fragment::leaf(PlainNode::Tab(
                editor_model::PlainTabNode::default(),
            ))],
            open_start: 0,
            open_end: 0,
        };
        assert!(!slice.is_empty());
    }

    #[test]
    fn extract_inline_within_single_textblock_is_bare_inline() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.open_start, 0);
        assert_eq!(slice.open_end, 0);
        assert_eq!(slice.content.len(), 1);
        if let editor_model::PlainNode::Text(t) = &slice.content[0].node {
            assert_eq!(t.text, "ell");
        } else {
            panic!("expected text run child");
        }
    }

    #[test]
    fn extract_full_document_paragraph_preserves_block_context() {
        let (state, ..) = state! {
            doc { root {
                p1: paragraph [alignment(editor_model::Alignment::Center)] carry([bold]) {
                    text("Hello")
                }
            } }
            selection: (p1, 0) -> (p1, 5)
        };

        let slice = Slice::extract(&state).expect("non-collapsed");

        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
        assert_eq!(slice.content.len(), 1);
        let paragraph = &slice.content[0];
        assert!(matches!(paragraph.node, PlainNode::Paragraph(_)));
        assert_eq!(paragraph.children.len(), 1);
        assert!(matches!(paragraph.children[0].node, PlainNode::Text(_)));
        assert!(paragraph.carry.is_empty());
        assert!(
            paragraph
                .modifiers
                .iter()
                .any(|modifier| matches!(modifier, Modifier::Alignment { .. }))
        );
    }

    #[test]
    fn extract_complete_paragraph_in_larger_document_remains_bare_inline() {
        let (state, ..) = state! {
            doc { root {
                p1: paragraph [alignment(editor_model::Alignment::Center)] carry([bold]) {
                    text("Hello")
                }
                paragraph { text("after") }
            } }
            selection: (p1, 0) -> (p1, 5)
        };

        let slice = Slice::extract(&state).expect("non-collapsed");

        assert_eq!((slice.open_start, slice.open_end), (0, 0));
        assert_eq!(slice.content.len(), 1);
        assert!(matches!(slice.content[0].node, PlainNode::Text(_)));
        assert!(slice.content[0].carry.is_empty());
        assert!(
            slice.content[0]
                .modifiers
                .iter()
                .all(|modifier| !matches!(modifier, Modifier::Alignment { .. }))
        );
    }

    #[test]
    fn extract_full_document_is_direction_and_affinity_independent() {
        let (state, first, last) = state! {
            doc { root {
                first: paragraph { text("first") }
                last: paragraph { text("last") }
            } }
            selection: (first, 0, >) -> (last, 4, <)
        };
        let forward = Slice::extract(&state).expect("forward selection");
        let backward_state = State {
            selection: Some(Selection::new(
                Position {
                    node: last,
                    offset: 4,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node: first,
                    offset: 0,
                    affinity: Affinity::Upstream,
                },
            )),
            ..state
        };

        let backward = Slice::extract(&backward_state).expect("backward selection");

        assert_eq!(backward, forward);
        assert_eq!((forward.open_start, forward.open_end), (1, 1));
    }

    #[test]
    fn extract_full_document_with_leading_unit_keeps_open_trailing_paragraph() {
        let (state, _root, _trailing) = state! {
            doc { _root: root { image _trailing: paragraph { text("after") } } }
            selection: (_root, 0, >) -> (_trailing, 5, <)
        };

        let slice = Slice::extract(&state).expect("non-collapsed");

        assert_eq!((slice.open_start, slice.open_end), (0, 1));
        assert_eq!(slice.content.len(), 2);
        assert!(matches!(slice.content[0].node, PlainNode::Image(_)));
        assert!(matches!(slice.content[1].node, PlainNode::Paragraph(_)));
    }

    #[test]
    fn extract_page_break_only_is_bare_inline() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("lo") page_break } } }
            selection: (p1, 2) -> (p1, 3)
        };

        let slice = Slice::extract(&state).expect("non-collapsed");

        assert_eq!(slice.open_start, 0);
        assert_eq!(slice.open_end, 0);
        assert_eq!(slice.content.len(), 1);
        assert!(matches!(slice.content[0].node, PlainNode::PageBreak(_)));
    }

    #[test]
    fn extract_text_through_page_break_is_bare_inline() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("lo") page_break } } }
            selection: (p1, 0) -> (p1, 3)
        };

        let slice = Slice::extract(&state).expect("non-collapsed");

        assert_eq!(slice.open_start, 0);
        assert_eq!(slice.open_end, 0);
        assert_eq!(slice.content.len(), 2);
        assert!(matches!(slice.content[0].node, PlainNode::Text(_)));
        assert!(matches!(slice.content[1].node, PlainNode::PageBreak(_)));
    }

    #[test]
    fn extract_whole_page_break_paragraph_keeps_closed_block() {
        let (state, ..) = state! {
            doc { root: root { paragraph { text("lo") page_break } paragraph {} } }
            selection: (root, 0) -> (root, 1)
        };

        let slice = Slice::extract(&state).expect("non-collapsed");

        assert_eq!(slice.open_start, 0);
        assert_eq!(slice.open_end, 0);
        assert_eq!(slice.content.len(), 1);
        assert!(matches!(slice.content[0].node, PlainNode::Paragraph(_)));
        assert_eq!(slice.content[0].children.len(), 2);
        assert!(matches!(
            slice.content[0].children[1].node,
            PlainNode::PageBreak(_)
        ));
    }

    #[test]
    fn extract_inline_within_fold_title_is_bare_inline() {
        let (s, ..) = state! {
            doc { root { fold {
                ft1: fold_title { text("Hello World") }
                fold_content { paragraph {} }
            } } }
            selection: (ft1, 1) -> (ft1, 4)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.open_start, 0);
        assert_eq!(slice.open_end, 0);
        assert_eq!(slice.content.len(), 1);
        if let editor_model::PlainNode::Text(t) = &slice.content[0].node {
            assert_eq!(t.text, "ell");
        } else {
            panic!("expected text run child");
        }
    }

    #[test]
    fn extract_closed_callout_child_does_not_include_fold_content_context() {
        let (state, ..) = state! {
            doc { root {
                fold {
                    fold_title { text("title") }
                    content: fold_content {
                        paragraph { text("inside") }
                        callout { paragraph { text("moved") } }
                    }
                }
                paragraph { text("after") }
            } }
            selection: (content, 1) -> (content, 2)
        };

        let slice = Slice::extract(&state).expect("non-collapsed");

        assert_eq!(slice.open_start, 0);
        assert_eq!(slice.open_end, 0);
        assert_eq!(slice.content.len(), 1);
        assert!(matches!(slice.content[0].node, PlainNode::Callout(_)));
    }

    #[test]
    fn extract_portable_children_drops_non_root_common_context() {
        let (state, ..) = state! {
            doc { root {
                fold {
                    fold_title { text("title") }
                    fold_content {
                        p1: paragraph { text("first") }
                        p2: paragraph { text("second") }
                    }
                }
            } }
            selection: (p1, 2) -> (p2, 3)
        };

        let slice = Slice::extract(&state).expect("non-collapsed");

        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
        assert_eq!(slice.content.len(), 2);
        assert!(
            slice
                .content
                .iter()
                .all(|fragment| matches!(fragment.node, PlainNode::Paragraph(_)))
        );
    }

    #[test]
    fn extract_nested_list_partial() {
        let (s, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { p1: paragraph { text("first") } }
                    list_item { p2: paragraph { text("second") } }
                    list_item { p3: paragraph { text("third") } }
                }
            } }
            selection: (p1, 2) -> (p2, 3)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.open_start, 3);
        assert_eq!(slice.open_end, 3);
        assert!(matches!(
            slice.content[0].node,
            editor_model::PlainNode::BulletList(_)
        ));
        assert_eq!(slice.content[0].children.len(), 2);
    }

    #[test]
    fn extract_complete_list_item_retains_open_list_context() {
        let (state, ..) = state! {
            doc { root {
                list: bullet_list {
                    list_item { paragraph { text("first") } }
                    list_item { paragraph { text("second") } }
                }
            } }
            selection: (list, 0) -> (list, 1)
        };

        let slice = Slice::extract(&state).expect("non-collapsed");

        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
        assert_eq!(slice.content.len(), 1);
        assert!(matches!(slice.content[0].node, PlainNode::BulletList(_)));
        assert_eq!(slice.content[0].children.len(), 1);
        assert!(matches!(
            slice.content[0].children[0].node,
            PlainNode::ListItem(_)
        ));
    }

    #[test]
    fn extract_node_selection_image() {
        let mut b = DocBuilder::new();
        let root = Dot::ROOT;
        let _p1 = b.block(NodeType::Paragraph, &[root]);
        b.text("a");
        let _img = b.image(&[root]);
        let _p2 = b.block(NodeType::Paragraph, &[root]);
        b.text("b");
        let s = b.finish(Some(Selection::new(
            Position::new(root, 1),
            Position::new(root, 2),
        )));
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.open_start, 0);
        assert_eq!(slice.open_end, 0);
        assert_eq!(slice.content.len(), 1);
        assert!(matches!(
            slice.content[0].node,
            editor_model::PlainNode::Image(_)
        ));
    }

    #[test]
    fn extract_range_between_paragraphs_excludes_image_before_selection() {
        let (s, ..) = state! {
            doc { root {
                image
                p1: paragraph { text("asd") }
                image
                p2: paragraph {}
            } }
            selection: (p1, 3) -> (p2, 0)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");

        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
        assert_eq!(slice.content.len(), 3);
        assert!(matches!(slice.content[0].node, PlainNode::Paragraph(_)));
        assert!(matches!(slice.content[1].node, PlainNode::Image(_)));
        assert!(matches!(slice.content[2].node, PlainNode::Paragraph(_)));
    }

    #[test]
    fn extract_preserves_single_op_init_block_attrs() {
        let (s, ..) = state! {
            doc { root: root {
                callout(variant: CalloutVariant::Warning) {
                    paragraph {}
                }
                paragraph {}
            } }
            selection: (root, 0) -> (root, 1)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.content.len(), 1);
        let PlainNode::Callout(plain) = &slice.content[0].node else {
            panic!("expected callout fragment");
        };
        assert_eq!(plain.variant, CalloutVariant::Warning);
    }

    #[test]
    fn extract_across_paragraphs_opens_edge_paragraphs() {
        let (s, ..) = state! {
            doc { root {
                p1: paragraph { text("abc") }
                p2: paragraph { text("xyz") }
            } }
            selection: (p1, 1) -> (p2, 2)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
        assert_eq!(slice.content.len(), 2);
    }

    #[test]
    fn extract_paragraph_break_only_preserves_plain_text_separator() {
        let (s, ..) = state! {
            doc { root {
                p1: paragraph { text("a") }
                p2: paragraph { text("b") }
            } }
            selection: (p1, 1) -> (p2, 0)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.content.len(), 2);
        assert!(
            slice
                .content
                .iter()
                .all(|child| matches!(child.node, PlainNode::Paragraph(_)))
        );
        assert_eq!(slice.to_text(), "\n");
    }

    #[test]
    fn extract_paragraph_break_only_preserves_paragraph_modifiers() {
        let (s, ..) = state! {
            doc { root {
                p1: paragraph [bold] { text("a") }
                p2: paragraph [italic] { text("b") }
            } }
            selection: (p1, 1) -> (p2, 0)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");

        assert_eq!(slice.content.len(), 2);
        assert!(
            slice.content[0]
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::Bold))
        );
        assert!(
            slice.content[1]
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::Italic))
        );
    }

    #[test]
    fn extract_empty_paragraph_break_before_non_paragraph_copies_empty_paragraph() {
        let mut b = DocBuilder::new();
        let root = Dot::ROOT;
        let p1 = b.block(NodeType::Paragraph, &[root]);
        let _img = b.image(&[root]);
        let _p2 = b.block(NodeType::Paragraph, &[root]);
        let s = b.finish(None);
        let selection = {
            let view = s.view();
            editor_state::paragraph_break_at_end(&editor_state::Position::new(p1, 0), &view)
                .expect("empty paragraph has break")
        };
        let s = State {
            selection: Some(selection),
            ..s
        };

        let slice = Slice::extract(&s).expect("non-collapsed");

        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 0);
        assert_eq!(slice.content.len(), 1);
        let paragraph = &slice.content[0];
        assert!(matches!(paragraph.node, PlainNode::Paragraph(_)));
        assert!(paragraph.children.is_empty());
    }

    #[test]
    fn extract_range_starting_with_paragraph_break_preserves_plain_text_separator() {
        let (s, ..) = state! {
            doc { root {
                p1: paragraph { text("a") }
                p2: paragraph { text("bc") }
            } }
            selection: (p1, 1) -> (p2, 1)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.to_text(), "\nb");
    }

    #[test]
    fn payload_round_trip() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let original = Slice::extract(&s).unwrap();
        let payload = original.to_payload(&Resource::new_test());
        assert!(!payload.html.is_empty());
        assert!(!payload.text.is_empty());

        let resource = Resource::new_test();
        let parsed = Slice::from_payload(Some(&payload.html), &payload.text, &resource);
        assert_eq!(parsed, original);
    }

    #[test]
    fn from_payload_text_only() {
        let parsed = Slice::from_payload(None, "hello\n\nworld", &Resource::new_test());
        assert_eq!(parsed.content.len(), 3);
    }

    #[test]
    fn extract_cell_rect_full_table_keeps_structure() {
        let (state, _tbl, _, c00, _, _, _, c11) = state! {
            doc { root { tbl: table {
                tr0: table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = cell_rect_sel(&state, c00, c11);
        let state = State {
            selection: Some(sel),
            ..state
        };
        let slice = Slice::extract(&state).expect("cell-rect must extract");
        assert_eq!(slice.content.len(), 1);
        let table = &slice.content[0];
        assert!(matches!(table.node, PlainNode::Table(_)));
        assert_eq!(table.children.len(), 2);
        for row in &table.children {
            assert!(matches!(row.node, PlainNode::TableRow(_)));
            assert_eq!(row.children.len(), 2);
            for cell in &row.children {
                assert!(matches!(cell.node, PlainNode::TableCell(_)));
            }
        }
        assert_eq!(slice.open_start, 0);
        assert_eq!(slice.open_end, 0);
    }

    #[test]
    fn extract_cell_rect_partial_carves_subtable() {
        let (state, _, c00, c01, _) = state! {
            doc { root { table { tr0: table_row {
                c00: table_cell { paragraph { text("A") } }
                c01: table_cell { paragraph { text("B") } }
                c02: table_cell { paragraph { text("C") } }
            } } } }
            selection: (c00, 0)
        };
        let sel = cell_rect_sel(&state, c00, c01);
        let state = State {
            selection: Some(sel),
            ..state
        };
        let slice = Slice::extract(&state).unwrap();
        let table = &slice.content[0];
        assert_eq!(table.children.len(), 1);
        assert_eq!(table.children[0].children.len(), 2);
    }

    #[test]
    fn extract_preserves_modifier_on_tab_fragment() {
        use editor_model::{Modifier, PlainNode};
        let mut b = DocBuilder::new();
        let root = Dot::ROOT;
        let para = b.block(NodeType::Paragraph, &[root]);
        b.text("a");
        let tab = b.atom(AtomLeaf::Tab, &[]);
        b.text("b");
        b.span(tab, tab, Modifier::FontSize { value: 2400 });
        let s = b.finish(Some(Selection::new(
            Position::new(para, 0),
            Position::new(para, 3),
        )));
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!((slice.open_start, slice.open_end), (1, 1));
        let paragraph = slice.content.first().expect("paragraph wrapper");
        assert!(matches!(paragraph.node, PlainNode::Paragraph(_)));
        let tab_frag = paragraph
            .children
            .iter()
            .find(|c| matches!(c.node, PlainNode::Tab(_)))
            .expect("Tab must appear in extracted slice");
        assert!(
            tab_frag
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 2400 })),
            "Tab's font_size modifier must be preserved in the slice"
        );
        let _ = tab;
    }

    #[test]
    fn extract_cell_rect_single_cell_emits_1x1_table() {
        let (state, _, c00, _) = state! {
            doc { root { table { tr0: table_row {
                c00: table_cell { paragraph { text("only") } }
                c01: table_cell { paragraph { text("nope") } }
            } } } }
            selection: (c00, 0)
        };
        let sel = cell_rect_sel(&state, c00, c00);
        let state = State {
            selection: Some(sel),
            ..state
        };
        let slice = Slice::extract(&state).unwrap();
        let table = &slice.content[0];
        assert_eq!(table.children.len(), 1);
        assert_eq!(table.children[0].children.len(), 1);
    }

    #[test]
    fn extract_slice_json_excludes_style_key() {
        let mut b = DocBuilder::new();
        let root = Dot::ROOT;
        let para = b.block(NodeType::Paragraph, &[root]);
        let chars = b.text("ab");
        b.span(chars[0], chars[1], Modifier::Bold);
        let s = b.finish(Some(Selection::new(
            Position::new(para, 0),
            Position::new(para, 2),
        )));
        let slice = Slice::extract(&s).expect("non-collapsed");
        let json = serde_json::to_string(&slice).unwrap();
        assert!(
            !json.contains("\"style\""),
            "slice schema must not carry style refs: {json}"
        );
    }

    #[test]
    fn extract_closed_block_fills_carry() {
        let (s, ..) = state! {
            doc { r: root {
                p1: paragraph [alignment(editor_model::Alignment::Center)] carry([bold]) {
                    text("X")
                }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        let para = &slice.content[0];
        assert!(matches!(para.node, PlainNode::Paragraph(_)));
        assert!(
            para.modifiers
                .iter()
                .any(|modifier| matches!(modifier, Modifier::Alignment { .. }))
        );
        assert!(
            para.carry.iter().any(|m| matches!(m, Modifier::Bold)),
            "a fully-contained block carries its carry modifiers into the fragment, got {:?}",
            para.carry
        );
    }

    #[test]
    fn extract_open_fragment_omits_carry() {
        let (s, ..) = state! {
            doc { root { p1: paragraph carry([bold]) { text("Hello") } } }
            selection: (p1, 1) -> (p1, 3)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert!(
            slice
                .content
                .iter()
                .all(|fragment| fragment.carry.is_empty()),
            "bare inline fragments discard source block carry, got {:?}",
            slice.content
        );
    }

    #[test]
    fn slice_without_carry_round_trips_and_defaults_empty() {
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(editor_model::PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                    text: "hi".into(),
                }))],
            }],
            open_start: 0,
            open_end: 0,
        };
        let json = serde_json::to_string(&slice).unwrap();
        assert!(
            !json.contains("carry"),
            "empty carry must be omitted on the wire (old-clipboard compat): {json}"
        );
        let parsed: Slice = serde_json::from_str(&json).expect("carry-less JSON must deserialize");
        assert_eq!(parsed, slice);
    }

    #[test]
    fn payload_wire_round_trip_preserves_carry() {
        let (s, ..) = state! {
            doc { r: root { p1: paragraph carry([bold]) { text("X") } } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let original = Slice::extract(&s).expect("non-collapsed");
        let payload = original.to_payload(&Resource::new_test());

        let resource = Resource::new_test();
        let parsed = Slice::from_payload(Some(&payload.html), &payload.text, &resource);
        let para = &parsed.content[0];
        assert!(
            para.carry.iter().any(|m| matches!(m, Modifier::Bold)),
            "carry survives the data-slice-v2 payload wire, got {:?}",
            para.carry
        );
        assert_eq!(parsed, original);
    }
}
