use editor_common::StrExt;
use editor_model::{Doc, Node, NodeId};

use super::class::{FlatClass, classify};
use crate::{Position, ResolvedPosition};

pub trait ResolvedPositionFlatExt<'a>: Sized {
    fn to_flat(&self) -> usize;
    fn from_flat(doc: &'a Doc, flat: usize) -> Option<Self>;
}

impl<'a> ResolvedPositionFlatExt<'a> for ResolvedPosition<'a> {
    fn to_flat(&self) -> usize {
        to_flat_walk(self.doc(), NodeId::ROOT, self.node_id(), self.offset()).unwrap_or(0)
    }

    fn from_flat(doc: &'a Doc, flat: usize) -> Option<Self> {
        let pos = from_flat_walk(doc, NodeId::ROOT, 0, flat)?;
        pos.resolve(doc)
    }
}

fn to_flat_walk(doc: &Doc, current: NodeId, target: NodeId, target_offset: usize) -> Option<usize> {
    if current == target {
        let entry = doc.get_entry(current)?;
        return Some(match &entry.node {
            Node::Text(_) => target_offset,
            _ => {
                let mut acc = 0usize;
                for (i, &child_id) in entry.children.iter().enumerate() {
                    if i == target_offset {
                        return Some(acc);
                    }
                    acc += subtree_flat_size(doc, child_id);
                }
                acc
            }
        });
    }

    let entry = doc.get_entry(current)?;
    let mut acc = 0usize;

    for &child_id in &entry.children {
        let child = doc.get_entry(child_id)?;
        let class = classify(&child.node);

        match class {
            FlatClass::Container => {
                acc += 1;

                if child_id == target {
                    return to_flat_walk(doc, child_id, target, target_offset)
                        .map(|inner| acc + inner);
                }
                if let Some(inner) = to_flat_walk(doc, child_id, target, target_offset) {
                    return Some(acc + inner);
                }

                acc += subtree_flat_size(doc, child_id) - 2;
                acc += 1;
            }
            FlatClass::Text => {
                if child_id == target {
                    return Some(acc + target_offset);
                }
                acc += subtree_flat_size(doc, child_id);
            }
            FlatClass::Break | FlatClass::Atom => {
                if child_id == target {
                    return Some(acc);
                }
                acc += 1;
            }
        }
    }

    None
}

fn subtree_flat_size(doc: &Doc, node_id: NodeId) -> usize {
    let entry = match doc.get_entry(node_id) {
        Some(e) => e,
        None => return 0,
    };
    match classify(&entry.node) {
        FlatClass::Text => match &entry.node {
            Node::Text(t) => t.text.char_count(),
            _ => 0,
        },
        FlatClass::Break | FlatClass::Atom => 1,
        FlatClass::Container => {
            2 + entry
                .children
                .iter()
                .map(|&child_id| subtree_flat_size(doc, child_id))
                .sum::<usize>()
        }
    }
}

