use editor_common::StrExt;
use editor_model::{Doc, Node, NodeId, NodeRef};
use editor_resource::Resource;

use crate::{Affinity, Position, Selection, paragraph_break_selection_at_paragraph_end};

pub fn resolve_word_selection_expansion(
    doc: &Doc,
    selection: Selection,
    resource: &Resource,
) -> Option<Selection> {
    resolve_selection_expansion(doc, selection, |doc, position| {
        word_selection_at(doc, position, resource)
    })
}

pub fn resolve_sentence_selection_expansion(
    doc: &Doc,
    selection: Selection,
    resource: &Resource,
) -> Option<Selection> {
    resolve_selection_expansion(doc, selection, |doc, position| {
        sentence_selection_at(doc, position, resource)
    })
}

pub fn resolve_paragraph_selection_expansion(doc: &Doc, selection: Selection) -> Option<Selection> {
    resolve_selection_expansion(doc, selection, paragraph_selection_at)
}

fn resolve_selection_expansion(
    doc: &Doc,
    selection: Selection,
    mut selection_at: impl FnMut(&Doc, Position) -> Option<Selection>,
) -> Option<Selection> {
    if selection.is_collapsed() {
        return selection_at(doc, selection.head);
    }

    let resolved = selection.resolve(doc)?;
    let from = Position::from(resolved.from());
    let to = Position::from(resolved.to());
    let to_lookup = range_end_lookup_position(doc, from, to);
    let from_selection = selection_at(doc, from)?;
    let to_selection = selection_at(doc, to_lookup)?;
    (from_selection == to_selection).then_some(from_selection)
}

fn word_selection_at(doc: &Doc, position: Position, resource: &Resource) -> Option<Selection> {
    inline_selection_at(doc, position, |context| context.word_selection(resource))
        .or_else(|| empty_textblock_selection_at(doc, position))
}

fn sentence_selection_at(doc: &Doc, position: Position, resource: &Resource) -> Option<Selection> {
    inline_selection_at(doc, position, |context| {
        context.sentence_selection(resource)
    })
    .or_else(|| empty_textblock_selection_at(doc, position))
}

fn paragraph_selection_at(doc: &Doc, position: Position) -> Option<Selection> {
    let paragraph = paragraph_at(doc, position)?;
    paragraph_selection(paragraph)
}

enum InlineTarget {
    Text { node_id: NodeId, offset: usize },
    Unit { parent_id: NodeId, index: usize },
}

fn inline_selection_at(
    doc: &Doc,
    position: Position,
    text_selection: impl FnOnce(&ConsecutiveText) -> Option<Selection>,
) -> Option<Selection> {
    match inline_target_at(doc, position) {
        Some(InlineTarget::Text { node_id, offset }) => {
            let text = ConsecutiveText::around(doc, node_id, offset)?;
            text_selection(&text)
        }
        Some(InlineTarget::Unit { parent_id, index }) => {
            Some(inline_unit_selection(parent_id, index))
        }
        None => None,
    }
}

fn inline_target_at(doc: &Doc, position: Position) -> Option<InlineTarget> {
    let node = doc.node(position.node_id)?;
    match node.node() {
        Node::Text(text) => {
            if position.offset == text.text.len()
                && let Some(next) = node.next_sibling()
                && matches!(
                    next.node(),
                    Node::HardBreak(_) | Node::PageBreak(_) | Node::Tab(_)
                )
            {
                return Some(InlineTarget::Unit {
                    parent_id: next.parent()?.id(),
                    index: next.index()?,
                });
            }
            if position.offset <= text.text.len() {
                Some(InlineTarget::Text {
                    node_id: node.id(),
                    offset: position.offset,
                })
            } else {
                None
            }
        }
        _ if node.spec().is_textblock() => inline_target_in_textblock(node, position),
        _ => None,
    }
}

