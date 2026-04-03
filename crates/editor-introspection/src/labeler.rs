use editor_model::{Doc, Node, NodeId, NodeType};
use editor_state::Selection;
use std::collections::{HashMap, HashSet};

pub(crate) struct Labeler {
    labels: HashMap<NodeId, String>,
}

impl Labeler {
    pub fn new(doc: &Doc, selection: &Selection) -> Self {
        let mut needed = HashSet::new();
        if selection.anchor.node_id != NodeId::ROOT {
            needed.insert(selection.anchor.node_id);
        }
        if selection.head.node_id != NodeId::ROOT {
            needed.insert(selection.head.node_id);
        }

        let mut labels = HashMap::new();
        let mut counters: HashMap<NodeType, usize> = HashMap::new();

        for node_ref in doc.root().descendants() {
            if needed.contains(&node_ref.id()) {
                let abbrev = node_type_abbreviation(node_ref.node());
                let counter = counters.entry(node_ref.as_type()).or_insert(0);
                *counter += 1;
                labels.insert(node_ref.id(), format!("{}{}", abbrev, counter));
            }
        }

        Self { labels }
    }

    pub fn label(&self, node_id: NodeId) -> Option<&str> {
        self.labels.get(&node_id).map(|s| s.as_str())
    }
}

fn node_type_abbreviation(node: &Node) -> &'static str {
    match node {
        Node::Root(_) => "r",
        Node::Paragraph(_) => "p",
        Node::Blockquote(_) => "bq",
        Node::Callout(_) => "co",
        Node::Text(_) => "t",
        Node::BulletList(_) => "bl",
        Node::OrderedList(_) => "ol",
        Node::ListItem(_) => "li",
        Node::Fold(_) => "fo",
        Node::FoldTitle(_) => "ft",
        Node::FoldContent(_) => "fc",
        Node::Table(_) => "tb",
        Node::TableRow(_) => "tr",
        Node::TableCell(_) => "tc",
        Node::Image(_) => "img",
        Node::File(_) => "f",
        Node::Embed(_) => "em",
        Node::Archived(_) => "ar",
        Node::HardBreak(_) => "hb",
        Node::HorizontalRule(_) => "hr",
        Node::PageBreak(_) => "pb",
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;

    #[test]
    fn collapsed_selection_labels_one_node() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let labeler = Labeler::new(&state.doc, &state.selection);
        assert_eq!(labeler.label(t1), Some("t1"));
    }

    #[test]
    fn range_selection_labels_two_nodes() {
        let (state, t1, t2) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    paragraph { t2: text("World") }
                }
            }
            selection: (t1, 0) -> (t2, 3)
        };
        let labeler = Labeler::new(&state.doc, &state.selection);
        assert_eq!(labeler.label(t1), Some("t1"));
        assert_eq!(labeler.label(t2), Some("t2"));
    }

    #[test]
    fn same_node_selection_labels_once() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let labeler = Labeler::new(&state.doc, &state.selection);
        assert_eq!(labeler.label(t1), Some("t1"));
    }

    #[test]
    fn labels_use_node_type_abbreviation() {
        let (state, p1) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let labeler = Labeler::new(&state.doc, &state.selection);
        assert_eq!(labeler.label(p1), Some("p1"));
    }

    #[test]
    fn depth_first_ordering() {
        let (state, t1, t2) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    paragraph { text("B") }
                    paragraph { t2: text("C") }
                }
            }
            selection: (t2, 0) -> (t1, 0)
        };
        let labeler = Labeler::new(&state.doc, &state.selection);
        assert_eq!(labeler.label(t1), Some("t1"));
        assert_eq!(labeler.label(t2), Some("t2"));
    }

    #[test]
    fn unlabeled_node_returns_none() {
        let (state, _, t2) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    paragraph { t2: text("B") }
                }
            }
            selection: (t1, 0)
        };
        let labeler = Labeler::new(&state.doc, &state.selection);
        assert_eq!(labeler.label(t2), None);
    }

    #[test]
    fn root_selection_excluded_from_labels() {
        let (state, ..) = state! {
            doc { r: root { paragraph { t1: text("A") } } }
            selection: (r, 0)
        };
        let labeler = Labeler::new(&state.doc, &state.selection);
        assert_eq!(labeler.label(NodeId::ROOT), None);
    }
}
