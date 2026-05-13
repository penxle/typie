use editor_model::{Node, NodeId, NodeRef};
use std::ops::Range;

use super::resolve::{ResolvedTextStyle, resolve_text_style};

pub struct TextRun {
    pub node_id: NodeId,
    pub byte_range: Range<usize>,
    pub style: ResolvedTextStyle,
}

pub fn collect_text_runs_for(children: &[NodeRef<'_>]) -> (String, Vec<TextRun>) {
    let mut text = String::new();
    let mut runs = Vec::new();

    for child in children {
        if let Node::Text(text_node) = child.node() {
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
    }

    (text, runs)
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
        let (text, runs) = collect_text_runs_for(&children_of(&doc, p1));
        assert_eq!(text, "hello");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].byte_range, 0..5);
    }

    #[test]
    fn multiple_text_nodes() {
        let (doc, p1) = doc! { root { p1: paragraph { text("hello") text(" world") } } };
        let (text, runs) = collect_text_runs_for(&children_of(&doc, p1));
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
        let (text, runs) = collect_text_runs_for(&children_of(&doc, p1));
        assert_eq!(text, "normalbig");
        assert_eq!(runs.len(), 2);
        assert!((runs[1].style.font_size - 32.0).abs() < 0.01);
    }

    #[test]
    fn empty_paragraph() {
        let (doc, p1) = doc! { root { p1: paragraph } };
        let (text, runs) = collect_text_runs_for(&children_of(&doc, p1));
        assert!(text.is_empty());
        assert!(runs.is_empty());
    }
}