fn inline_target_in_textblock(textblock: NodeRef<'_>, position: Position) -> Option<InlineTarget> {
    let index = match position.affinity {
        Affinity::Downstream => position.offset,
        Affinity::Upstream => position.offset.checked_sub(1)?,
    };
    let child = textblock.children().nth(index)?;
    match child.node() {
        Node::Text(text) => Some(InlineTarget::Text {
            node_id: child.id(),
            offset: if position.affinity == Affinity::Upstream {
                text.text.len()
            } else {
                0
            },
        }),
        Node::HardBreak(_) | Node::PageBreak(_) | Node::Tab(_) => Some(InlineTarget::Unit {
            parent_id: textblock.id(),
            index,
        }),
        _ => None,
    }
}

fn inline_unit_selection(parent_id: NodeId, index: usize) -> Selection {
    Selection::new(
        Position {
            node_id: parent_id,
            offset: index,
            affinity: Affinity::Downstream,
        },
        Position {
            node_id: parent_id,
            offset: index + 1,
            affinity: Affinity::Upstream,
        },
    )
}

fn range_end_lookup_position(doc: &Doc, from: Position, to: Position) -> Position {
    if to.offset > 0 {
        return Position {
            node_id: to.node_id,
            offset: to.offset - 1,
            affinity: Affinity::Downstream,
        };
    }

    let Some(node) = doc.node(to.node_id) else {
        return from;
    };
    if !matches!(node.node(), Node::Text(_)) {
        return from;
    }

    let mut current = node;
    while let Some(prev) = current.prev_sibling() {
        let Node::Text(text) = prev.node() else {
            break;
        };
        if text.text.len() > 0 {
            return Position {
                node_id: prev.id(),
                offset: text.text.len() - 1,
                affinity: Affinity::Downstream,
            };
        }
        current = prev;
    }

    from
}

fn paragraph_at(doc: &Doc, position: Position) -> Option<NodeRef<'_>> {
    doc.node(position.node_id)?
        .ancestors()
        .find(|n| matches!(n.node(), Node::Paragraph(_)))
}

fn paragraph_selection(paragraph: NodeRef<'_>) -> Option<Selection> {
    let child_count = paragraph.children().count();
    if child_count == 0 {
        return paragraph_break_selection_at_paragraph_end(
            paragraph.doc(),
            Position {
                node_id: paragraph.id(),
                offset: 0,
                affinity: Affinity::Downstream,
            },
        );
    }

    Some(Selection::new(
        Position {
            node_id: paragraph.id(),
            offset: 0,
            affinity: Affinity::Downstream,
        },
        Position {
            node_id: paragraph.id(),
            offset: child_count,
            affinity: Affinity::Upstream,
        },
    ))
}

fn empty_textblock_selection_at(doc: &Doc, position: Position) -> Option<Selection> {
    let node = doc.node(position.node_id)?;
    let textblock = if node.spec().is_textblock() {
        node
    } else {
        node.ancestors().find(|n| n.spec().is_textblock())?
    };

    if !textblock.children().all(|child| match child.node() {
        Node::Text(text) => text.text.is_empty(),
        _ => false,
    }) {
        return None;
    }

    paragraph_break_selection_at_paragraph_end(doc, position)
}

#[derive(Clone, Copy)]
struct TextRun {
    node_id: NodeId,
    start: usize,
    end: usize,
}

struct ConsecutiveText {
    text: String,
    runs: Vec<TextRun>,
    target: usize,
}

