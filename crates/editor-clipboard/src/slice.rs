use editor_model::{Fragment, Node, NodeRef, PlainNode, PlainRootNode, PlainTextNode};
use editor_resource::Resource;
use editor_state::{CellRect, ResolvedSelection, State, is_prefix_of};
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
        let rs = state.selection.as_ref()?.resolve(&state.doc)?;
        if rs.is_collapsed() {
            return None;
        }
        if let Some(rect) = rs.as_cell_rect() {
            return Some(extract_cell_rect(&rect));
        }
        let common = rs.common_ancestor();
        let common_depth = common.path().len();
        let open_start = (rs
            .from()
            .path()
            .len()
            .saturating_sub(1)
            .saturating_sub(common_depth)) as u32;
        let open_end = (rs
            .to()
            .path()
            .len()
            .saturating_sub(1)
            .saturating_sub(common_depth)) as u32;

        let fragment = build_fragment(&rs, common);
        // When the selection is contained within a single inline node, the
        // common ancestor is that inline node, so build_fragment returns a bare
        // inline leaf. Wrap it in the enclosing textblock and bump
        // open_start/open_end so all inline selections keep their source
        // textblock shape as wider textblock selections.
        let (fragment, open_start, open_end) =
            wrap_bare_inline_in_enclosing_textblock(fragment, common, open_start, open_end);
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

fn nearest_enclosing_textblock<'a>(node: NodeRef<'a>) -> Option<NodeRef<'a>> {
    let mut current = Some(node);
    while let Some(n) = current {
        if n.spec().is_textblock() {
            return Some(n);
        }
        current = n.parent();
    }
    None
}

fn wrap_bare_inline_in_enclosing_textblock(
    fragment: Fragment,
    common: NodeRef<'_>,
    open_start: u32,
    open_end: u32,
) -> (Fragment, u32, u32) {
    let is_bare_inline = matches!(
        fragment.node,
        PlainNode::Text(_) | PlainNode::HardBreak(_) | PlainNode::Tab(_)
    );
    if !is_bare_inline {
        return (fragment, open_start, open_end);
    }
    let Some(textblock) = nearest_enclosing_textblock(common) else {
        return (fragment, open_start, open_end);
    };
    let wrapped = Fragment {
        node: textblock.node().to_plain(),
        modifiers: textblock.explicit_modifiers().cloned().collect(),
        style: textblock.entry().style.get().clone(),
        children: vec![fragment],
    };
    (wrapped, open_start + 1, open_end + 1)
}

fn build_fragment<'a>(rs: &ResolvedSelection<'a>, node: NodeRef<'a>) -> Fragment {
    if rs.contains_subtree(&node) {
        return node_to_fragment(node);
    }
    match node.node() {
        Node::Text(text_node) => {
            let s = text_node.text.to_string();
            let from = rs.from();
            let to = rs.to();
            let from_node_path = &from.path()[..from.path().len() - 1];
            let to_node_path = &to.path()[..to.path().len() - 1];
            let lo = if node.path() == from_node_path {
                from.offset()
            } else {
                0
            };
            let hi = if node.path() == to_node_path {
                to.offset()
            } else {
                s.chars().count()
            };
            let substring: String = s.chars().skip(lo).take(hi - lo).collect();
            Fragment::leaf(PlainNode::Text(PlainTextNode { text: substring }))
                .with_modifiers(node.explicit_modifiers().cloned().collect())
        }
        _ => {
            let from_node_path = rs
                .doc()
                .node(rs.from().node_id())
                .expect("from node exists")
                .path();
            let to_node_path = rs
                .doc()
                .node(rs.to().node_id())
                .expect("to node exists")
                .path();
            let kids: Vec<Fragment> = node
                .children()
                .filter(|c| {
                    let cp = c.path();
                    rs.contains_subtree(c)
                        || is_prefix_of(&cp, &from_node_path)
                        || is_prefix_of(&cp, &to_node_path)
                })
                .map(|c| build_fragment(rs, c))
                .collect();
            Fragment {
                node: node.node().to_plain(),
                modifiers: node.explicit_modifiers().cloned().collect(),
                style: node.entry().style.get().clone(),
                children: kids,
            }
        }
    }
}

fn node_to_fragment(node: NodeRef<'_>) -> Fragment {
    Fragment {
        node: node.node().to_plain(),
        modifiers: node.explicit_modifiers().cloned().collect(),
        style: node.entry().style.get().clone(),
        children: node.children().map(node_to_fragment).collect(),
    }
}

