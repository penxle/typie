use editor_model::Node;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlatClass {
    Text,
    Break,
    Atom,
    Container,
}

pub fn classify(node: &Node) -> FlatClass {
    match node {
        Node::Text(_) => FlatClass::Text,
        _ => {
            let spec = node.spec();
            if spec.inline {
                FlatClass::Break
            } else if spec.is_leaf() {
                FlatClass::Atom
            } else {
                FlatClass::Container
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_model::{HardBreakNode, ImageNode, ParagraphNode, TextNode};

    #[test]
    fn classify_returns_text_for_text_node() {
        let node = Node::Text(TextNode::default());
        assert_eq!(classify(&node), FlatClass::Text);
    }

    #[test]
    fn classify_returns_break_for_inline_leaf() {
        assert_eq!(
            classify(&Node::HardBreak(HardBreakNode {})),
            FlatClass::Break
        );
    }

    #[test]
    fn classify_returns_atom_for_noninline_leaf() {
        let img = Node::Image(ImageNode::default());
        assert_eq!(classify(&img), FlatClass::Atom);
    }

    #[test]
    fn classify_returns_container_for_paragraph() {
        assert_eq!(
            classify(&Node::Paragraph(ParagraphNode::default())),
            FlatClass::Container
        );
    }
}
