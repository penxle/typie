use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{
    AtomLeaf, ChildView, DocView, Fragment, HardBreakNode, LeafView, Modifier, ModifierType, Node,
    NodeView, OwnModifier, PageBreakNode, PlainHorizontalRuleNode, PlainNode, PlainRootNode,
    PlainTextNode, TabNode,
};
use editor_resource::Resource;
use editor_state::State;
use editor_state::{CellRect, ResolvedSelection};
use serde::{Deserialize, Serialize};

use crate::html::parse as html_parse;
use crate::html::serialize as html_serialize;
use crate::payload::ClipboardPayload;
use crate::text::parse as text_parse;
use crate::text::serialize as text_serialize;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slice {
    pub fragment: Fragment,
    #[serde(default)]
    pub open_start: u32,
    #[serde(default)]
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
        let common_id = common_ancestor(&view, &rs)?;
        let common_nv = view.node(common_id)?;
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
        Some(Slice {
            fragment,
            open_start,
            open_end,
        })
    }

    pub fn to_text(&self) -> String {
        text_serialize::to_text(self)
    }

    pub fn from_text(text: &str) -> Slice {
        text_parse::from_text(text)
    }

    pub fn to_html(&self) -> String {
        html_serialize::to_html(self)
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

    pub fn is_empty(&self) -> bool {
        self.fragment.children.is_empty()
            && !matches!(
                self.fragment.node,
                PlainNode::Text(_) | PlainNode::HardBreak(_) | PlainNode::Tab(_)
            )
    }

    pub fn to_payload(&self) -> ClipboardPayload {
        ClipboardPayload {
            html: self.to_html(),
            text: self.to_text(),
        }
    }
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

fn leaf_modifiers(own: &BTreeMap<ModifierType, OwnModifier>) -> Vec<Modifier> {
    own.values().map(|o| o.value.clone()).collect()
}

fn atom_to_plain(leaf: &AtomLeaf) -> PlainNode {
    match leaf {
        AtomLeaf::HardBreak => Node::HardBreak(HardBreakNode {}).to_plain(),
        AtomLeaf::Tab => Node::Tab(TabNode {}).to_plain(),
        AtomLeaf::PageBreak => Node::PageBreak(PageBreakNode {}).to_plain(),
        AtomLeaf::HorizontalRule { variant } => {
            PlainNode::HorizontalRule(PlainHorizontalRuleNode { variant: *variant })
        }
        AtomLeaf::Image { node } => Node::Image(node.clone()).to_plain(),
        AtomLeaf::File { node } => Node::File(node.clone()).to_plain(),
        AtomLeaf::Embed { node } => Node::Embed(node.clone()).to_plain(),
        AtomLeaf::Archived { node } => Node::Archived(node.clone()).to_plain(),
    }
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
    } else if let Some(atom) = leaf.as_atom() {
        flush_run(run, out);
        out.push(Fragment {
            node: atom_to_plain(atom),
            modifiers: own.map(leaf_modifiers).unwrap_or_default(),
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
    let lo = if nv.id() == from.node() {
        from.offset()
    } else {
        0
    };
    let hi = if nv.id() == to.node() {
        to.offset()
    } else {
        children.len()
    };

    let mut out: Vec<Fragment> = Vec::new();
    let mut run: Option<RunAccum> = None;
    for (i, c) in children.iter().enumerate() {
        match c {
            ChildView::Leaf(l) => {
                if i < lo || i >= hi {
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
        children: out,
    }
}

fn extract_cell_rect(state: &State, view: &DocView, rect: &CellRect) -> Slice {
    let Some(table) = view.node(rect.table_id()) else {
        return Slice {
            fragment: Fragment::leaf(PlainNode::Root(PlainRootNode::default())),
            open_start: 0,
            open_end: 0,
        };
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
            children: cells,
        });
    }
    let table_frag = Fragment {
        node: table.node().to_plain(),
        modifiers: block_modifiers(state, &table),
        children: rows,
    };
    Slice {
        fragment: Fragment {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: vec![],
            children: vec![table_frag],
        },
        open_start: 0,
        open_end: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_doc::DocBuilder;
    use editor_macros::state;
    use editor_model::{AtomLeaf, Modifier, NodeType};
    use editor_resource::Resource;
    use editor_state::{Position, Selection};

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
        let slice = Slice {
            fragment: Fragment::leaf(PlainNode::Root(PlainRootNode::default())),
            open_start: 0,
            open_end: 0,
        };
        assert!(slice.is_empty());
    }

    #[test]
    fn is_empty_keeps_text_leaf_non_empty() {
        let slice = Slice {
            fragment: Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: "hello".into(),
            })),
            open_start: 0,
            open_end: 0,
        };
        assert!(!slice.is_empty());
    }

    #[test]
    fn is_empty_keeps_tab_leaf_non_empty() {
        let slice = Slice {
            fragment: Fragment::leaf(PlainNode::Tab(editor_model::PlainTabNode::default())),
            open_start: 0,
            open_end: 0,
        };
        assert!(!slice.is_empty());
    }

    #[test]
    fn extract_inline_within_single_textblock_wraps_in_open_textblock() {
        // No more text-node identity: an inline selection within one textblock
        // yields the (carved) textblock fragment with both edges open so paste
        // merges the run inline.
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
        assert!(matches!(slice.fragment.node, PlainNode::Paragraph(_)));
        assert_eq!(slice.fragment.children.len(), 1);
        if let editor_model::PlainNode::Text(t) = &slice.fragment.children[0].node {
            assert_eq!(t.text, "ell");
        } else {
            panic!("expected text run child");
        }
    }

    #[test]
    fn extract_inline_within_fold_title_wraps_in_open_textblock() {
        let (s, ..) = state! {
            doc { root { fold {
                ft1: fold_title { text("Hello World") }
                fold_content { paragraph {} }
            } } }
            selection: (ft1, 1) -> (ft1, 4)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
        assert!(matches!(slice.fragment.node, PlainNode::FoldTitle(_)));
        assert_eq!(slice.fragment.children.len(), 1);
        if let editor_model::PlainNode::Text(t) = &slice.fragment.children[0].node {
            assert_eq!(t.text, "ell");
        } else {
            panic!("expected text run child");
        }
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
            slice.fragment.node,
            editor_model::PlainNode::BulletList(_)
        ));
        assert_eq!(slice.fragment.children.len(), 2);
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
        assert!(matches!(
            slice.fragment.node,
            editor_model::PlainNode::Root(_)
        ));
        assert_eq!(slice.fragment.children.len(), 1);
        assert!(matches!(
            slice.fragment.children[0].node,
            editor_model::PlainNode::Image(_)
        ));
    }

    #[test]
    fn extract_across_paragraphs_open_two() {
        let (s, ..) = state! {
            doc { root {
                p1: paragraph { text("abc") }
                p2: paragraph { text("xyz") }
            } }
            selection: (p1, 1) -> (p2, 2)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.open_start, 2);
        assert_eq!(slice.open_end, 2);
        assert!(matches!(
            slice.fragment.node,
            editor_model::PlainNode::Root(_)
        ));
        assert_eq!(slice.fragment.children.len(), 2);
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
        assert!(matches!(slice.fragment.node, PlainNode::Root(_)));
        assert_eq!(slice.fragment.children.len(), 2);
        assert!(
            slice
                .fragment
                .children
                .iter()
                .all(|child| matches!(child.node, PlainNode::Paragraph(_)))
        );
        assert_eq!(slice.to_text(), "\n\n");
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

        assert_eq!(slice.fragment.children.len(), 2);
        assert!(
            slice.fragment.children[0]
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::Bold))
        );
        assert!(
            slice.fragment.children[1]
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

        assert!(matches!(slice.fragment.node, PlainNode::Root(_)));
        assert_eq!(slice.open_start, 2);
        assert_eq!(slice.open_end, 0);
        assert_eq!(slice.fragment.children.len(), 1);
        let paragraph = &slice.fragment.children[0];
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
        assert_eq!(slice.to_text(), "\n\nb");
    }

    #[test]
    fn payload_round_trip() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let original = Slice::extract(&s).unwrap();
        let payload = original.to_payload();
        assert!(!payload.html.is_empty());
        assert!(!payload.text.is_empty());

        let resource = Resource::new_test();
        let parsed = Slice::from_payload(Some(&payload.html), &payload.text, &resource);
        assert_eq!(parsed, original);
    }

    #[test]
    fn from_payload_text_only() {
        let parsed = Slice::from_payload(None, "hello\n\nworld", &Resource::new_test());
        assert_eq!(parsed.fragment.children.len(), 3);
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
        assert!(matches!(slice.fragment.node, PlainNode::Root(_)));
        assert_eq!(slice.fragment.children.len(), 1);
        let table = &slice.fragment.children[0];
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
        let table = &slice.fragment.children[0];
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
        let para = &slice.fragment;
        let tab_frag = para
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
        let table = &slice.fragment.children[0];
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
}