impl ConsecutiveText {
    fn around(doc: &Doc, node_id: NodeId, local_offset: usize) -> Option<Self> {
        let node = doc.node(node_id)?;
        let Node::Text(text) = node.node() else {
            return None;
        };
        if local_offset > text.text.len() {
            return None;
        }

        let mut text_node_ids = Vec::new();
        let mut current = node;
        while let Some(prev) = current.prev_sibling() {
            if !matches!(prev.node(), Node::Text(_)) {
                break;
            }
            text_node_ids.insert(0, prev.id());
            current = prev;
        }
        text_node_ids.push(node.id());
        current = node;
        while let Some(next) = current.next_sibling() {
            if !matches!(next.node(), Node::Text(_)) {
                break;
            }
            text_node_ids.push(next.id());
            current = next;
        }

        let mut runs = Vec::new();
        let mut full_text = String::new();
        let mut accumulated = 0;
        let mut target = None;

        for id in text_node_ids {
            let text_node = doc.node(id)?;
            let Node::Text(text) = text_node.node() else {
                return None;
            };
            let len = text.text.len();
            if id == node_id {
                target = Some(accumulated + local_offset);
            }
            full_text.push_str(&text.text.to_string());
            runs.push(TextRun {
                node_id: id,
                start: accumulated,
                end: accumulated + len,
            });
            accumulated += len;
        }

        Some(Self {
            text: full_text,
            runs,
            target: target?,
        })
    }

    fn word_selection(&self, resource: &Resource) -> Option<Selection> {
        let (start, end) = word_range_in_text(&self.text, self.target, resource)?;
        self.selection_from_range(start, end)
    }

    fn sentence_selection(&self, resource: &Resource) -> Option<Selection> {
        let (start, end) = sentence_range_in_text(&self.text, self.target, resource)?;
        self.selection_from_range(start, end)
    }

    fn selection_from_range(&self, start: usize, end: usize) -> Option<Selection> {
        if start >= end {
            return None;
        }

        let anchor = self.position_at_offset(start, false)?;
        let head = self.position_at_offset(end, true)?;
        Some(Selection::new(anchor, head))
    }

    fn position_at_offset(&self, offset: usize, end_bias: bool) -> Option<Position> {
        for run in &self.runs {
            if offset < run.start {
                continue;
            }
            let fits = if end_bias {
                offset <= run.end
            } else {
                offset < run.end
            };
            if fits {
                return Some(Position {
                    node_id: run.node_id,
                    offset: offset - run.start,
                    affinity: if end_bias {
                        Affinity::Upstream
                    } else {
                        Affinity::Downstream
                    },
                });
            }
        }

        if end_bias {
            self.runs.last().map(|run| Position {
                node_id: run.node_id,
                offset: run.end - run.start,
                affinity: Affinity::Upstream,
            })
        } else {
            None
        }
    }
}

fn word_range_in_text(
    text: &str,
    char_offset: usize,
    resource: &Resource,
) -> Option<(usize, usize)> {
    if text.is_empty() {
        return None;
    }

    let char_count = text.char_count();
    let target = char_offset.min(char_count);
    let boundaries = text_boundaries(
        text,
        resource.segmenters.word.as_borrowed().segment_str(text),
    );
    let mut range = range_from_boundaries(&boundaries, target, char_count);

    if range.0 == range.1 && target > 0 {
        range = range_from_boundaries(&boundaries, target - 1, char_count);
    }

    if range.0 > 0
        && text
            .chars()
            .skip(range.0)
            .take(range.1 - range.0)
            .all(char::is_whitespace)
    {
        range = range_from_boundaries(&boundaries, range.0 - 1, char_count);
    }

    (range.0 < range.1).then_some(range)
}

fn sentence_range_in_text(
    text: &str,
    char_offset: usize,
    resource: &Resource,
) -> Option<(usize, usize)> {
    if text.is_empty() {
        return None;
    }

    let char_count = text.char_count();
    let target = char_offset.min(char_count);
    let boundaries = text_boundaries(
        text,
        resource.segmenters.sentence.as_borrowed().segment_str(text),
    );
    let (start, mut end) = {
        let range = range_from_boundaries(&boundaries, target, char_count);
        if range.0 == range.1 && target > 0 {
            range_from_boundaries(&boundaries, target - 1, char_count)
        } else {
            range
        }
    };

    let chars: Vec<_> = text.chars().collect();
    while end > start && chars.get(end - 1).is_some_and(|c| c.is_whitespace()) {
        end -= 1;
    }

    (start < end).then_some((start, end))
}

