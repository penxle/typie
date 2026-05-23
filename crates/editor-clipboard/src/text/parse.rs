use editor_model::{
    Fragment, PlainHardBreakNode, PlainNode, PlainParagraphNode, PlainRootNode, PlainTextNode,
};

use crate::slice::Slice;

pub fn from_text(text: &str) -> Slice {
    let stripped = text.trim_start_matches('\u{feff}');
    let normalized = stripped.replace("\r\n", "\n").replace('\r', "\n");
    let blocks: Vec<Fragment> = normalized.split("\n\n").map(paragraph_from_block).collect();

    Slice {
        fragment: Fragment {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: vec![],
            children: blocks,
        },
        open_start: 0,
        open_end: 0,
    }
}

fn paragraph_from_block(block: &str) -> Fragment {
    let mut inline: Vec<Fragment> = Vec::new();
    let mut first = true;
    for line in block.split('\n') {
        if !first {
            inline.push(Fragment::leaf(PlainNode::HardBreak(
                PlainHardBreakNode::default(),
            )));
        }
        if !line.is_empty() {
            inline.push(Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: line.to_string(),
            })));
        }
        first = false;
    }
    Fragment {
        node: PlainNode::Paragraph(PlainParagraphNode::default()),
        modifiers: vec![],
        children: inline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_text_single_paragraph() {
        let slice = from_text("Hello World");
        assert!(matches!(slice.fragment.node, PlainNode::Root(_)));
        assert_eq!(slice.fragment.children.len(), 1);
        assert!(matches!(
            slice.fragment.children[0].node,
            PlainNode::Paragraph(_)
        ));
        let p = &slice.fragment.children[0];
        assert_eq!(p.children.len(), 1);
        if let PlainNode::Text(t) = &p.children[0].node {
            assert_eq!(t.text, "Hello World");
        } else {
            panic!("expected text");
        }
        assert_eq!(slice.open_start, 0);
        assert_eq!(slice.open_end, 0);
    }

    #[test]
    fn from_text_double_newline_splits_paragraph() {
        let slice = Slice::from_text("first\n\nsecond");
        assert_eq!(slice.fragment.children.len(), 2);
    }

    #[test]
    fn from_text_single_newline_hardbreak() {
        let slice = Slice::from_text("line1\nline2");
        assert_eq!(slice.fragment.children.len(), 1);
        let p = &slice.fragment.children[0];
        assert_eq!(p.children.len(), 3);
    }

    #[test]
    fn from_text_strips_bom() {
        let slice = Slice::from_text("\u{feff}hello");
        if let PlainNode::Text(t) = &slice.fragment.children[0].children[0].node {
            assert_eq!(t.text, "hello");
        } else {
            panic!("expected text");
        }
    }

    #[test]
    fn from_text_crlf_normalizes_to_lf() {
        let slice = Slice::from_text("a\r\nb");
        assert_eq!(slice.fragment.children.len(), 1);
        let p = &slice.fragment.children[0];
        assert_eq!(p.children.len(), 3);
        if let PlainNode::Text(t) = &p.children[0].node {
            assert_eq!(t.text, "a");
        } else {
            panic!("expected text");
        }
        assert!(matches!(p.children[1].node, PlainNode::HardBreak(_)));
        if let PlainNode::Text(t) = &p.children[2].node {
            assert_eq!(t.text, "b");
        } else {
            panic!("expected text");
        }
    }

    #[test]
    fn from_text_crlf_double_splits_paragraph() {
        let slice = Slice::from_text("a\r\n\r\nb");
        assert_eq!(slice.fragment.children.len(), 2);
    }

    #[test]
    fn from_text_bare_cr_treated_as_lf() {
        let slice = Slice::from_text("a\rb");
        assert_eq!(slice.fragment.children.len(), 1);
        let p = &slice.fragment.children[0];
        assert_eq!(p.children.len(), 3);
    }
}
