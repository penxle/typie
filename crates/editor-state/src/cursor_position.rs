use editor_model::{Node, NodeRef};

use crate::Position;

pub trait NodeRefCursorExt {
    /// Find the first cursor position within this node's subtree.
    fn first_cursor_position(&self) -> Option<Position>;

    /// Find the last cursor position within this node's subtree.
    fn last_cursor_position(&self) -> Option<Position>;
}

impl NodeRefCursorExt for NodeRef<'_> {
    fn first_cursor_position(&self) -> Option<Position> {
        if let Node::Text(_) = self.node() {
            return Some(Position::new(self.id(), 0));
        }

        match self.first_child() {
            Some(child) => child.first_cursor_position(),
            None => {
                if self.spec().content.is_leaf() {
                    let parent = self.parent()?;
                    let idx = self.index()?;
                    Some(Position::new(parent.id(), idx))
                } else {
                    Some(Position::new(self.id(), 0))
                }
            }
        }
    }

    fn last_cursor_position(&self) -> Option<Position> {
        if let Node::Text(text_node) = self.node() {
            return Some(Position::new(self.id(), text_node.text.len()));
        }

        match self.last_child() {
            Some(child) => child.last_cursor_position(),
            None => {
                if self.spec().content.is_leaf() {
                    let parent = self.parent()?;
                    let idx = self.index()?;
                    Some(Position::new(parent.id(), idx + 1))
                } else {
                    Some(Position::new(self.id(), 0))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::NodeId;

    use super::*;

    #[test]
    fn first_cursor_position_in_paragraph() {
        let (doc, t) = doc! {
            root { paragraph { t: text("Hello") } }
        };
        assert_eq!(
            doc.root().unwrap().first_cursor_position(),
            Some(Position::new(t, 0))
        );
    }

    #[test]
    fn last_cursor_position_in_paragraph() {
        let (doc, _, t2) = doc! {
            root {
                paragraph { _t1: text("Hello") }
                paragraph { t2: text("World") }
            }
        };
        assert_eq!(
            doc.root().unwrap().last_cursor_position(),
            Some(Position::new(t2, 5))
        );
    }

    #[test]
    fn first_cursor_position_images_only() {
        let (doc,) = doc! {
            root { image image image }
        };
        assert_eq!(
            doc.root().unwrap().first_cursor_position(),
            Some(Position::new(NodeId::ROOT, 0))
        );
    }

    #[test]
    fn last_cursor_position_images_only() {
        let (doc,) = doc! {
            root { image image image }
        };
        assert_eq!(
            doc.root().unwrap().last_cursor_position(),
            Some(Position::new(NodeId::ROOT, 3))
        );
    }

    #[test]
    fn first_cursor_position_empty_paragraph() {
        let (doc, p) = doc! {
            root { p: paragraph {} }
        };
        assert_eq!(
            doc.root().unwrap().first_cursor_position(),
            Some(Position::new(p, 0))
        );
    }
}
