use editor_model::{Doc, FlatKind, Node};
use std::ops::Range;

use super::segment::FlatSegment;

pub trait DocFlatExt {
    fn flat_segments(&self) -> FlatSegments;
    fn flat_size(&self) -> usize;
    fn flat_text(&self, range: Range<usize>) -> String;
}

pub struct FlatSegments {
    inner: std::vec::IntoIter<(usize, FlatSegment)>,
}

impl Iterator for FlatSegments {
    type Item = (usize, FlatSegment);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl DocFlatExt for Doc {
    fn flat_segments(&self) -> FlatSegments {
        let layout = self.flat_layout();
        let segments: Vec<(usize, FlatSegment)> = layout
            .entries
            .iter()
            .map(|e| {
                let seg = match e.kind {
                    FlatKind::Text => FlatSegment::Text {
                        node_id: e.node_id,
                        text: match self.get_entry(e.node_id).map(|n| &n.node) {
                            Some(Node::Text(t)) => t.text.to_string(),
                            _ => String::new(),
                        },
                    },
                    FlatKind::Break => FlatSegment::Break { node_id: e.node_id },
                    FlatKind::Atom => FlatSegment::Atom { node_id: e.node_id },
                    FlatKind::Open => FlatSegment::Open { node_id: e.node_id },
                    FlatKind::Close => FlatSegment::Close { node_id: e.node_id },
                };
                (e.start, seg)
            })
            .collect();
        FlatSegments {
            inner: segments.into_iter(),
        }
    }

    fn flat_size(&self) -> usize {
        self.flat_layout().size
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
        // Open(p) + "hello" + Close(p) = 1 + 5 + 1 = 7
        assert_eq!(doc.flat_size(), 7);
    }

    #[test]
    fn flat_size_multiple_paragraphs() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("ab") }
                paragraph { text("cd") }
            }
        };
        // Open(p1) + "ab" + Close(p1) + Open(p2) + "cd" + Close(p2) = 4 + 4 = 8
        assert_eq!(doc.flat_size(), 8);
    }

    #[test]
    fn flat_size_hard_break() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") hard_break {} text("b") }
            }
        };
        // Open(p) + "a" + Break + "b" + Close(p) = 1+1+1+1+1 = 5
        assert_eq!(doc.flat_size(), 5);
    }

    #[test]
    fn flat_size_empty_paragraph() {
        let (doc, ..) = doc! { root { paragraph {} } };
        // Open(p) + Close(p) = 2
        assert_eq!(doc.flat_size(), 2);
    }

    #[test]
    fn flat_size_empty_blockquote() {
        let (doc, ..) = doc! { root { blockquote { paragraph {} } } };
        // Open(bq) + Open(p) + Close(p) + Close(bq) = 4
        assert_eq!(doc.flat_size(), 4);
    }

    #[test]
    fn flat_size_nested_with_text() {
        let (doc, ..) = doc! { root { blockquote { paragraph { text("hi") } } } };
        // Open(bq) + Open(p) + "hi" + Close(p) + Close(bq) = 1+1+2+1+1 = 6
        assert_eq!(doc.flat_size(), 6);
    }

    #[test]
    fn flat_size_multiple_nested_paragraphs() {
        let (doc, ..) = doc! {
            root {
                blockquote {
                    paragraph { text("a") }
                    paragraph { text("b") }
                }
            }
        };
        // Open(bq) + [Open(p1)+"a"+Close(p1)] + [Open(p2)+"b"+Close(p2)] + Close(bq)
        // = 1 + 3 + 3 + 1 = 8
        assert_eq!(doc.flat_size(), 8);
    }

    #[test]
    fn flat_text_extracts_full_doc() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("hello") }
                paragraph { text("world") }
            }
        };
        let expected = format!(
            "{open}hello{close}{open}world{close}",
            open = "\u{2028}",
            close = "\u{2029}"
        );
        assert_eq!(doc.flat_text(0..doc.flat_size()), expected);
    }

    #[test]
    fn flat_text_extracts_partial_range() {
        let (doc, ..) = doc! { root { paragraph { text("hello world") } } };
        // Open(p)=0, "hello world"=1..12, Close(p)=12
        assert_eq!(doc.flat_text(4..9), "lo wo");
    }

    #[test]
    fn flat_text_clamps_to_bounds() {
        let (doc, ..) = doc! { root { paragraph { text("hi") } } };
        // Open=0, "hi"=1..3, Close=3; total=4
        assert_eq!(doc.flat_text(0..100), "\u{2028}hi\u{2029}");
    }

    #[test]
    fn flat_text_empty_range() {
        let (doc, ..) = doc! { root { paragraph { text("hi") } } };
        assert_eq!(doc.flat_text(1..1), "");
    }

    #[test]
    fn flat_text_unicode_respects_char_boundaries() {
        let (doc, ..) = doc! { root { paragraph { text("한글abc") } } };
        // Open=0, "한글abc"=1..6, Close=6
        assert_eq!(doc.flat_text(1..3), "한글");
        assert_eq!(doc.flat_text(3..6), "abc");
    }

    #[test]
    fn flat_size_includes_horizontal_rule_as_atom() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                horizontal_rule {}
                paragraph { text("b") }
            }
        };
        // Open(p1)+"a"+Close(p1) + Atom(hr) + Open(p2)+"b"+Close(p2) = 3+1+3 = 7
        assert_eq!(doc.flat_size(), 7);
        let o = "\u{2028}";
        let c = "\u{2029}";
        let expected = format!("{o}a{c}\u{FFFC}{o}b{c}");
        assert_eq!(doc.flat_text(0..7), expected);
    }

    #[test]
    fn flat_text_token_chars_at_boundaries() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("ab") }
                paragraph { text("cd") }
            }
        };
        // ⟨ab⟩⟨cd⟩ → positions: O=0 a=1 b=2 C=3 O=4 c=5 d=6 C=7
        assert_eq!(doc.flat_text(2..6), "b\u{2029}\u{2028}c");
        assert_eq!(doc.flat_text(3..5), "\u{2029}\u{2028}");
    }

    #[test]
    fn flat_segments_empty_blockquote() {
        let (doc, bq, p) = doc! { root { bq: blockquote { p: paragraph {} } } };
        let segments: Vec<_> = doc.flat_segments().collect();
        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0], (0, FlatSegment::Open { node_id: bq }));
        assert_eq!(segments[1], (1, FlatSegment::Open { node_id: p }));
        assert_eq!(segments[2], (2, FlatSegment::Close { node_id: p }));
        assert_eq!(segments[3], (3, FlatSegment::Close { node_id: bq }));
    }

    #[test]
    fn flat_segments_no_block_separators() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                paragraph { text("b") }
            }
        };
        let segments: Vec<_> = doc.flat_segments().collect();
        assert_eq!(segments.len(), 6);
        assert!(
            segments
                .iter()
                .all(|(_, seg)| !matches!(seg, FlatSegment::Break { .. }))
        );
    }

    #[test]
    fn flat_text_empty_blockquote() {
        let (doc, ..) = doc! { root { blockquote { paragraph {} } } };
        assert_eq!(doc.flat_text(0..4), "\u{2028}\u{2028}\u{2029}\u{2029}");
    }

    #[test]
    fn flat_size_adjacent_blockquotes() {
        let (doc, ..) = doc! {
            root {
                blockquote { paragraph { text("a") } }
                blockquote { paragraph { text("b") } }
            }
        };
        // Each bq: Open(bq)+Open(p)+"x"+Close(p)+Close(bq) = 5
        // Total: 5 + 5 = 10
        assert_eq!(doc.flat_size(), 10);
    }
}
