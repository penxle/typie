use editor_crdt::{Dot, LwwRegOp, OrMapOp, OrSetOp};
use editor_model::{DocOp, Modifier, PlainStyleEntry, StyleOp};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(
    style_id: String,
    old: Option<PlainStyleEntry>,
    new: Option<PlainStyleEntry>,
) -> Step {
    Step::SetStyle {
        style_id,
        old: new,
        new: old,
    }
}

/// Apply a style entry mutation by diffing the document's *actual* state against `new`.
/// `prev` captured at step creation time is intentionally ignored here — it serves
/// only as a guide for inverse construction (see `Step::inverse`).
pub(crate) fn apply_to(
    batched: &mut BatchedState,
    _validations: &mut Vec<Validation>,
    style_id: &str,
    new: Option<PlainStyleEntry>,
) -> Result<(), StepError> {
    let actual = capture_style_entry(batched, style_id);

    match (actual, new) {
        (None, Some(next)) => emit_define(batched, style_id, &next),
        (Some(_), None) => emit_delete(batched, style_id),
        (Some(current), Some(next)) => emit_edit(batched, style_id, &current, &next),
        (None, None) => Ok(()),
    }
}

fn capture_style_entry(batched: &BatchedState, style_id: &str) -> Option<PlainStyleEntry> {
    if !batched.doc.style_present(style_id) {
        return None;
    }
    let entry = batched.doc.style_entry(style_id)?;
    Some(PlainStyleEntry {
        name: entry.name.get().clone(),
        modifiers: entry.modifiers.iter().cloned().collect(),
    })
}

fn emit_define(
    batched: &mut BatchedState,
    style_id: &str,
    next: &PlainStyleEntry,
) -> Result<(), StepError> {
    batched.apply(DocOp::Style {
        style_id: style_id.to_string(),
        op: StyleOp::Presence(OrMapOp::Set {
            key: style_id.to_string(),
            value: (),
        }),
    })?;
    batched.apply(DocOp::Style {
        style_id: style_id.to_string(),
        op: StyleOp::Name(LwwRegOp::Set {
            value: next.name.clone(),
        }),
    })?;
    for m in &next.modifiers {
        batched.apply(DocOp::Style {
            style_id: style_id.to_string(),
            op: StyleOp::Modifiers(OrSetOp::Add { elem: m.clone() }),
        })?;
    }
    Ok(())
}

fn emit_delete(batched: &mut BatchedState, style_id: &str) -> Result<(), StepError> {
    let modifier_dots: Vec<Dot> = match batched.doc.style_entry(style_id) {
        Some(entry) => entry
            .modifiers
            .iter()
            .flat_map(|m| entry.modifiers.tags_for(m).copied())
            .collect(),
        None => Vec::new(),
    };
    for dot in modifier_dots {
        batched.apply(DocOp::Style {
            style_id: style_id.to_string(),
            op: StyleOp::Modifiers(OrSetOp::Remove { observed: dot }),
        })?;
    }

    let mut presence_observed: Vec<Dot> = batched
        .doc
        .styles_tags_for(&style_id.to_string())
        .copied()
        .collect();
    if !presence_observed.is_empty() {
        presence_observed.sort_unstable();
        presence_observed.dedup();
        batched.apply(DocOp::Style {
            style_id: style_id.to_string(),
            op: StyleOp::Presence(OrMapOp::Unset {
                observed: presence_observed,
            }),
        })?;
    }
    Ok(())
}

fn emit_edit(
    batched: &mut BatchedState,
    style_id: &str,
    current: &PlainStyleEntry,
    next: &PlainStyleEntry,
) -> Result<(), StepError> {
    if current.name != next.name {
        batched.apply(DocOp::Style {
            style_id: style_id.to_string(),
            op: StyleOp::Name(LwwRegOp::Set {
                value: next.name.clone(),
            }),
        })?;
    }

    let removed: Vec<Modifier> = current
        .modifiers
        .iter()
        .filter(|m| !next.modifiers.contains(*m))
        .cloned()
        .collect();
    let added: Vec<Modifier> = next
        .modifiers
        .iter()
        .filter(|m| !current.modifiers.contains(*m))
        .cloned()
        .collect();

    if !removed.is_empty() {
        let remove_dots: Vec<Dot> = match batched.doc.style_entry(style_id) {
            Some(e) => removed
                .iter()
                .flat_map(|m| e.modifiers.tags_for(m).copied())
                .collect(),
            None => Vec::new(),
        };
        for dot in remove_dots {
            batched.apply(DocOp::Style {
                style_id: style_id.to_string(),
                op: StyleOp::Modifiers(OrSetOp::Remove { observed: dot }),
            })?;
        }
    }
    for m in added {
        batched.apply(DocOp::Style {
            style_id: style_id.to_string(),
            op: StyleOp::Modifiers(OrSetOp::Add { elem: m }),
        })?;
    }
    Ok(())
}
