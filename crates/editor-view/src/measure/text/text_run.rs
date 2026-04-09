use editor_model::{Doc, Node, NodeId, NodeRef};
use std::ops::Range;

use super::resolve::{ResolvedTextStyle, resolve_text_style};

pub struct TextRun {
    pub node_id: NodeId,
    pub byte_range: Range<usize>,
    pub style: ResolvedTextStyle,
}

pub fn collect_text_runs(_doc: &Doc, paragraph: &NodeRef<'_>) -> (String, Vec<TextRun>) {
    let mut text = String::new();
    let mut runs = Vec::new();

    for child in paragraph.children() {
        if let Node::Text(text_node) = child.node() {
            let start = text.len();
            text.push_str(&text_node.text);
            let end = text.len();

            if start < end {
                runs.push(TextRun {
                    node_id: child.id(),
                    byte_range: start..end,
                    style: resolve_text_style(&child),
                });
            }
        }
    }

    (text, runs)
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;

    #[test]
    fn single_text_node() {
        let (doc, p1) = doc! {
            root { p1: paragraph { text("hello") } }
        };
        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);
        assert_eq!(text, "hello");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].byte_range, 0..5);
    }

    #[test]
    fn multiple_text_nodes() {
        let (doc, p1) = doc! {
            root { p1: paragraph { text("hello") text(" world") } }
        };
        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);
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
        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);
        assert_eq!(text, "normalbig");
        assert_eq!(runs.len(), 2);
        // "big" font_size: 2400 centiunits = 24pt = 32px
        assert!((runs[1].style.font_size - 32.0).abs() < 0.01);
    }

    #[test]
    fn empty_paragraph() {
        let (doc, p1) = doc! {
            root { p1: paragraph }
        };
        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);
        assert!(text.is_empty());
        assert!(runs.is_empty());
    }
}
