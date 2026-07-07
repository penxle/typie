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
                tr.insert_subtree(block, offset, subtree.clone())?;
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
