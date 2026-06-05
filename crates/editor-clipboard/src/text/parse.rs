use editor_model::{
    Fragment, PlainHardBreakNode, PlainNode, PlainParagraphNode, PlainRootNode, PlainTabNode,
    PlainTextNode,
};

use crate::slice::Slice;

pub fn from_text(text: &str) -> Slice {
    let stripped = text.trim_start_matches('\u{feff}');
    let normalized = stripped.replace("\r\n", "\n").replace('\r', "\n");
    let children: Vec<Fragment> = if normalized.is_empty() {
        Vec::new()
    } else {
        normalized.split("\n\n").map(paragraph_from_block).collect()
    };
    let open_depth = u32::from(!children.is_empty());

    Slice {
        fragment: Fragment {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: vec![],
            style: None,
            children,
        },
        open_start: open_depth,
        open_end: open_depth,
    }
}

fn push_line_with_tabs(line: &str, inline: &mut Vec<Fragment>) {
    let mut first = true;
    for segment in line.split('\t') {
        if !first {
            inline.push(Fragment::leaf(PlainNode::Tab(PlainTabNode::default())));
        }
        if !segment.is_empty() {
            inline.push(Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: segment.to_string(),
            })));
        }
        first = false;
    }
}

fn paragraph_from_block(block: &str) -> Fragment {
    Fragment {
        node: PlainNode::Paragraph(PlainParagraphNode::default()),
        modifiers: vec![],
        style: None,
        children: inline_from_block(block),
    }
}

fn inline_from_block(block: &str) -> Vec<Fragment> {
    let mut inline = Vec::new();
    let mut first = true;
    for line in block.split('\n') {
        if !first {
            inline.push(Fragment::leaf(PlainNode::HardBreak(
                PlainHardBreakNode::default(),
            )));
        }
        push_line_with_tabs(line, &mut inline);
        first = false;
    }
    inline
}

#[cfg(test)]
mod tests {
    use super::*;

    fn only_paragraph(slice: &Slice) -> &Fragment {
        assert!(matches!(slice.fragment.node, PlainNode::Root(_)));
        assert_eq!(slice.fragment.children.len(), 1);
        let paragraph = &slice.fragment.children[0];
        assert!(matches!(paragraph.node, PlainNode::Paragraph(_)));
        paragraph
    }

    #[test]
    fn from_text_empty_is_empty_slice() {
        let slice = from_text("");
        assert!(matches!(slice.fragment.node, PlainNode::Root(_)));
        assert!(slice.fragment.children.is_empty());
        assert_eq!(slice.open_start, 0);
        assert_eq!(slice.open_end, 0);
    }

    #[test]
    fn from_text_single_block_is_open_paragraph_slice() {
        let slice = from_text("Hello World");
        let paragraph = only_paragraph(&slice);
        assert_eq!(paragraph.children.len(), 1);
        if let PlainNode::Text(t) = &paragraph.children[0].node {
            assert_eq!(t.text, "Hello World");
        } else {
            panic!("expected text");
        }
        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
    }

    #[test]
    fn from_text_double_newline_splits_paragraph() {
        let slice = Slice::from_text("first\n\nsecond");
        assert_eq!(slice.fragment.children.len(), 2);
        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
    }

    #[test]
    fn from_text_single_newline_hardbreak() {
        let slice = Slice::from_text("line1\nline2");
        let paragraph = only_paragraph(&slice);
        assert_eq!(paragraph.children.len(), 3);
        assert!(matches!(
            paragraph.children[1].node,
            PlainNode::HardBreak(_)
        ));
        assert_eq!(slice.open_start, 1);
        assert_eq!(slice.open_end, 1);
    }

    #[test]
    fn from_text_strips_bom() {
        let slice = Slice::from_text("\u{feff}hello");
        let paragraph = only_paragraph(&slice);
        if let PlainNode::Text(t) = &paragraph.children[0].node {
            assert_eq!(t.text, "hello");
        } else {
            panic!("expected text");
        }
    }

    #[test]
    fn from_text_crlf_normalizes_to_lf() {
        let slice = Slice::from_text("a\r\nb");
        let paragraph = only_paragraph(&slice);
        assert_eq!(paragraph.children.len(), 3);
        if let PlainNode::Text(t) = &paragraph.children[0].node {
            assert_eq!(t.text, "a");
        } else {
            panic!("expected text");
        }
        assert!(matches!(
            paragraph.children[1].node,
            PlainNode::HardBreak(_)
        ));
        if let PlainNode::Text(t) = &paragraph.children[2].node {
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
        let paragraph = only_paragraph(&slice);
        assert_eq!(paragraph.children.len(), 3);
        assert!(matches!(
            paragraph.children[1].node,
            PlainNode::HardBreak(_)
        ));
    }

    #[test]
    fn from_text_tab_becomes_tab_node() {
        let slice = Slice::from_text("a\tb");
        let paragraph = only_paragraph(&slice);
        assert_eq!(paragraph.children.len(), 3);
        if let PlainNode::Text(t) = &paragraph.children[0].node {
            assert_eq!(t.text, "a");
        } else {
            panic!("expected text");
        }
        assert!(matches!(paragraph.children[1].node, PlainNode::Tab(_)));
        if let PlainNode::Text(t) = &paragraph.children[2].node {
            assert_eq!(t.text, "b");
        } else {
            panic!("expected text");
        }
    }

    #[test]
    fn from_text_leading_tab_becomes_tab_node() {
        let slice = Slice::from_text("\tindented");
        let paragraph = only_paragraph(&slice);
        assert!(matches!(paragraph.children[0].node, PlainNode::Tab(_)));
    }
}
