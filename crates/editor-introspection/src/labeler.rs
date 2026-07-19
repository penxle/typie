use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeType};
use editor_state::Selection;
use hashbrown::{HashMap, HashSet};

pub(crate) struct Labeler {
    labels: HashMap<Dot, String>,
}

impl Labeler {
    pub fn new(view: &DocView, selection: Option<&Selection>) -> Self {
        let sel = match selection {
            None => {
                return Self {
                    labels: HashMap::new(),
                };
            }
            Some(s) => s,
        };

        let mut needed = HashSet::new();
        needed.insert(sel.anchor.node);
        needed.insert(sel.head.node);

        let mut labels = HashMap::new();
        let mut counters: HashMap<NodeType, usize> = HashMap::new();

        let root = view.root().unwrap();
        if needed.contains(&root.id()) {
            let abbrev = node_type_abbreviation(root.node_type());
            let counter = counters.entry(root.node_type()).or_insert(0);
            *counter += 1;
            labels.insert(root.id(), format!("{}{}", abbrev, counter));
        }

        for child in root.descendants() {
            let (id, node_type) = match child {
                ChildView::Block(b) => (b.id(), b.node_type()),
                ChildView::Leaf(l) => (l.dot(), l.node_type()),
            };
            if needed.contains(&id) {
                let abbrev = node_type_abbreviation(node_type);
                let counter = counters.entry(node_type).or_insert(0);
                *counter += 1;
                labels.insert(id, format!("{}{}", abbrev, counter));
            }
        }

        Self { labels }
    }

    pub fn label(&self, node: Dot) -> Option<&str> {
        self.labels.get(&node).map(|s| s.as_str())
    }
}

fn node_type_abbreviation(node_type: NodeType) -> &'static str {
    match node_type {
        NodeType::Root => "r",
        NodeType::Paragraph => "p",
        NodeType::Blockquote => "bq",
        NodeType::Callout => "co",
        NodeType::Text => "t",
        NodeType::BulletList => "bl",
        NodeType::OrderedList => "ol",
        NodeType::ListItem => "li",
        NodeType::Fold => "fo",
        NodeType::FoldTitle => "ft",
        NodeType::FoldContent => "fc",
        NodeType::Table => "tb",
        NodeType::TableRow => "tr",
        NodeType::TableCell => "tc",
        NodeType::Image => "img",
        NodeType::File => "f",
        NodeType::Embed => "em",
        NodeType::Archived => "ar",
        NodeType::HardBreak => "hb",
        NodeType::HorizontalRule => "hr",
        NodeType::PageBreak => "pb",
        NodeType::Tab => "tab",
        NodeType::Unknown => "unk",
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;

    #[test]
    fn collapsed_selection_labels_one_node() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let labeler = Labeler::new(&state.view(), state.selection.as_ref());
        assert_eq!(labeler.label(p1), Some("p1"));
    }

    #[test]
    fn range_selection_labels_two_nodes() {
        let (state, p1, p2) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    p2: paragraph { text("World") }
                }
            }
            selection: (p1, 0) -> (p2, 3)
        };
        let labeler = Labeler::new(&state.view(), state.selection.as_ref());
        assert_eq!(labeler.label(p1), Some("p1"));
        assert_eq!(labeler.label(p2), Some("p2"));
    }

    #[test]
    fn same_node_selection_labels_once() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let labeler = Labeler::new(&state.view(), state.selection.as_ref());
        assert_eq!(labeler.label(p1), Some("p1"));
    }

    #[test]
    fn labels_use_node_type_abbreviation() {
        let (state, p1) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let labeler = Labeler::new(&state.view(), state.selection.as_ref());
        assert_eq!(labeler.label(p1), Some("p1"));
    }

    #[test]
    fn depth_first_ordering() {
        let (state, p1, p2) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    paragraph { text("B") }
                    p2: paragraph { text("C") }
                }
            }
            selection: (p2, 0) -> (p1, 0)
        };
        let labeler = Labeler::new(&state.view(), state.selection.as_ref());
        assert_eq!(labeler.label(p1), Some("p1"));
        assert_eq!(labeler.label(p2), Some("p2"));
    }

    #[test]
    fn unlabeled_node_returns_none() {
        let (state, _, p2) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p1, 0)
        };
        let labeler = Labeler::new(&state.view(), state.selection.as_ref());
        assert_eq!(labeler.label(p2), None);
    }

    #[test]
    fn root_selection_labeled() {
        let (state, ..) = state! {
            doc { r: root { p1: paragraph { text("A") } } }
            selection: (r, 0)
        };
        let labeler = Labeler::new(&state.view(), state.selection.as_ref());
        assert_eq!(labeler.label(Dot::ROOT), Some("r1"));
    }
}
