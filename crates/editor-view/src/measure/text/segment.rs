use std::ops::Range;

use editor_model::{Node, NodeRef};

/// One contiguous run of paragraph children that share a single parley layout
/// (or one strut-only line, for `Empty`). Produced by [`split_into_segments`]
/// by splitting a paragraph's children at every `HardBreak`.
///
/// `child_range` is the paragraph-offset interval this segment owns. Matching
/// against a cursor's `Position::offset` is inclusive of both endpoints
/// (`start <= offset && offset <= end`, not `Range::contains`). A `hard_break`
/// is the selectable gap between adjacent segments, not part of either
/// segment's range.
pub(crate) enum Segment<'a> {
    Text {
        children: Vec<NodeRef<'a>>,
        child_range: Range<usize>,
    },
    Empty {
        child_range: Range<usize>,
    },
}

impl<'a> Segment<'a> {
    pub(crate) fn child_range(&self) -> &Range<usize> {
        match self {
            Segment::Text { child_range, .. } | Segment::Empty { child_range } => child_range,
        }
    }
}

/// Splits a paragraph's children into segments at every `hard_break`.
///
/// Invariants (debug-asserted):
/// - The returned vec is non-empty.
/// - Number of segments equals (number of `hard_break` children) + 1, except
///   when the paragraph ends with `hard_break` an additional trailing `Empty`
///   segment is appended (so the user-visible "cursor after trailing
///   hard_break" position has a line to anchor on).
pub(crate) fn split_into_segments<'a>(paragraph: &NodeRef<'a>) -> Vec<Segment<'a>> {
    let mut segments: Vec<Segment<'a>> = Vec::new();
    let mut buf: Vec<NodeRef<'a>> = Vec::new();
    let mut seg_start: usize = 0;
    let mut last_was_hard_break = false;
    let mut child_count: usize = 0;

    for (idx, child) in paragraph.children().enumerate() {
        child_count = idx + 1;
        match child.node() {
            Node::HardBreak(_) => {
                if buf.is_empty() {
                    segments.push(Segment::Empty {
                        child_range: seg_start..seg_start,
                    });
                } else {
                    segments.push(Segment::Text {
                        children: std::mem::take(&mut buf),
                        child_range: seg_start..idx,
                    });
                }
                seg_start = idx + 1;
                last_was_hard_break = true;
            }
            Node::Text(_) | Node::Tab(_) => {
                buf.push(child);
                last_was_hard_break = false;
            }
            _ => {
                // PageBreak or any other non-flow inline node: skip from the
                // text flow (preserves the current behavior of the old
                // collect_text_runs).
                last_was_hard_break = false;
            }
        }
    }

    if child_count == 0 {
        segments.push(Segment::Empty { child_range: 0..0 });
    } else if last_was_hard_break {
        segments.push(Segment::Empty {
            child_range: seg_start..seg_start,
        });
    } else {
        segments.push(Segment::Text {
            children: buf,
            child_range: seg_start..child_count,
        });
    }

    debug_assert!(!segments.is_empty());
    for seg in &segments {
        debug_assert!(seg.child_range().start <= seg.child_range().end);
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::doc;

    fn variants(segments: &[Segment<'_>]) -> Vec<(&'static str, Range<usize>)> {
        segments
            .iter()
            .map(|s| match s {
                Segment::Text { child_range, .. } => ("text", child_range.clone()),
                Segment::Empty { child_range } => ("empty", child_range.clone()),
            })
            .collect()
    }

    #[test]
    fn empty_paragraph_yields_single_empty_segment() {
        let (doc, p1) = doc! { root { p1: paragraph } };
        let p = doc.node(p1).unwrap();
        let segs = split_into_segments(&p);
        assert_eq!(variants(&segs), vec![("empty", 0..0)]);
    }

    #[test]
    fn single_text_paragraph_yields_single_text_segment() {
        let (doc, p1) = doc! { root { p1: paragraph { text("hello") } } };
        let p = doc.node(p1).unwrap();
        let segs = split_into_segments(&p);
        assert_eq!(variants(&segs), vec![("text", 0..1)]);
    }

    #[test]
    fn single_hard_break_yields_two_empties() {
        let (doc, p1) = doc! { root { p1: paragraph { hard_break } } };
        let p = doc.node(p1).unwrap();
        let segs = split_into_segments(&p);
        assert_eq!(variants(&segs), vec![("empty", 0..0), ("empty", 1..1)]);
    }

    #[test]
    fn text_then_hard_break_yields_text_then_empty() {
        let (doc, p1) = doc! { root { p1: paragraph { text("a") hard_break } } };
        let p = doc.node(p1).unwrap();
        let segs = split_into_segments(&p);
        assert_eq!(variants(&segs), vec![("text", 0..1), ("empty", 2..2)]);
    }

    #[test]
    fn hard_break_then_text_yields_empty_then_text() {
        let (doc, p1) = doc! { root { p1: paragraph { hard_break text("a") } } };
        let p = doc.node(p1).unwrap();
        let segs = split_into_segments(&p);
        assert_eq!(variants(&segs), vec![("empty", 0..0), ("text", 1..2)]);
    }

    #[test]
    fn text_break_text_yields_two_text_segments() {
        let (doc, p1) = doc! { root { p1: paragraph { text("hel") hard_break text("lo") } } };
        let p = doc.node(p1).unwrap();
        let segs = split_into_segments(&p);
        assert_eq!(variants(&segs), vec![("text", 0..1), ("text", 2..3)]);
    }

    #[test]
    fn consecutive_hard_breaks_yield_three_segments() {
        let (doc, p1) = doc! {
            root { p1: paragraph { text("a") hard_break hard_break text("b") } }
        };
        let p = doc.node(p1).unwrap();
        let segs = split_into_segments(&p);
        assert_eq!(
            variants(&segs),
            vec![("text", 0..1), ("empty", 2..2), ("text", 3..4)]
        );
    }

    #[test]
    fn tab_stays_in_text_segment() {
        let (doc, p1) = doc! { root { p1: paragraph { text("a") tab text("b") } } };
        let p = doc.node(p1).unwrap();
        let segs = split_into_segments(&p);
        assert_eq!(variants(&segs), vec![("text", 0..3)]);
        match &segs[0] {
            Segment::Text { children, .. } => assert_eq!(children.len(), 3),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn segments_carry_text_children_in_order() {
        let (doc, p1) =
            doc! { root { p1: paragraph { text("a") text("b") hard_break text("c") } } };
        let p = doc.node(p1).unwrap();
        let segs = split_into_segments(&p);
        match &segs[0] {
            Segment::Text { children, .. } => {
                assert_eq!(children.len(), 2);
            }
            _ => panic!("expected Text"),
        }
        match &segs[1] {
            Segment::Text { children, .. } => {
                assert_eq!(children.len(), 1);
            }
            _ => panic!("expected Text"),
        }
    }
}
