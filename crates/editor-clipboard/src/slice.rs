use editor_model::{Fragment, Node, NodeRef, PlainNode, PlainTextNode};
use editor_state::{ResolvedSelection, State, is_prefix_of};
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

    pub fn from_html(html: &str) -> Slice {
        html_parse::from_html(html)
    }

    pub fn from_payload(html: Option<&str>, text: &str) -> Slice {
        match html {
            Some(h) if !h.is_empty() => Self::from_html(h),
            _ => Self::from_text(text),
        }
    }

    pub fn to_payload(&self) -> ClipboardPayload {
        ClipboardPayload {
            html: self.to_html(),
            text: self.to_text(),
        }
    }
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
                .with_modifiers(node.modifiers().cloned().collect())
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
                modifiers: node.modifiers().cloned().collect(),
                children: kids,
            }
        }
    }
}

fn node_to_fragment(node: NodeRef<'_>) -> Fragment {
    Fragment {
        node: node.node().to_plain(),
        modifiers: node.modifiers().cloned().collect(),
        children: node.children().map(node_to_fragment).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

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
    fn extract_inline_within_single_text_node() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let slice = Slice::extract(&s).expect("non-collapsed");
        assert_eq!(slice.open_start, 0);
        assert_eq!(slice.open_end, 0);
        if let editor_model::PlainNode::Text(t) = &slice.fragment.node {
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

        let parsed = Slice::from_payload(Some(&payload.html), &payload.text);
        assert_eq!(parsed, original);
    }

    #[test]
    fn from_payload_text_only() {
        let parsed = Slice::from_payload(None, "hello\n\nworld");
        assert_eq!(parsed.fragment.children.len(), 2);
    }
}