fn from_flat_walk(
    doc: &Doc,
    container: NodeId,
    start_flat: usize,
    target: usize,
) -> Option<Position> {
    let entry = doc.get_entry(container)?;
    let mut acc = start_flat;

    for (i, &child_id) in entry.children.iter().enumerate() {
        let child = doc.get_entry(child_id)?;
        let class = classify(&child.node);

        match class {
            FlatClass::Container => {
                let child_size = subtree_flat_size(doc, child_id);
                let content_size = child_size - 2;

                if target == acc {
                    return Some(Position::new(container, i));
                }
                acc += 1;

                // Content range (inclusive): text end == close token position maps into content
                if target >= acc && target <= acc + content_size {
                    return from_flat_walk(doc, child_id, acc, target);
                }
                acc += content_size;

                // Close token for empty containers (content_size == 0 means no content range
                // was entered above; this handles the container-end sentinel)
                if target == acc {
                    return Some(Position::new(child_id, child.children.len()));
                }
                acc += 1;
            }
            FlatClass::Text => {
                let text_size = match &child.node {
                    Node::Text(t) => t.text.char_count(),
                    _ => 0,
                };
                if target >= acc && target <= acc + text_size {
                    return Some(Position::new(child_id, target - acc));
                }
                acc += text_size;
            }
            FlatClass::Break | FlatClass::Atom => {
                if target == acc {
                    return Some(Position::new(container, i));
                }
                acc += 1;
            }
        }
    }

    if target == acc {
        return Some(Position::new(container, entry.children.len()));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::doc;

    #[test]
    fn to_flat_text_node_offset_zero() {
        let (doc, t1, ..) = doc! { root { paragraph { t1: text("hello") } } };
        let resolved = Position::new(t1, 0).resolve(&doc).unwrap();
        // Open(p)=0, text starts at 1
        assert_eq!(resolved.to_flat(), 1);
    }

    #[test]
    fn to_flat_text_node_middle_offset() {
        let (doc, t1, ..) = doc! { root { paragraph { t1: text("hello") } } };
        let resolved = Position::new(t1, 3).resolve(&doc).unwrap();
        // Open(p)=0, text[3] at 1+3=4
        assert_eq!(resolved.to_flat(), 4);
    }

    #[test]
    fn to_flat_across_paragraphs() {
        let (doc, t2) = doc! {
            root {
                paragraph { text("hello") }
                paragraph { t2: text("world") }
            }
        };
        // P1: Open=0, "hello"=1..6, Close=6 (size=7)
        // P2: Open=7, "world"=8..13, Close=13
        // t2 offset 2 → 8+2 = 10
        let resolved = Position::new(t2, 2).resolve(&doc).unwrap();
        assert_eq!(resolved.to_flat(), 10);
    }

    #[test]
    fn to_flat_container_position_start() {
        let (doc, p2) = doc! {
            root {
                paragraph { text("abc") }
                p2: paragraph { text("de") }
            }
        };
        // P1 size=5 (Open+"abc"+Close), P2 starts at 5
        // Position(p2, 0) = inside p2 before children = after Open(p2) = 5+1 = 6
        let resolved = Position::new(p2, 0).resolve(&doc).unwrap();
        assert_eq!(resolved.to_flat(), 6);
    }

    #[test]
    fn to_flat_container_position_end() {
        let (doc, p1, ..) = doc! {
            root { p1: paragraph { text("hello") } }
        };
        // Position(p1, 1) = after first child (text node) = Open + "hello" = 1+5 = 6
        let resolved = Position::new(p1, 1).resolve(&doc).unwrap();
        assert_eq!(resolved.to_flat(), 6);
    }

    #[test]
    fn to_flat_nested_empty_paragraph() {
        let (doc, p) = doc! { root { blockquote { p: paragraph {} } } };
        // Open(bq)=0, Open(p)=1 → Position(p, 0) = after Open(p) = 2
        let resolved = Position::new(p, 0).resolve(&doc).unwrap();
        assert_eq!(resolved.to_flat(), 2);
    }

    #[test]
    fn to_flat_root_offset() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                paragraph { text("b") }
            }
        };
        // Position(root, 0) = 0
        let resolved = Position::new(NodeId::ROOT, 0).resolve(&doc).unwrap();
        assert_eq!(resolved.to_flat(), 0);

        // Position(root, 1) = after P1 (size=3) = 3
        let resolved = Position::new(NodeId::ROOT, 1).resolve(&doc).unwrap();
        assert_eq!(resolved.to_flat(), 3);

        // Position(root, 2) = after P1+P2 = 3+3 = 6
        let resolved = Position::new(NodeId::ROOT, 2).resolve(&doc).unwrap();
        assert_eq!(resolved.to_flat(), 6);
    }

    #[test]
    fn from_flat_at_open_token() {
        let (doc, ..) = doc! { root { paragraph { text("hello") } } };
        // flat=0 is Open(p) → Position(root, 0)
        let resolved = ResolvedPosition::from_flat(&doc, 0).unwrap();
        assert_eq!(resolved.node_id(), NodeId::ROOT);
        assert_eq!(resolved.offset(), 0);
    }

    #[test]
    fn from_flat_inside_text() {
        let (doc, t1, ..) = doc! { root { paragraph { t1: text("hello") } } };
        // flat=1 is start of text → Position(t1, 0)
        let resolved = ResolvedPosition::from_flat(&doc, 1).unwrap();
        assert_eq!(resolved.node_id(), t1);
        assert_eq!(resolved.offset(), 0);

        // flat=4 → Position(t1, 3)
        let resolved = ResolvedPosition::from_flat(&doc, 4).unwrap();
        assert_eq!(resolved.node_id(), t1);
        assert_eq!(resolved.offset(), 3);
    }

    #[test]
    fn from_flat_at_close_token() {
        let (doc, t1, ..) = doc! { root { paragraph { t1: text("hello") } } };
        // flat=6 is both text end and Close(p): text range [1..6] inclusive wins → Position(t1, 5)
        let resolved = ResolvedPosition::from_flat(&doc, 6).unwrap();
        assert_eq!(resolved.node_id(), t1);
        assert_eq!(resolved.offset(), 5);
    }

    #[test]
    fn from_flat_after_close_is_next_open() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                paragraph { text("b") }
            }
        };
        // P1: O=0 a=1 C=2, P2: O=3 b=4 C=5
        // flat=3 is Open(p2) → Position(root, 1)
        let resolved = ResolvedPosition::from_flat(&doc, 3).unwrap();
        assert_eq!(resolved.node_id(), NodeId::ROOT);
        assert_eq!(resolved.offset(), 1);
    }

    #[test]
    fn from_flat_nested_empty_paragraph() {
        let (doc, p) = doc! { root { blockquote { p: paragraph {} } } };
        // O(bq)=0, O(p)=1, C(p)=2, C(bq)=3
        // flat=2 is Close(p) → Position(p, 0) (empty paragraph, children.len()=0)
        let resolved = ResolvedPosition::from_flat(&doc, 2).unwrap();
        assert_eq!(resolved.node_id(), p);
        assert_eq!(resolved.offset(), 0);
    }

    #[test]
    fn from_flat_out_of_range_returns_none() {
        let (doc, ..) = doc! { root { paragraph { text("hi") } } };
        // flat_size = 4, so flat=100 is invalid
        assert!(ResolvedPosition::from_flat(&doc, 100).is_none());
    }

    #[test]
    fn from_flat_end_of_doc() {
        let (doc, ..) = doc! { root { paragraph { text("hi") } } };
        // flat_size = 4, flat=4 → Position(root, 1)
        let resolved = ResolvedPosition::from_flat(&doc, 4).unwrap();
        assert_eq!(resolved.node_id(), NodeId::ROOT);
        assert_eq!(resolved.offset(), 1);
    }

    #[test]
    fn to_flat_from_flat_roundtrip_text_positions() {
        let (doc, t1, ..) = doc! { root { paragraph { t1: text("hello world") } } };
        for offset in 0..=11 {
            let pos = Position::new(t1, offset).resolve(&doc).unwrap();
            let flat = pos.to_flat();
            let back = ResolvedPosition::from_flat(&doc, flat).unwrap();
            assert_eq!(back.node_id(), t1, "at offset {offset}");
            assert_eq!(back.offset(), offset, "at offset {offset}");
        }
    }

    #[test]
    fn roundtrip_nested_container() {
        let (doc, p) = doc! { root { blockquote { p: paragraph {} } } };
        // Position(p, 0) → flat=2 → back to Position(p, 0)
        let pos = Position::new(p, 0).resolve(&doc).unwrap();
        let flat = pos.to_flat();
        assert_eq!(flat, 2);
        let back = ResolvedPosition::from_flat(&doc, flat).unwrap();
        assert_eq!(back.node_id(), p);
        assert_eq!(back.offset(), 0);
    }

    #[test]
    fn roundtrip_deeply_nested() {
        let (doc, t1) = doc! {
            root { blockquote { callout { paragraph { t1: text("x") } } } }
        };
        // O(bq)=0, O(callout)=1, O(p)=2, "x"=3, C(p)=4, C(callout)=5, C(bq)=6
        let pos = Position::new(t1, 0).resolve(&doc).unwrap();
        let flat = pos.to_flat();
        assert_eq!(flat, 3);
        let back = ResolvedPosition::from_flat(&doc, flat).unwrap();
        assert_eq!(back.node_id(), t1);
        assert_eq!(back.offset(), 0);
    }

    #[test]
    fn from_flat_across_paragraphs_lands_in_second() {
        let (doc, t2) = doc! {
            root {
                paragraph { text("abc") }
                paragraph { t2: text("de") }
            }
        };
        // P1: O=0 a=1 b=2 c=3 C=4, P2: O=5 d=6 e=7 C=8
        // flat=6 → Position(t2, 0)
        let resolved = ResolvedPosition::from_flat(&doc, 6).unwrap();
        assert_eq!(resolved.node_id(), t2);
        assert_eq!(resolved.offset(), 0);
    }

    #[test]
    fn from_flat_consecutive_close_open() {
        let (doc, t1, ..) = doc! {
            root {
                paragraph { t1: text("abc") }
                paragraph { text("de") }
            }
        };
        // P1: O=0 a=1 b=2 c=3 C=4
        // flat=4: text range [1,4] inclusive, target=4 → Position(t1, 3)
        let resolved = ResolvedPosition::from_flat(&doc, 4).unwrap();
        assert_eq!(resolved.node_id(), t1);
        assert_eq!(resolved.offset(), 3);
    }
}