fn extract_cell_rect(rect: &CellRect<'_>) -> Slice {
    let table = rect.table;
    let mut rows: Vec<Fragment> = Vec::new();
    for r in rect.rows.clone() {
        let Some(row) = table.children().nth(r) else {
            continue;
        };
        let mut cells: Vec<Fragment> = Vec::new();
        for c in rect.cols.clone() {
            if let Some(cell) = row.children().nth(c) {
                cells.push(node_to_fragment(cell));
            }
        }
        if cells.is_empty() {
            continue;
        }
        rows.push(Fragment {
            node: row.node().to_plain(),
            modifiers: row.explicit_modifiers().cloned().collect(),
            style: row.entry().style.get().clone(),
            children: cells,
        });
    }
    let table_frag = Fragment {
        node: table.node().to_plain(),
        modifiers: table.explicit_modifiers().cloned().collect(),
        style: table.entry().style.get().clone(),
        children: rows,
    };
    Slice {
        fragment: Fragment {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: vec![],
            style: None,
            children: vec![table_frag],
        },
        open_start: 0,
        open_end: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;
    use editor_resource::Resource;

    #[test]
    fn extract_collapsed_returns_none() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        assert!(Slice::extract(&s).is_none());
    }

    #[test]
    fn extract_no_selection_returns_none() {
        let (s, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
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
    fn extract_inline_within_single_text_node() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
        assert!(matches!(
            slice.fragment.node,
            editor_model::PlainNode::Paragraph(_)
        ));
        if let editor_model::PlainNode::Text(t) = &slice.fragment.children[0].node {
            assert_eq!(t.text, "ell");
        } else {
            panic!("expected text node");
        }
    }

    #[test]
    fn extract_inline_within_fold_title_keeps_fold_title_wrapper() {
        let (s, ..) = state! {
            doc { root { fold {
                fold_title { t1: text("Hello World") }
                fold_content { paragraph {} }
            } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
        assert!(matches!(
            slice.fragment.node,
            editor_model::PlainNode::FoldTitle(_)
        ));
        if let editor_model::PlainNode::Text(t) = &slice.fragment.children[0].node {
            assert_eq!(t.text, "ell");
        } else {
            panic!("expected text node");
        }
    }

    #[test]
    fn extract_nested_list_partial() {
        let (s, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { t1: text("first") } }
                    list_item { paragraph { t2: text("second") } }
                    list_item { paragraph { t3: text("third") } }
                }
            } }
            selection: (t1, 2) -> (t2, 3)
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
        let (s, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                image
                paragraph { text("b") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
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
                paragraph { t1: text("abc") }
                paragraph { t2: text("xyz") }
            } }
            selection: (t1, 1) -> (t2, 2)
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
    fn extract_across_text_nodes_in_same_paragraph() {
        let (s, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello")
                t2: text("World")
            } } }
            selection: (t1, 2) -> (t2, 3)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
        assert!(matches!(
            slice.fragment.node,
            editor_model::PlainNode::Paragraph(_)
        ));
        assert_eq!(slice.fragment.children.len(), 2);
        if let editor_model::PlainNode::Text(t) = &slice.fragment.children[0].node {
            assert_eq!(t.text, "llo");
        } else {
            panic!("expected text in children[0]");
        }
        if let editor_model::PlainNode::Text(t) = &slice.fragment.children[1].node {
            assert_eq!(t.text, "Wor");
        } else {
            panic!("expected text in children[1]");
        }
    }

    #[test]
    fn payload_round_trip() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
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
        assert_eq!(parsed.fragment.children.len(), 2);
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
        let sel = editor_state::cell_rect_selection(&state.doc, c00, c11).unwrap();
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
        let sel = editor_state::cell_rect_selection(&state.doc, c00, c01).unwrap();
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
        let (s, t1, tab, t2) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("a")
                        tab: tab [font_size(2400)]
                        t2: text("b")
                    }
                }
            }
            selection: (t1, 0) -> (t2, 1)
        };
        let _ = (t1, t2);
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
        let sel = editor_state::cell_rect_selection(&state.doc, c00, c00).unwrap();
        let state = State {
            selection: Some(sel),
            ..state
        };
        let slice = Slice::extract(&state).unwrap();
        let table = &slice.fragment.children[0];
        assert_eq!(table.children.len(), 1);
        assert_eq!(table.children[0].children.len(), 1);
    }
}
