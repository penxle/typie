use editor_common::StrExt;
use editor_model::{Doc, Node, NodeId};
use std::ops::Range;

use super::class::{FlatClass, classify};
use super::segment::FlatSegment;

pub trait DocFlatExt {
    fn flat_segments(&self) -> FlatSegments<'_>;
    fn flat_size(&self) -> usize;
    fn flat_text(&self, range: Range<usize>) -> String;
}

pub struct FlatSegments<'a> {
    inner: std::vec::IntoIter<(usize, FlatSegment<'a>)>,
}

impl<'a> Iterator for FlatSegments<'a> {
    type Item = (usize, FlatSegment<'a>);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl DocFlatExt for Doc {
    fn flat_segments(&self) -> FlatSegments<'_> {
        let mut segments = Vec::new();
        let mut cumulative = 0usize;
        visit_flat(self, NodeId::ROOT, &mut cumulative, &mut segments);
        FlatSegments {
            inner: segments.into_iter(),
        }
    }

    fn flat_size(&self) -> usize {
        self.flat_segments()
            .last()
            .map(|(start, seg)| start + seg.size())
            .unwrap_or(0)
    }

    fn flat_text(&self, range: Range<usize>) -> String {
        let mut out = String::new();
        for (seg_start, seg) in self.flat_segments() {
            let seg_end = seg_start + seg.size();
            if seg_end <= range.start {
                continue;
            }
            if seg_start >= range.end {
                break;
            }
            let local_start = range.start.saturating_sub(seg_start);
            let local_end = (range.end - seg_start).min(seg.size());
            let s = seg.as_str();
            let mut chars = s.chars();
            for _ in 0..local_start {
                chars.next();
            }
            for _ in local_start..local_end {
                if let Some(c) = chars.next() {
                    out.push(c);
                }
            }
        }
        out
    }
}

fn visit_flat<'a>(
    doc: &'a Doc,
    node_id: NodeId,
    cumulative: &mut usize,
    out: &mut Vec<(usize, FlatSegment<'a>)>,
) {
    let entry = match doc.get_entry(node_id) {
        Some(e) => e,
        None => return,
    };
    let mut prev_block = false;

    for &child_id in &entry.children {
        let child = match doc.get_entry(child_id) {
            Some(c) => c,
            None => continue,
        };
        let class = classify(&child.node);
        let is_block = matches!(class, FlatClass::Container | FlatClass::Atom);

        if is_block && prev_block {
            out.push((*cumulative, FlatSegment::Break { node_id: child_id }));
            *cumulative += 1;
        }

        match class {
            FlatClass::Text => {
                let text = match &child.node {
                    Node::Text(t) => t.text.as_str(),
                    _ => unreachable!("classified as Text"),
                };
                out.push((
                    *cumulative,
                    FlatSegment::Text {
                        node_id: child_id,
                        text,
                    },
                ));
                *cumulative += text.char_count();
            }
            FlatClass::Break => {
                out.push((*cumulative, FlatSegment::Break { node_id: child_id }));
                *cumulative += 1;
            }
            FlatClass::Atom => {
                out.push((*cumulative, FlatSegment::Atom { node_id: child_id }));
                *cumulative += 1;
            }
            FlatClass::Container => {
                visit_flat(doc, child_id, cumulative, out);
            }
        }

        prev_block = is_block;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::doc;

    #[test]
    fn flat_size_empty_doc() {
        let (doc, ..) = doc! { root {} };
        assert_eq!(doc.flat_size(), 0);
    }

    #[test]
    fn flat_size_single_paragraph() {
        let (doc, ..) = doc! { root { paragraph { text("hello") } } };
        assert_eq!(doc.flat_size(), 5);
    }

    #[test]
    fn flat_size_multiple_paragraphs_with_separators() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("ab") }
                paragraph { text("cd") }
            }
        };
        // "ab" + "\n" + "cd" = 5
        assert_eq!(doc.flat_size(), 5);
    }

    #[test]
    fn flat_size_hard_break_adds_newline() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") hard_break {} text("b") }
            }
        };
        // "a" + "\n" + "b" = 3
        assert_eq!(doc.flat_size(), 3);
    }

    #[test]
    fn flat_text_extracts_full_doc() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("hello") }
                paragraph { text("world") }
            }
        };
        assert_eq!(doc.flat_text(0..doc.flat_size()), "hello\nworld");
    }

    #[test]
    fn flat_text_extracts_partial_range() {
        let (doc, ..) = doc! {
            root { paragraph { text("hello world") } }
        };
        assert_eq!(doc.flat_text(3..8), "lo wo");
    }

    #[test]
    fn flat_text_clamps_to_bounds() {
        let (doc, ..) = doc! { root { paragraph { text("hi") } } };
        assert_eq!(doc.flat_text(0..100), "hi");
    }

    #[test]
    fn flat_text_empty_range() {
        let (doc, ..) = doc! { root { paragraph { text("hi") } } };
        assert_eq!(doc.flat_text(1..1), "");
    }

    #[test]
    fn flat_text_unicode_respects_char_boundaries() {
        let (doc, ..) = doc! { root { paragraph { text("한글abc") } } };
        assert_eq!(doc.flat_text(0..2), "한글");
        assert_eq!(doc.flat_text(2..5), "abc");
    }

    #[test]
    fn flat_size_includes_hard_rule_as_atom() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                horizontal_rule {}
                paragraph { text("b") }
            }
        };
        // "a" + "\n" + "\u{fffc}" + "\n" + "b" = 5
        assert_eq!(doc.flat_size(), 5);
        assert_eq!(doc.flat_text(0..5), "a\n\u{fffc}\nb");
    }

    #[test]
    fn flat_text_range_at_block_boundary() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("ab") }
                paragraph { text("cd") }
            }
        };
        // "ab\ncd": a=0, b=1, \n=2, c=3, d=4
        assert_eq!(doc.flat_text(1..3), "b\n");
        assert_eq!(doc.flat_text(2..4), "\nc");
        assert_eq!(doc.flat_text(2..3), "\n");
    }
}
