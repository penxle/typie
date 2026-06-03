use editor_model::{Node, NodeId, NodeRef};
use std::ops::Range;

use super::resolve::{ResolvedTextStyle, resolve_text_style};

pub struct TextRun {
    pub node_id: NodeId,
    pub byte_range: Range<usize>,
    pub style: ResolvedTextStyle,
}

pub struct TabMark {
    pub node_id: NodeId,
    pub child_index: usize,
    pub byte_offset: usize,
}

pub fn collect_text_runs_for(children: &[NodeRef<'_>]) -> (String, Vec<TextRun>, Vec<TabMark>) {
    let mut text = String::new();
    let mut runs = Vec::new();
    let mut tabs = Vec::new();

    for (idx, child) in children.iter().enumerate() {
        match child.node() {
            Node::Text(text_node) => {
                let start = text.len();
                text.push_str(&text_node.text.to_string());
                let end = text.len();
                if start < end {
                    runs.push(TextRun {
                        node_id: child.id(),
                        byte_range: start..end,
                        style: resolve_text_style(child),
                    });
                }
            }
            Node::Tab(_) => {
                tabs.push(TabMark {
                    node_id: child.id(),
                    child_index: child.index().unwrap_or(idx),
                    byte_offset: text.len(),
                });
            }
            _ => {}
        }
    }

    (text, runs, tabs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::doc;

    fn children_of(doc: &editor_model::Doc, p: editor_model::NodeId) -> Vec<NodeRef<'_>> {
        doc.node(p).unwrap().children().collect()
    }

    #[test]
    fn single_text_node() {
        let (doc, p1) = doc! { root { p1: paragraph { text("hello") } } };
        let (text, runs, _tabs) = collect_text_runs_for(&children_of(&doc, p1));
        assert_eq!(text, "hello");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].byte_range, 0..5);
    }

    #[test]
    fn multiple_text_nodes() {
        let (doc, p1) = doc! { root { p1: paragraph { text("hello") text(" world") } } };
        let (text, runs, _tabs) = collect_text_runs_for(&children_of(&doc, p1));
        assert_eq!(text, "hello world");
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].byte_range, 0..5);
        assert_eq!(runs[1].byte_range, 5..11);
    }

    #[test]
    fn text_node_with_modifiers() {
        let (doc, p1) = doc! {
            root {
                p1: paragraph {
                    text("normal")
                    text("big") [font_size(2400)]
                }
            }
        };
        let (text, runs, _tabs) = collect_text_runs_for(&children_of(&doc, p1));
        assert_eq!(text, "normalbig");
        assert_eq!(runs.len(), 2);
        assert!((runs[1].style.font_size - 32.0).abs() < 0.01);
    }

    #[test]
    fn empty_paragraph() {
        let (doc, p1) = doc! { root { p1: paragraph } };
        let (text, runs, _tabs) = collect_text_runs_for(&children_of(&doc, p1));
        assert!(text.is_empty());
        assert!(runs.is_empty());
    }

    #[test]
    fn records_tab_mark_byte_offset_and_index() {
        let (doc, p1) = doc! { root { p1: paragraph { text("ab") tab text("c") } } };
        let children: Vec<NodeRef<'_>> = doc.node(p1).unwrap().children().collect();
        let (text, _runs, tabs) = collect_text_runs_for(&children);
        assert_eq!(text, "abc");
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].child_index, 1);
        assert_eq!(tabs[0].byte_offset, 2);
    }
}
