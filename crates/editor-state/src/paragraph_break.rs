use editor_model::{Doc, Node, NodeRef, NodeType};

use crate::{
    Affinity, NodeRefCursorExt, Position, ResolvedSelection, Selection,
    position_before_or_same_logical_boundary, positions_at_same_logical_boundary,
};

pub fn paragraph_break_selection_at_paragraph_end(
    doc: &Doc,
    position: Position,
) -> Option<Selection> {
    let paragraph = paragraph_owner(doc, position)?;
    let end = paragraph_end_boundary_position(paragraph)?;
    if !positions_at_same_logical_boundary(doc, position, end) {
        return None;
    }
    trailing_break_for_paragraph(paragraph)
}

pub(crate) fn selection_exactly_matches_trailing_paragraph_break(
    doc: &Doc,
    selection: Selection,
) -> bool {
    let Some(resolved) = selection.resolve(doc) else {
        return false;
    };
    let from = Position::from(resolved.from());
    let to = Position::from(resolved.to());
    let Some(paragraph_break) = paragraph_break_selection_at_paragraph_end(doc, from) else {
        return false;
    };
    Selection::new(from, to) == paragraph_break
}

pub fn closest_empty_paragraph_break_end_between(
    doc: &Doc,
    from: Position,
    to: Position,
) -> Option<Position> {
    if positions_at_same_logical_boundary(doc, from, to) {
        return None;
    }

    let range = Selection::new(from, to).resolve(doc)?;
    let forward = range.anchor() < range.head();
    let mut best = None;
    collect_closest_empty_paragraph_break_end(
        doc,
        &range,
        range.common_ancestor(),
        from,
        to,
        forward,
        &mut best,
    );
    best
}

fn trailing_break_for_paragraph(paragraph: NodeRef<'_>) -> Option<Selection> {
    if !matches!(paragraph.node(), Node::Paragraph(_)) {
        return None;
    }

    if paragraph_has_trailing_page_break(&paragraph) {
        return None;
    }

    let Some(next) = paragraph.next_sibling() else {
        return None;
    };

    if matches!(next.node(), Node::Paragraph(_)) {
        return Some(Selection::new(
            paragraph_end_boundary_position(paragraph)?,
            paragraph_start_boundary_position(next)?,
        ));
    }

    if !paragraph_is_empty(&paragraph) || !empty_paragraph_is_removable(&paragraph) {
        return None;
    }

    Some(Selection::new(
        Position {
            node_id: paragraph.id(),
            offset: 0,
            affinity: Affinity::Downstream,
        },
        after_node_position(&paragraph)?,
    ))
}

fn paragraph_owner<'a>(doc: &'a Doc, pos: Position) -> Option<NodeRef<'a>> {
    doc.node(pos.node_id)?
        .ancestors()
        .find(|node| matches!(node.node(), Node::Paragraph(_)))
}

fn paragraph_start_boundary_position(paragraph: NodeRef<'_>) -> Option<Position> {
    if !matches!(paragraph.node(), Node::Paragraph(_)) {
        return None;
    }
    Some(Position {
        affinity: Affinity::Upstream,
        ..paragraph.first_cursor_position()?
    })
}

fn paragraph_end_boundary_position(paragraph: NodeRef<'_>) -> Option<Position> {
    if !matches!(paragraph.node(), Node::Paragraph(_)) {
        return None;
    }
    Some(Position {
        affinity: Affinity::Downstream,
        ..paragraph.last_cursor_position()?
    })
}

fn paragraph_is_empty(paragraph: &NodeRef<'_>) -> bool {
    if !matches!(paragraph.node(), Node::Paragraph(_)) {
        return false;
    }
    paragraph.children().all(|child| match child.node() {
        Node::Text(text) => text.text.is_empty(),
        _ => false,
    })
}

fn paragraph_has_trailing_page_break(paragraph: &NodeRef<'_>) -> bool {
    paragraph
        .last_child()
        .is_some_and(|child| matches!(child.node(), Node::PageBreak(_)))
}

fn empty_paragraph_is_removable(paragraph: &NodeRef<'_>) -> bool {
    let Some(parent) = paragraph.parent() else {
        return false;
    };
    let Some(index) = paragraph.index() else {
        return false;
    };
    let remaining: Vec<NodeType> = parent
        .children()
        .enumerate()
        .filter_map(|(i, child)| (i != index).then_some(child.as_type()))
        .collect();
    parent.spec().content.validate(&remaining).is_ok()
}

fn after_node_position(node: &NodeRef<'_>) -> Option<Position> {
    Some(Position {
        node_id: node.parent()?.id(),
        offset: node.index()? + 1,
        affinity: Affinity::Upstream,
    })
}

fn collect_closest_empty_paragraph_break_end(
    doc: &Doc,
    range: &ResolvedSelection<'_>,
    node: NodeRef<'_>,
    from: Position,
    to: Position,
    forward: bool,
    best: &mut Option<Position>,
) {
    if !range.intersects_subtree(&node) {
        return;
    }
    if matches!(node.node(), Node::Paragraph(_))
        && let Some(selection) =
            paragraph_break_selection_at_paragraph_end(doc, Position::new(node.id(), 0))
        && head_crosses_position(doc, from, selection.head, to, forward)
        && best.is_none_or(|current| closer_to_head(doc, selection.head, current, forward))
    {
        *best = Some(selection.head);
    }
    for child in node.children() {
        collect_closest_empty_paragraph_break_end(doc, range, child, from, to, forward, best);
    }
}

