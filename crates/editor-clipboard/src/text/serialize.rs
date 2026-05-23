use crate::slice::Slice;
use editor_model::{Fragment, PlainNode};

pub fn to_text(slice: &Slice) -> String {
    let mut out = String::new();
    walk(&slice.fragment, &mut out);
    out
}

fn walk(fragment: &Fragment, out: &mut String) {
    match &fragment.node {
        PlainNode::Text(t) => out.push_str(&t.text),
        PlainNode::HardBreak(_) => out.push('\n'),
        _ => {
            let is_block = is_block_node(&fragment.node);
            let first_in_block = is_block && !out.is_empty() && !out.ends_with("\n\n");
            if first_in_block {
                if out.ends_with('\n') {
                    out.push('\n');
                } else {
                    out.push_str("\n\n");
                }
            }
            for child in &fragment.children {
                walk(child, out);
            }
        }
    }
}

fn is_block_node(n: &PlainNode) -> bool {
    !matches!(n, PlainNode::Text(_) | PlainNode::HardBreak(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::Slice;
    use editor_macros::state;

    #[test]
    fn to_text_single_paragraph() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0) -> (t1, 11)
        };
        let slice = Slice::extract(&s).unwrap();
        assert_eq!(to_text(&slice), "Hello World");
    }

    #[test]
    fn to_text_multi_paragraph_with_hardbreak() {
        let (s, ..) = state! {
            doc { root {
                paragraph { t1: text("first") hard_break {} text("line") }
                paragraph { t2: text("second") }
            } }
            selection: (t1, 0) -> (t2, 6)
        };
        let slice = Slice::extract(&s).unwrap();
        assert_eq!(slice.to_text(), "first\nline\n\nsecond");
    }
}
