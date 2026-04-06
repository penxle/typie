use editor_common::StrExt;
use editor_model::{Doc, Node, NodeId};
use editor_state::{Position, ResolvedPosition};

use super::class::{FlatClass, classify};

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
                // Container: accumulate flat sizes of children[0..target_offset]
                let mut acc = 0usize;
                let mut prev_block = false;
                for (i, &child_id) in entry.children.iter().enumerate() {
                    if i == target_offset {
                        return Some(acc);
                    }
                    let child = doc.get_entry(child_id)?;
                    let is_block = matches!(
                        classify(&child.node),
                        FlatClass::Container | FlatClass::Atom
                    );
                    if is_block && prev_block {
                        acc += 1;
                    }
                    acc += subtree_flat_size(doc, child_id);
                    prev_block = is_block;
                }
                acc
            }
        });
    }

    let entry = doc.get_entry(current)?;
    let mut acc = 0usize;
    let mut prev_block = false;

    for &child_id in &entry.children {
        let child = doc.get_entry(child_id)?;
        let class = classify(&child.node);
        let is_block = matches!(class, FlatClass::Container | FlatClass::Atom);
        if is_block && prev_block {
            acc += 1;
        }

        if child_id == target {
            return match class {
                FlatClass::Text => Some(acc + target_offset),
                FlatClass::Container => {
                    to_flat_walk(doc, child_id, target, target_offset).map(|inner| acc + inner)
                }
                _ => Some(acc),
            };
        }

        if let FlatClass::Container = class
            && let Some(inner) = to_flat_walk(doc, child_id, target, target_offset)
        {
            return Some(acc + inner);
        }

        acc += subtree_flat_size(doc, child_id);
        prev_block = is_block;
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
            let mut size = 0usize;
            let mut prev_block = false;
            for &child_id in &entry.children {
                let child = match doc.get_entry(child_id) {
                    Some(c) => c,
                    None => continue,
                };
                let is_block = matches!(
                    classify(&child.node),
                    FlatClass::Container | FlatClass::Atom
                );
                if is_block && prev_block {
                    size += 1;
                }
                size += subtree_flat_size(doc, child_id);
                prev_block = is_block;
            }
            size
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
    let mut prev_block = false;

    for (i, &child_id) in entry.children.iter().enumerate() {
        let child = doc.get_entry(child_id)?;
        let class = classify(&child.node);
        let is_block = matches!(class, FlatClass::Container | FlatClass::Atom);

        if is_block && prev_block {
            // Block separator at position `acc` - upstream: position(container, i)
            if target == acc {
                return Some(Position::new(container, i));
            }
            acc += 1;
        }

        let child_size = subtree_flat_size(doc, child_id);

        if target >= acc && target <= acc + child_size {
            match class {
                FlatClass::Text => {
                    return Some(Position::new(child_id, target - acc));
                }
                FlatClass::Break | FlatClass::Atom => {
                    // target == acc → before leaf (position(container, i))
                    // target == acc + 1 → after leaf (position(container, i + 1))
                    return Some(Position::new(
                        container,
                        if target == acc { i } else { i + 1 },
                    ));
                }
                FlatClass::Container => {
                    return from_flat_walk(doc, child_id, acc, target);
                }
            }
        }

        acc += child_size;
        prev_block = is_block;
    }

    if target == acc {
        // End of container
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
        assert_eq!(resolved.to_flat(), 0);
    }

    #[test]
    fn to_flat_text_node_middle_offset() {
        let (doc, t1, ..) = doc! { root { paragraph { t1: text("hello") } } };
        let resolved = Position::new(t1, 3).resolve(&doc).unwrap();
        assert_eq!(resolved.to_flat(), 3);
    }

    #[test]
    fn to_flat_across_paragraphs() {
        let (doc, t2) = doc! {
            root {
                paragraph { text("hello") }
                paragraph { t2: text("world") }
            }
        };
        // "hello" = 0..5, "\n" = 5, "world" = 6..11
        let resolved = Position::new(t2, 2).resolve(&doc).unwrap();
        assert_eq!(resolved.to_flat(), 8);
    }

    #[test]
    fn to_flat_container_position_start() {
        let (doc, p2) = doc! {
            root {
                paragraph { text("abc") }
                p2: paragraph { text("de") }
            }
        };
        // position(p2, 0) = start of second paragraph = flat 4 (after "abc\n")
        let resolved = Position::new(p2, 0).resolve(&doc).unwrap();
        assert_eq!(resolved.to_flat(), 4);
    }

    #[test]
    fn to_flat_container_position_end() {
        let (doc, p1, ..) = doc! {
            root { p1: paragraph { text("hello") } }
        };
        // position(p1, 1) = after first child (the text node) = flat 5
        let resolved = Position::new(p1, 1).resolve(&doc).unwrap();
        assert_eq!(resolved.to_flat(), 5);
    }

    #[test]
    fn from_flat_zero_is_doc_start() {
        let (doc, t1, ..) = doc! { root { paragraph { t1: text("hello") } } };
        let resolved = ResolvedPosition::from_flat(&doc, 0).unwrap();
        assert_eq!(resolved.node_id(), t1);
        assert_eq!(resolved.offset(), 0);
    }

    #[test]
    fn from_flat_within_text() {
        let (doc, t1, ..) = doc! { root { paragraph { t1: text("hello") } } };
        let resolved = ResolvedPosition::from_flat(&doc, 3).unwrap();
        assert_eq!(resolved.node_id(), t1);
        assert_eq!(resolved.offset(), 3);
    }

    #[test]
    fn from_flat_out_of_range_returns_none() {
        let (doc, ..) = doc! { root { paragraph { text("hi") } } };
        assert!(ResolvedPosition::from_flat(&doc, 100).is_none());
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
    fn from_flat_across_paragraphs_lands_in_second() {
        let (doc, t2) = doc! {
            root {
                paragraph { text("abc") }
                paragraph { t2: text("de") }
            }
        };
        // flat: 'a'=0, 'b'=1, 'c'=2, '\n'=3, 'd'=4, 'e'=5
        let resolved = ResolvedPosition::from_flat(&doc, 4).unwrap();
        assert_eq!(resolved.node_id(), t2);
        assert_eq!(resolved.offset(), 0);
    }

    #[test]
    fn from_flat_at_block_separator_resolves_to_text_end() {
        let (doc, t1, ..) = doc! {
            root {
                paragraph { t1: text("abc") }
                paragraph { text("de") }
            }
        };
        // flat=3 is the end of "abc" AND the separator position between paragraphs.
        // Text node's inclusive range check catches this first, so we land at end of t1.
        let resolved = ResolvedPosition::from_flat(&doc, 3).unwrap();
        assert_eq!(resolved.node_id(), t1);
        assert_eq!(resolved.offset(), 3);
    }
}