fn head_crosses_position(
    doc: &Doc,
    from: Position,
    stop: Position,
    to: Position,
    forward: bool,
) -> bool {
    if positions_at_same_logical_boundary(doc, from, stop) {
        return false;
    }
    if forward {
        position_before_or_same_logical_boundary(doc, from, stop)
            && position_before_or_same_logical_boundary(doc, stop, to)
    } else {
        position_before_or_same_logical_boundary(doc, stop, from)
            && position_before_or_same_logical_boundary(doc, to, stop)
    }
}

fn closer_to_head(doc: &Doc, candidate: Position, current: Position, forward: bool) -> bool {
    if forward {
        position_before_or_same_logical_boundary(doc, candidate, current)
    } else {
        position_before_or_same_logical_boundary(doc, current, candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::doc;

    #[test]
    fn p_to_p_has_paragraph_break_selection() {
        let (doc, _p1, t1, _p2, t2) = doc! {
            root {
                p1: paragraph { t1: text("a") }
                p2: paragraph { t2: text("b") }
            }
        };

        assert_eq!(
            paragraph_break_selection_at_paragraph_end(&doc, Position::new(t1, 1)),
            Some(Selection::new(
                Position {
                    node_id: t1,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: t2,
                    offset: 0,
                    affinity: Affinity::Upstream,
                }
            ))
        );
    }

    #[test]
    fn p_to_non_paragraph_has_no_break() {
        let (doc, _p, t) = doc! {
            root {
                p: paragraph { t: text("a") }
                callout { paragraph { text("b") } }
                paragraph {}
            }
        };

        assert_eq!(
            paragraph_break_selection_at_paragraph_end(&doc, Position::new(t, 1)),
            None
        );
    }

    #[test]
    fn paragraph_with_trailing_page_break_has_no_paragraph_break() {
        let (doc, p, _t) = doc! {
            root {
                p: paragraph { t: text("a") page_break }
                paragraph { text("b") }
            }
        };

        assert_eq!(
            paragraph_break_selection_at_paragraph_end(&doc, Position::new(p, 2)),
            None
        );
    }

    #[test]
    fn page_break_only_paragraph_has_no_paragraph_break() {
        let (doc, p) = doc! {
            root {
                p: paragraph { page_break }
                paragraph { text("b") }
            }
        };

        assert_eq!(
            paragraph_break_selection_at_paragraph_end(&doc, Position::new(p, 1)),
            None
        );
    }

    #[test]
    fn removable_empty_before_non_paragraph_has_paragraph_break_selection() {
        let (doc, root, e) = doc! {
            root: root {
                e: paragraph {}
                callout { paragraph { text("b") } }
                paragraph {}
            }
        };

        assert_eq!(
            paragraph_break_selection_at_paragraph_end(&doc, Position::new(e, 0)),
            Some(Selection::new(
                Position::new(e, 0),
                Position {
                    node_id: root,
                    offset: 1,
                    affinity: Affinity::Upstream,
                }
            ))
        );
    }

    #[test]
    fn required_trailing_empty_has_no_break() {
        let (doc, e) = doc! {
            root {
                callout { paragraph { text("b") } }
                e: paragraph {}
            }
        };

        assert_eq!(
            paragraph_break_selection_at_paragraph_end(&doc, Position::new(e, 0)),
            None
        );
    }

    #[test]
    fn removable_trailing_empty_has_no_break() {
        let (doc, e) = doc! {
            root {
                paragraph { text("b") }
                e: paragraph {}
            }
        };

        assert_eq!(
            paragraph_break_selection_at_paragraph_end(&doc, Position::new(e, 0)),
            None
        );
    }

    #[test]
    fn selection_normalize_preserves_paragraph_break_range_affinities() {
        let (doc, _p1, t1, _p2, t2) = doc! {
            root {
                p1: paragraph { t1: text("a") }
                p2: paragraph { t2: text("b") }
            }
        };

        let selection = Selection::new(Position::new(t1, 1), Position::new(t2, 0))
            .normalize(&doc)
            .expect("selection normalizes");
        assert_eq!(
            selection,
            paragraph_break_selection_at_paragraph_end(&doc, Position::new(t1, 1))
                .expect("P -> P has paragraph break")
        );
    }

    #[test]
    fn selection_normalize_preserves_reversed_paragraph_break_direction() {
        let (doc, _p1, t1, _p2, t2) = doc! {
            root {
                p1: paragraph { t1: text("a") }
                p2: paragraph { t2: text("b") }
            }
        };

        let paragraph_break =
            paragraph_break_selection_at_paragraph_end(&doc, Position::new(t1, 1))
                .expect("P -> P has paragraph break");
        let selection = Selection::new(Position::new(t2, 0), Position::new(t1, 1))
            .normalize(&doc)
            .expect("selection normalizes");
        assert_eq!(
            selection,
            Selection::new(paragraph_break.head, paragraph_break.anchor)
        );
    }

    #[test]
    fn selection_normalize_preserves_removable_empty_paragraph_break() {
        let (doc, root, p1) = doc! {
            root: root {
                p1: paragraph {}
                image
                paragraph {}
            }
        };

        let selection = Selection::new(
            Position::new(p1, 0),
            Position {
                node_id: root,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        )
        .normalize(&doc)
        .expect("selection normalizes");

        assert_eq!(
            selection,
            Selection::new(
                Position::new(p1, 0),
                Position {
                    node_id: root,
                    offset: 1,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }
}
