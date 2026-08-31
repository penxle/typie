use editor_crdt::Dot;
use editor_model::{ChildView, Modifier, NodeView, Subtree};
use editor_state::ProjectedState;
use editor_transaction::Transaction;

use crate::CommandError;
use crate::helpers::capture_atom_leaf_subtree_at;

pub(crate) enum CharlikeSlot {
    Char { ch: char, modifiers: Vec<Modifier> },
    Atom { subtree: Subtree },
}

pub(crate) fn capture_charlike_slots(
    ps: &ProjectedState,
    block: &NodeView,
    from: usize,
    to: usize,
) -> Result<Vec<CharlikeSlot>, CommandError> {
    let mut slots = Vec::new();
    for (slot, c) in block
        .children()
        .enumerate()
        .skip(from)
        .take(to.saturating_sub(from))
    {
        let ChildView::Leaf(l) = c else {
            continue;
        };
        if let Some(ch) = l.as_char() {
            slots.push(CharlikeSlot::Char {
                ch,
                modifiers: block.leaf_own_modifiers_at(slot),
            });
        } else if l.is_charlike() {
            slots.push(CharlikeSlot::Atom {
                subtree: capture_atom_leaf_subtree_at(ps, block, slot)?,
            });
        }
    }
    Ok(slots)
}

pub(crate) fn insert_charlike_slots(
    tr: &mut Transaction,
    block: Dot,
    start: usize,
    slots: &[CharlikeSlot],
) -> Result<(), CommandError> {
    let mut offset = start;
    let mut text = String::new();
    let mut modifiers: Vec<Vec<Modifier>> = Vec::new();
    for slot in slots {
        match slot {
            CharlikeSlot::Char { ch, modifiers: m } => {
                text.push(*ch);
                modifiers.push(m.clone());
            }
            CharlikeSlot::Atom { subtree } => {
                flush_charlike_text(tr, block, &mut offset, &mut text, &mut modifiers)?;
                tr.reissue_subtree(block, offset, subtree.clone())?;
                offset += 1;
            }
        }
    }
    flush_charlike_text(tr, block, &mut offset, &mut text, &mut modifiers)?;
    Ok(())
}

fn flush_charlike_text(
    tr: &mut Transaction,
    block: Dot,
    offset: &mut usize,
    text: &mut String,
    modifiers: &mut Vec<Vec<Modifier>>,
) -> Result<(), CommandError> {
    if text.is_empty() {
        return Ok(());
    }
    let start = *offset;
    tr.insert_text(block, start, text.as_str())?;
    let char_dots: Vec<_> = {
        let view = tr.view();
        view.node(block)
            .map(|p| {
                p.children()
                    .skip(start)
                    .take(modifiers.len())
                    .filter_map(|c| match c {
                        ChildView::Leaf(l) => l.as_char().map(|_| l.dot()),
                        ChildView::Block(_) => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    if char_dots.len() != modifiers.len() {
        return Err(CommandError::Corrupted(
            "inserted charlike text dots missing".into(),
        ));
    }
    for (dot, mods) in char_dots.iter().zip(modifiers.iter()) {
        for m in mods {
            tr.add_span_modifier(*dot, *dot, m.clone())?;
        }
    }
    *offset += modifiers.len();
    text.clear();
    modifiers.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::NodeType;
    use editor_state::State;

    use super::*;

    fn atom_dot(state: &State, block: Dot) -> Dot {
        state
            .view()
            .node(block)
            .unwrap()
            .children()
            .find_map(|c| match c {
                ChildView::Leaf(l) if l.node_type() == NodeType::Tab => Some(l.dot()),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn reinserted_charlike_atom_keeps_its_alias_class() {
        let (initial, source, target) = state! {
            doc { root {
                source: paragraph { text("ab") tab }
                target: paragraph { text("cd") }
            } }
            selection: (source, 0)
        };
        let old_atom = atom_dot(&initial, source);

        let (slots, target_len, source_len) = {
            let view = initial.view();
            let src = view.node(source).unwrap();
            let len = src.children().count();
            let slots = capture_charlike_slots(&initial.projected, &src, 0, len).unwrap();
            let target_len = view.node(target).unwrap().children().count();
            (slots, target_len, len)
        };

        let mut tr = Transaction::new(&initial);
        insert_charlike_slots(&mut tr, target, target_len, &slots).unwrap();
        let (actual, ..) = tr.commit();

        let view = actual.view();
        assert_eq!(view.node(target).unwrap().inline_text(), "cdab");
        let new_atom = atom_dot(&actual, target);
        assert_ne!(new_atom, old_atom);
        assert!(
            view.alias_classes()
                .members_of(new_atom)
                .is_some_and(|m| m.contains(&old_atom)),
            "옮겨 적은 원자는 옛 dot과 같은 동치류"
        );
        assert_eq!(
            view.node(source).unwrap().children().count(),
            source_len - 1,
            "원본에서 원자가 빠진다"
        );
        assert!(actual.projected.projected().hidden.is_empty());
    }
}