fn text_boundaries(text: &str, byte_boundaries: impl Iterator<Item = usize>) -> Vec<usize> {
    let mut boundaries = vec![0];
    for boundary in byte_boundaries {
        let char_boundary = text.nth_byte_char_offset(boundary);
        if boundaries.last().copied() != Some(char_boundary) {
            boundaries.push(char_boundary);
        }
    }
    let char_count = text.char_count();
    if boundaries.last().copied() != Some(char_count) {
        boundaries.push(char_count);
    }
    boundaries
}

fn range_from_boundaries(
    boundaries: &[usize],
    char_offset: usize,
    char_count: usize,
) -> (usize, usize) {
    let start = boundaries
        .iter()
        .rev()
        .find(|&&boundary| boundary <= char_offset)
        .copied()
        .unwrap_or(0);
    let end = boundaries
        .iter()
        .find(|&&boundary| boundary > char_offset)
        .copied()
        .unwrap_or(char_count);
    (start, end)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_resource::Resource;

    use super::*;

    #[test]
    fn word_expansion_uses_consecutive_text_nodes() {
        let resource = Resource::new_test();
        let (state, t1, t2) = state! {
            doc { root { paragraph { t1: text("hel") t2: text("lo world") } } }
            selection: (t1, 1)
        };

        let actual =
            resolve_word_selection_expansion(&state.doc, state.selection.unwrap(), &resource);

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(t1, 0),
                Position {
                    node_id: t2,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn word_expansion_on_whitespace_selects_previous_word() {
        let resource = Resource::new_test();
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 5)
        };

        let actual =
            resolve_word_selection_expansion(&state.doc, state.selection.unwrap(), &resource);

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(t, 0),
                Position {
                    node_id: t,
                    offset: 5,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn word_expansion_at_text_hard_break_boundary_selects_hard_break() {
        let resource = Resource::new_test();
        let (state, p, t) = state! {
            doc { root { p: paragraph { t: text("h") hard_break } } }
            selection: (t, 1)
        };

        let selection = Selection::collapsed(Position {
            node_id: t,
            offset: 1,
            affinity: Affinity::Upstream,
        });
        let actual = resolve_word_selection_expansion(&state.doc, selection, &resource);

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(p, 1),
                Position {
                    node_id: p,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn word_expansion_at_text_tab_boundary_selects_tab() {
        let resource = Resource::new_test();
        let (state, p, t) = state! {
            doc { root { p: paragraph { t: text("h") tab } } }
            selection: (t, 1)
        };

        let selection = Selection::collapsed(Position {
            node_id: t,
            offset: 1,
            affinity: Affinity::Upstream,
        });
        let actual = resolve_word_selection_expansion(&state.doc, selection, &resource);

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(p, 1),
                Position {
                    node_id: p,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn sentence_expansion_on_tab_inline_unit_selects_unit() {
        let resource = Resource::new_test();
        let (state, p) = state! {
            doc { root { p: paragraph { text("Hello") tab } } }
            selection: (p, 1)
        };

        let actual =
            resolve_sentence_selection_expansion(&state.doc, state.selection.unwrap(), &resource);

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(p, 1),
                Position {
                    node_id: p,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn word_expansion_on_required_empty_paragraph_returns_none() {
        let resource = Resource::new_test();
        let (state, _p) = state! {
            doc { root { p: paragraph {} } }
            selection: (p, 0)
        };

        let actual =
            resolve_word_selection_expansion(&state.doc, state.selection.unwrap(), &resource);

        assert_eq!(actual, None);
    }

    #[test]
    fn word_expansion_on_empty_paragraph_selects_its_trailing_paragraph_break() {
        let resource = Resource::new_test();
        let (state, p, t) = state! {
            doc { root { p: paragraph {} paragraph { t: text("next") } } }
            selection: (p, 0)
        };

        let actual =
            resolve_word_selection_expansion(&state.doc, state.selection.unwrap(), &resource);

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(p, 0),
                Position {
                    node_id: t,
                    offset: 0,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn sentence_expansion_trims_trailing_whitespace() {
        let resource = Resource::new_test();
        let (state, t) = state! {
            doc { root { paragraph { t: text("Hello.  Next.") } } }
            selection: (t, 1)
        };

        let actual =
            resolve_sentence_selection_expansion(&state.doc, state.selection.unwrap(), &resource);

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(t, 0),
                Position {
                    node_id: t,
                    offset: 6,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn sentence_expansion_at_last_sentence_end_selects_last_sentence() {
        let resource = Resource::new_test();
        let (state, t) = state! {
            doc { root { paragraph { t: text("Hello. Last.") } } }
            selection: (t, 12)
        };

        let actual =
            resolve_sentence_selection_expansion(&state.doc, state.selection.unwrap(), &resource);

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(t, 7),
                Position {
                    node_id: t,
                    offset: 12,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn sentence_expansion_on_inline_unit_selects_unit() {
        let resource = Resource::new_test();
        let (state, p) = state! {
            doc { root { p: paragraph { text("Hello") hard_break } } }
            selection: (p, 1)
        };

        let actual =
            resolve_sentence_selection_expansion(&state.doc, state.selection.unwrap(), &resource);

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(p, 1),
                Position {
                    node_id: p,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn sentence_expansion_on_required_empty_paragraph_returns_none() {
        let resource = Resource::new_test();
        let (state, _p) = state! {
            doc { root { p: paragraph {} } }
            selection: (p, 0)
        };

        let actual =
            resolve_sentence_selection_expansion(&state.doc, state.selection.unwrap(), &resource);

        assert_eq!(actual, None);
    }

    #[test]
    fn word_expansion_expands_range_within_same_word() {
        let resource = Resource::new_test();
        let (state, t1, t2) = state! {
            doc { root { paragraph { t1: text("hel") t2: text("lo world") } } }
            selection: (t1, 1) -> (t2, 2)
        };

        let actual =
            resolve_word_selection_expansion(&state.doc, state.selection.unwrap(), &resource);

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(t1, 0),
                Position {
                    node_id: t2,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn word_expansion_does_not_shrink_range_across_words() {
        let resource = Resource::new_test();
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 1) -> (t, 8)
        };

        let actual =
            resolve_word_selection_expansion(&state.doc, state.selection.unwrap(), &resource);

        assert_eq!(actual, None);
    }

    #[test]
    fn paragraph_expansion_selects_internal_child_range() {
        let (state, p, _t1, _t2) = state! {
            doc { root { p: paragraph { t1: text("Hello ") t2: text("world!") } } }
            selection: (t1, 3)
        };

        let actual = resolve_paragraph_selection_expansion(&state.doc, state.selection.unwrap());

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(p, 0),
                Position {
                    node_id: p,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn paragraph_expansion_includes_trailing_hard_break() {
        let (state, p, _t) = state! {
            doc { root { p: paragraph { t: text("Hello") hard_break } } }
            selection: (t, 3)
        };

        let actual = resolve_paragraph_selection_expansion(&state.doc, state.selection.unwrap());

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(p, 0),
                Position {
                    node_id: p,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn paragraph_expansion_includes_trailing_page_break() {
        let (state, p, _t) = state! {
            doc { root { p: paragraph { t: text("Hello") page_break } } }
            selection: (t, 3)
        };

        let actual = resolve_paragraph_selection_expansion(&state.doc, state.selection.unwrap());

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(p, 0),
                Position {
                    node_id: p,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn paragraph_expansion_on_empty_paragraph_selects_its_trailing_paragraph_break() {
        let (state, p, t) = state! {
            doc { root { p: paragraph {} paragraph { t: text("next") } } }
            selection: (p, 0)
        };

        let actual = resolve_paragraph_selection_expansion(&state.doc, state.selection.unwrap());

        assert_eq!(
            actual,
            Some(Selection::new(
                Position::new(p, 0),
                Position {
                    node_id: t,
                    offset: 0,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }
}
