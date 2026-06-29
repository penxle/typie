use std::time::Duration;

use editor_common::HistoryTag;
use editor_common::time::Instant;

use editor_crdt::{ListOp, LwwRegOp, Op, OrMapOp, OrSetOp};
use editor_model::{
    EditOp, Marker, Modifier, ModifierAttrOp, NodeAttr, NodeAttrOp, NodeLwwOp, NodeType, SpanOp,
    StyleOp, StyleRegOp,
};

use crate::projected_state::ProjectedState;
use crate::{Composition, PendingModifiers, PendingStyle, StableSelection};

/// Editor state restored alongside a doc undo/redo: the caret/selection and the
/// transient IME/pending overlays that the op-level history does not encode as
/// document ops. Recorded as the pre-transaction state; restored on undo.
///
/// The selection is stored as a [`StableSelection`] (path + boundary binding),
/// not a raw `(node, offset)` `Position`. Concurrent remote ops can restructure
/// the document between the time an entry is recorded and the time it is restored
/// (e.g. a remote paragraph split re-parents this node's children), which would
/// leave a raw position dangling. Re-resolving the stable form on restore keeps
/// the invariant "`state.selection` always resolves" — the same guarantee the
/// remote-changeset path already relies on.
#[derive(Clone, Default, PartialEq)]
pub struct TransientState {
    pub selection: Option<StableSelection>,
    pub composition: Option<Composition>,
    pub pending_modifiers: PendingModifiers,
    pub pending_style: Option<PendingStyle>,
}

pub enum PriorValue {
    BlockModifier(Option<Modifier>),
    NodeAttr(NodeAttr),
    NodeStyle(Option<String>),
    NodeMarker(Option<Marker>),
    StyleName(String),
    StyleModifier(Modifier),
    SpanModifier(Modifier),
}

pub struct RecordedOp {
    pub op: Op<EditOp>,
    pub prior: Option<PriorValue>,
}

pub struct UndoEntry {
    pub ops: Vec<RecordedOp>,
    pub tag: Option<HistoryTag>,
    /// Transient state to restore when this entry is applied (undone). Recorded
    /// as the state *before* the transaction; the redo entry pushed by `undo`
    /// captures the state current at undo time.
    pub transient: TransientState,
}

pub struct UndoHistory {
    undos: Vec<UndoEntry>,
    redos: Vec<UndoEntry>,
    merge_interval: Duration,
    last_push: Option<Instant>,
    last_tag: Option<HistoryTag>,
    last_tag_revision: u64,
}

impl UndoHistory {
    pub fn new(merge_interval: Duration) -> Self {
        Self {
            undos: Vec::new(),
            redos: Vec::new(),
            merge_interval,
            last_push: None,
            last_tag: None,
            last_tag_revision: 0,
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undos.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redos.is_empty()
    }

    pub fn undos_len(&self) -> usize {
        self.undos.len()
    }

    pub fn redos_len(&self) -> usize {
        self.redos.len()
    }

    pub fn last_tag(&self) -> Option<&HistoryTag> {
        self.last_tag.as_ref()
    }

    /// The transient state recorded with the most recent undo entry (e.g. the
    /// caret before the last edit). Used by repaste to recover the pre-paste
    /// caret without popping the entry.
    pub fn last_transient(&self) -> Option<&TransientState> {
        self.undos.last().map(|e| &e.transient)
    }

    pub fn last_tag_revision(&self) -> u64 {
        self.last_tag_revision
    }

    fn bump_last_tag_revision(&mut self) {
        self.last_tag_revision = self.last_tag_revision.wrapping_add(1);
    }

    pub fn clear_last_tag(&mut self) {
        if self.last_tag.is_some() {
            self.last_tag = None;
            self.bump_last_tag_revision();
        }
    }

    pub fn sync_last_tag_from_top(&mut self) {
        let last_tag = self.undos.last().and_then(|e| e.tag.clone());
        let should_bump = self.last_tag != last_tag || last_tag.is_some();
        self.last_tag = last_tag;
        if should_bump {
            self.bump_last_tag_revision();
        }
    }

    /// Replace the most recent undo entry's tag in place. Used by repaste to
    /// stamp the pasted span's start onto the existing `PasteHtml` tag once the
    /// paste transaction has committed (the inline/structural decision needs the
    /// post-paste state, which is unknown while the transaction's tag is set).
    pub fn set_last_tag(&mut self, tag: Option<HistoryTag>) {
        if let Some(entry) = self.undos.last_mut() {
            entry.tag = tag.clone();
        }
        if self.last_tag != tag {
            self.last_tag = tag;
            self.bump_last_tag_revision();
        }
    }

    pub fn record(&mut self, entry: UndoEntry, now: Instant) {
        self.redos.clear();

        let tag = entry.tag.clone();
        let should_merge = self
            .last_push
            .map(|t| now.duration_since(t) < self.merge_interval)
            .unwrap_or(false);
        let can_merge = should_merge
            && matches!(self.undos.last(), Some(e) if e.tag.is_none())
            && entry.tag.is_none();

        if can_merge {
            self.undos
                .last_mut()
                .expect("can_merge guarantees Some")
                .ops
                .extend(entry.ops);
        } else {
            self.undos.push(entry);
        }
        self.last_push = Some(now);

        match tag {
            Some(t) => {
                self.last_tag = Some(t);
                self.bump_last_tag_revision();
            }
            None => self.clear_last_tag(),
        }
    }

    /// Undo the most recent entry: apply each op's inverse to `state`, returning
    /// the applied inverse ops (so the editor can broadcast them) and the
    /// selection to restore. `current_selection` is stored on the pushed redo
    /// entry so a subsequent redo restores the selection current at undo time.
    pub fn undo(
        &mut self,
        state: &mut ProjectedState,
        current_transient: TransientState,
    ) -> Option<(Vec<Op<EditOp>>, TransientState)> {
        let entry = self.undos.pop()?;
        let tag = entry.tag.clone();
        let restore_transient = entry.transient.clone();
        let mut redo_ops: Vec<RecordedOp> = Vec::new();
        let mut applied: Vec<Op<EditOp>> = Vec::new();
        'entry: for ro in entry.ops.into_iter().rev() {
            for inv_payload in invert(state, &ro) {
                let prior_for_redo = capture_prior(state, &inv_payload);
                let Ok(inv_op) = state.apply(inv_payload) else {
                    break 'entry;
                };
                applied.push(inv_op.clone());
                redo_ops.push(RecordedOp {
                    op: inv_op,
                    prior: prior_for_redo,
                });
            }
        }
        self.redos.push(UndoEntry {
            ops: redo_ops,
            tag,
            transient: current_transient,
        });
        self.sync_last_tag_from_top();
        Some((applied, restore_transient))
    }

    pub fn redo(
        &mut self,
        state: &mut ProjectedState,
        current_transient: TransientState,
    ) -> Option<(Vec<Op<EditOp>>, TransientState)> {
        let entry = self.redos.pop()?;
        let tag = entry.tag.clone();
        let restore_transient = entry.transient.clone();
        let mut undo_ops: Vec<RecordedOp> = Vec::new();
        let mut applied: Vec<Op<EditOp>> = Vec::new();
        'entry: for ro in entry.ops.into_iter().rev() {
            for inv_payload in invert(state, &ro) {
                let prior_for_undo = capture_prior(state, &inv_payload);
                let Ok(inv_op) = state.apply(inv_payload) else {
                    break 'entry;
                };
                applied.push(inv_op.clone());
                undo_ops.push(RecordedOp {
                    op: inv_op,
                    prior: prior_for_undo,
                });
            }
        }
        self.undos.push(UndoEntry {
            ops: undo_ops,
            tag,
            transient: current_transient,
        });
        self.sync_last_tag_from_top();
        Some((applied, restore_transient))
    }
}

pub fn capture_prior(state: &ProjectedState, op: &EditOp) -> Option<PriorValue> {
    match op {
        EditOp::BlockModifier(ModifierAttrOp::SetModifier { target, modifier }) => {
            let prior = state
                .block_modifiers()
                .modifiers_of(*target)
                .get(&modifier.as_type())
                .cloned();
            Some(PriorValue::BlockModifier(prior))
        }
        EditOp::BlockModifier(ModifierAttrOp::ClearModifier { target, key }) => {
            let prior = state
                .block_modifiers()
                .modifiers_of(*target)
                .get(key)
                .cloned();
            Some(PriorValue::BlockModifier(prior))
        }
        EditOp::NodeStyle(NodeLwwOp { target, .. }) => {
            Some(PriorValue::NodeStyle(state.node_styles().value_of(*target)))
        }
        EditOp::NodeMarker(NodeLwwOp { target, .. }) => Some(PriorValue::NodeMarker(
            state.node_markers().value_of(*target),
        )),
        EditOp::NodeAttr(NodeAttrOp { target, attr }) => {
            let node_type = state
                .projected()
                .node_attrs
                .get(target)
                .map(|n| n.as_type())
                .unwrap_or_else(|| node_type_of_attr(attr));
            let prior_node = state.node_attrs().attrs_of(*target, node_type);
            let prior_attrs = prior_node.to_plain().to_attrs();
            let target_disc = std::mem::discriminant(attr);
            let prior_attr = prior_attrs
                .into_iter()
                .find(|a| std::mem::discriminant(a) == target_disc)?;
            Some(PriorValue::NodeAttr(prior_attr))
        }
        EditOp::Style(StyleRegOp {
            style_id,
            op: StyleOp::Name(_),
        }) => {
            let prior = state
                .styles()
                .style_entry(style_id)
                .map(|e| e.name.get().clone())
                .unwrap_or_default();
            Some(PriorValue::StyleName(prior))
        }
        EditOp::Style(StyleRegOp {
            style_id,
            op: StyleOp::Modifiers(OrSetOp::Remove { observed }),
        }) => {
            let observed_dot = *observed;
            let found = state.styles().iter().find_map(|(dot, reg)| {
                if reg.style_id != *style_id {
                    return None;
                }
                if *dot != observed_dot {
                    return None;
                }
                if let StyleOp::Modifiers(OrSetOp::Add { elem }) = &reg.op {
                    Some(elem.clone())
                } else {
                    None
                }
            });
            found.map(PriorValue::StyleModifier)
        }
        EditOp::Span(SpanOp::RemoveSpan {
            start,
            end,
            modifier_type,
        }) => {
            let found = state.spans().iter().find_map(|(_, span_op)| {
                if let SpanOp::AddSpan {
                    start: s,
                    end: e,
                    modifier,
                } = span_op
                {
                    if s == start && e == end && &modifier.as_type() == modifier_type {
                        Some(modifier.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            found.map(PriorValue::SpanModifier)
        }
        _ => None,
    }
}

fn node_type_of_attr(attr: &NodeAttr) -> NodeType {
    match attr {
        NodeAttr::Root { .. } => NodeType::Root,
        NodeAttr::Paragraph { .. } => NodeType::Paragraph,
        NodeAttr::Blockquote { .. } => NodeType::Blockquote,
        NodeAttr::Callout { .. } => NodeType::Callout,
        NodeAttr::Text { .. } => NodeType::Text,
        NodeAttr::BulletList { .. } => NodeType::BulletList,
        NodeAttr::OrderedList { .. } => NodeType::OrderedList,
        NodeAttr::ListItem { .. } => NodeType::ListItem,
        NodeAttr::Fold { .. } => NodeType::Fold,
        NodeAttr::FoldTitle { .. } => NodeType::FoldTitle,
        NodeAttr::FoldContent { .. } => NodeType::FoldContent,
        NodeAttr::Table { .. } => NodeType::Table,
        NodeAttr::TableRow { .. } => NodeType::TableRow,
        NodeAttr::TableCell { .. } => NodeType::TableCell,
        NodeAttr::Image { .. } => NodeType::Image,
        NodeAttr::File { .. } => NodeType::File,
        NodeAttr::Embed { .. } => NodeType::Embed,
        NodeAttr::Archived { .. } => NodeType::Archived,
        NodeAttr::HardBreak { .. } => NodeType::HardBreak,
        NodeAttr::HorizontalRule { .. } => NodeType::HorizontalRule,
        NodeAttr::PageBreak { .. } => NodeType::PageBreak,
        NodeAttr::Tab { .. } => NodeType::Tab,
    }
}

/// The inverse op(s) of a recorded op. Most ops invert 1:1, but redoing a
/// deletion (inverting its `Undel`) expands into one single-element `Del` per
/// still-visible target. An empty result means the op has no inverse to apply
/// (e.g. a missing prior value, or a deletion whose targets are all gone).
pub fn invert(state: &ProjectedState, ro: &RecordedOp) -> Vec<EditOp> {
    let dot = ro.op.id;
    match &ro.op.payload {
        // Undo of an insertion deletes the inserted char — but only if it is
        // still visible. If a concurrent op already deleted it, there is nothing
        // to remove (and a positional `Del` would overrun the sequence).
        EditOp::Seq(ListOp::Ins { .. }) => match state.seq_visible_pos(dot) {
            Some(pos) => vec![EditOp::Seq(ListOp::Del { pos, len: 1 })],
            None => Vec::new(),
        },
        EditOp::Seq(ListOp::Del { .. }) => vec![EditOp::Seq(ListOp::Undel { del: dot })],
        // Redo of a deletion: re-delete each still-visible target individually,
        // in descending position order so an earlier removal never shifts a
        // later one. Targets a concurrent op already deleted are skipped, which
        // prevents the out-of-bounds `Del` that previously panicked.
        EditOp::Seq(ListOp::Undel { del }) => state
            .del_target_positions(*del)
            .into_iter()
            .map(|pos| EditOp::Seq(ListOp::Del { pos, len: 1 }))
            .collect(),
        EditOp::Span(SpanOp::AddSpan {
            start,
            end,
            modifier,
        }) => vec![EditOp::Span(SpanOp::RemoveSpan {
            start: *start,
            end: *end,
            modifier_type: modifier.as_type(),
        })],
        EditOp::Span(SpanOp::RemoveSpan { start, end, .. }) => match &ro.prior {
            Some(PriorValue::SpanModifier(modifier)) => vec![EditOp::Span(SpanOp::AddSpan {
                start: *start,
                end: *end,
                modifier: modifier.clone(),
            })],
            _ => Vec::new(),
        },
        EditOp::BlockModifier(ModifierAttrOp::SetModifier { target, modifier }) => {
            match &ro.prior {
                Some(PriorValue::BlockModifier(Some(prior_m))) => {
                    vec![EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                        target: *target,
                        modifier: prior_m.clone(),
                    })]
                }
                Some(PriorValue::BlockModifier(None)) => {
                    vec![EditOp::BlockModifier(ModifierAttrOp::ClearModifier {
                        target: *target,
                        key: modifier.as_type(),
                    })]
                }
                _ => Vec::new(),
            }
        }
        EditOp::BlockModifier(ModifierAttrOp::ClearModifier { target, key: _ }) => {
            match &ro.prior {
                Some(PriorValue::BlockModifier(Some(prior_m))) => {
                    vec![EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                        target: *target,
                        modifier: prior_m.clone(),
                    })]
                }
                _ => Vec::new(),
            }
        }
        EditOp::NodeStyle(NodeLwwOp { target, .. }) => match &ro.prior {
            Some(PriorValue::NodeStyle(prior)) => vec![EditOp::NodeStyle(NodeLwwOp {
                target: *target,
                op: LwwRegOp::Set {
                    value: prior.clone(),
                },
            })],
            _ => Vec::new(),
        },
        EditOp::NodeMarker(NodeLwwOp { target, .. }) => match &ro.prior {
            Some(PriorValue::NodeMarker(prior)) => vec![EditOp::NodeMarker(NodeLwwOp {
                target: *target,
                op: LwwRegOp::Set {
                    value: prior.clone(),
                },
            })],
            _ => Vec::new(),
        },
        EditOp::NodeAttr(NodeAttrOp { target, .. }) => match &ro.prior {
            Some(PriorValue::NodeAttr(prior_attr)) => vec![EditOp::NodeAttr(NodeAttrOp {
                target: *target,
                attr: prior_attr.clone(),
            })],
            _ => Vec::new(),
        },
        EditOp::Style(StyleRegOp {
            style_id,
            op: StyleOp::Name(_),
        }) => match &ro.prior {
            Some(PriorValue::StyleName(prior_name)) => vec![EditOp::Style(StyleRegOp {
                style_id: style_id.clone(),
                op: StyleOp::Name(LwwRegOp::Set {
                    value: prior_name.clone(),
                }),
            })],
            _ => Vec::new(),
        },
        EditOp::Style(StyleRegOp {
            style_id,
            op: StyleOp::Modifiers(OrSetOp::Add { .. }),
        }) => vec![EditOp::Style(StyleRegOp {
            style_id: style_id.clone(),
            op: StyleOp::Modifiers(OrSetOp::Remove { observed: dot }),
        })],
        EditOp::Style(StyleRegOp {
            style_id,
            op: StyleOp::Modifiers(OrSetOp::Remove { .. }),
        }) => match &ro.prior {
            Some(PriorValue::StyleModifier(prior_mod)) => vec![EditOp::Style(StyleRegOp {
                style_id: style_id.clone(),
                op: StyleOp::Modifiers(OrSetOp::Add {
                    elem: prior_mod.clone(),
                }),
            })],
            _ => Vec::new(),
        },
        EditOp::Style(StyleRegOp {
            style_id,
            op: StyleOp::Presence(OrMapOp::Set { key: _, .. }),
        }) => vec![EditOp::Style(StyleRegOp {
            style_id: style_id.clone(),
            op: StyleOp::Presence(OrMapOp::Unset {
                observed: vec![dot],
            }),
        })],
        EditOp::Style(StyleRegOp {
            style_id,
            op: StyleOp::Presence(OrMapOp::Unset { .. }),
        }) => vec![EditOp::Style(StyleRegOp {
            style_id: style_id.clone(),
            op: StyleOp::Presence(OrMapOp::Set {
                key: style_id.clone(),
                value: (),
            }),
        })],
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use editor_common::time::Instant;

    use editor_crdt::{ListOp, LwwRegOp, OrMapOp, OrSetOp};
    use editor_model::{
        Anchor, Bias, CalloutNodeAttr, CalloutVariant, EditOp, Modifier, ModifierAttrOp,
        ModifierType, NodeAttr, NodeAttrOp, NodeLwwOp, NodeType, SeqItem, SpanOp, StyleOp,
        StyleRegOp,
    };

    use super::{
        HistoryTag, PriorValue, RecordedOp, TransientState, UndoEntry, UndoHistory, capture_prior,
    };
    use crate::projected_state::ProjectedState;

    fn seq_char(pos: usize, c: char) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Char(c),
        })
    }

    fn seq_block(pos: usize, node_type: NodeType, parents: Vec<editor_crdt::Dot>) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Block { node_type, parents },
        })
    }

    fn record_op(state: &mut ProjectedState, payload: EditOp) -> RecordedOp {
        let prior = capture_prior(state, &payload);
        let op = state.apply(payload).expect("apply succeeded");
        RecordedOp { op, prior }
    }

    fn single_entry(ro: RecordedOp) -> UndoEntry {
        UndoEntry {
            ops: vec![ro],
            tag: None,
            transient: TransientState::default(),
        }
    }

    #[test]
    fn last_tag_tracks_record_and_syncs_on_undo_redo() {
        let mut state = ProjectedState::empty();
        let mut history = UndoHistory::new(Duration::from_secs(0));

        assert!(history.last_tag().is_none());
        assert_eq!(history.last_tag_revision(), 0);

        let ro1 = record_op(&mut state, seq_char(1, 'a'));
        history.record(single_entry(ro1), Instant::now());
        assert!(history.last_tag().is_none());
        assert_eq!(
            history.last_tag_revision(),
            0,
            "untagged record of None->None does not bump"
        );

        let ro2 = record_op(&mut state, seq_char(2, 'b'));
        history.record(
            UndoEntry {
                ops: vec![ro2],
                tag: Some(HistoryTag::AutoReplacement),
                transient: TransientState::default(),
            },
            Instant::now(),
        );
        assert_eq!(history.last_tag(), Some(&HistoryTag::AutoReplacement));
        let rev_after_tagged = history.last_tag_revision();
        assert!(rev_after_tagged > 0, "tagged record bumps revision");

        let ro3 = record_op(&mut state, seq_char(3, 'c'));
        history.record(single_entry(ro3), Instant::now());
        assert!(
            history.last_tag().is_none(),
            "untagged record clears a set last_tag"
        );
        assert!(
            history.last_tag_revision() > rev_after_tagged,
            "clearing a set tag bumps revision"
        );

        history.undo(&mut state, TransientState::default());
        assert_eq!(
            history.last_tag(),
            Some(&HistoryTag::AutoReplacement),
            "undo syncs last_tag to the new (tagged) top"
        );

        history.redo(&mut state, TransientState::default());
        assert!(
            history.last_tag().is_none(),
            "redo syncs last_tag to the new (untagged) top"
        );
    }

    #[test]
    fn clear_last_tag_bumps_only_when_set() {
        let mut history = UndoHistory::new(Duration::from_secs(0));
        history.clear_last_tag();
        assert_eq!(
            history.last_tag_revision(),
            0,
            "clearing an already-None tag does not bump"
        );
        history.sync_last_tag_from_top();
        assert!(history.last_tag().is_none());
    }

    #[test]
    fn undo_skips_noop_inverse_and_applies_real_inverse() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();

        let set_op = state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: para,
                modifier: Modifier::FontSize { value: 1600 },
            }))
            .unwrap();

        let clear_op = state
            .apply(EditOp::BlockModifier(ModifierAttrOp::ClearModifier {
                target: para,
                key: ModifierType::Bold,
            }))
            .unwrap();

        assert_eq!(
            state
                .block_modifiers()
                .modifiers_of(para)
                .get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );

        let entry = UndoEntry {
            ops: vec![
                RecordedOp {
                    op: set_op,
                    prior: Some(PriorValue::BlockModifier(None)),
                },
                RecordedOp {
                    op: clear_op,
                    prior: Some(PriorValue::BlockModifier(None)),
                },
            ],
            tag: None,
            transient: TransientState::default(),
        };

        let mut history = UndoHistory::new(Duration::from_secs(0));
        history.record(entry, Instant::now());
        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state
                .block_modifiers()
                .modifiers_of(para)
                .get(&ModifierType::FontSize),
            None,
            "SetModifier inverse must be applied — undo must not abort on the no-op ClearModifier"
        );
    }

    #[test]
    fn seq_ins_undo_restores_text_and_redo_reapplies() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();

        assert_eq!(state.view().node(para).unwrap().inline_text(), "");

        let ro_a = record_op(&mut state, seq_char(1, 'a'));
        let ro_b = record_op(&mut state, seq_char(2, 'b'));
        let ro_c = record_op(&mut state, seq_char(3, 'c'));

        assert_eq!(state.view().node(para).unwrap().inline_text(), "abc");

        let entry = UndoEntry {
            ops: vec![ro_a, ro_b, ro_c],
            tag: None,
            transient: TransientState::default(),
        };

        let mut history = UndoHistory::new(Duration::from_secs(1));
        history.record(entry, Instant::now());
        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state.view().node(para).unwrap().inline_text(),
            "",
            "undo must restore text to empty"
        );

        history.redo(&mut state, TransientState::default());

        assert_eq!(
            state.view().node(para).unwrap().inline_text(),
            "abc",
            "redo must reapply the three inserted chars"
        );
    }

    #[test]
    fn seq_del_undo_restores_deleted_char() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();

        state.apply(seq_char(1, 'x')).unwrap();
        state.apply(seq_char(2, 'y')).unwrap();

        assert_eq!(state.view().node(para).unwrap().inline_text(), "xy");

        let ro_del = record_op(&mut state, EditOp::Seq(ListOp::Del { pos: 1, len: 1 }));

        assert_eq!(state.view().node(para).unwrap().inline_text(), "y");

        let mut history = UndoHistory::new(Duration::from_secs(1));
        history.record(single_entry(ro_del), Instant::now());
        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state.view().node(para).unwrap().inline_text(),
            "xy",
            "undo of Del must restore the deleted char via Undel"
        );
    }

    // Regression: an IME composition step replaces the composing char in place,
    // emitting Del(old)+Ins(new) grouped with the original Ins(old) into one undo
    // unit (e.g. typing 'a' then composing it into 'b' yields Ins a, Del a, Ins b).
    // Undo must clear to "" and redo must reproduce "b" — NOT resurrect the
    // intermediate 'a' that the replace deleted. Redo previously re-applied the
    // unit's ops in reverse order, re-inserting 'a' after its re-deletion ran, so
    // the replaced intermediates came back (the "안녕하세요" → garbage bug).
    #[test]
    fn redo_after_ime_replace_does_not_resurrect_intermediate_char() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();

        let ro_ins_a = record_op(&mut state, seq_char(1, 'a'));
        let ro_del_a = record_op(&mut state, EditOp::Seq(ListOp::Del { pos: 1, len: 1 }));
        let ro_ins_b = record_op(&mut state, seq_char(1, 'b'));

        assert_eq!(state.view().node(para).unwrap().inline_text(), "b");

        let entry = UndoEntry {
            ops: vec![ro_ins_a, ro_del_a, ro_ins_b],
            tag: None,
            transient: TransientState::default(),
        };

        let mut history = UndoHistory::new(Duration::from_secs(1));
        history.record(entry, Instant::now());

        history.undo(&mut state, TransientState::default());
        assert_eq!(
            state.view().node(para).unwrap().inline_text(),
            "",
            "undo of the replace unit must clear the paragraph"
        );

        history.redo(&mut state, TransientState::default());
        assert_eq!(
            state.view().node(para).unwrap().inline_text(),
            "b",
            "redo must reproduce 'b' without resurrecting the replaced 'a'"
        );
    }

    // The "안녕하세요" bug in miniature: composing a single syllable through
    // several jamo replaces each previous form in place (ㅎ → 하 → 한), emitting
    // Ins ㅎ, Del ㅎ, Ins 하, Del 하, Ins 한 in one undo unit. Redo must reproduce
    // only the final "한"; reverse-order re-application resurrected every replaced
    // intermediate.
    #[test]
    fn redo_after_chained_ime_replaces_keeps_only_final_form() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();

        let ops = vec![
            record_op(&mut state, seq_char(1, 'ㅎ')),
            record_op(&mut state, EditOp::Seq(ListOp::Del { pos: 1, len: 1 })),
            record_op(&mut state, seq_char(1, '하')),
            record_op(&mut state, EditOp::Seq(ListOp::Del { pos: 1, len: 1 })),
            record_op(&mut state, seq_char(1, '한')),
        ];

        assert_eq!(state.view().node(para).unwrap().inline_text(), "한");

        let mut history = UndoHistory::new(Duration::from_secs(1));
        history.record(
            UndoEntry {
                ops,
                tag: None,
                transient: TransientState::default(),
            },
            Instant::now(),
        );

        history.undo(&mut state, TransientState::default());
        assert_eq!(state.view().node(para).unwrap().inline_text(), "");

        history.redo(&mut state, TransientState::default());
        assert_eq!(
            state.view().node(para).unwrap().inline_text(),
            "한",
            "redo must keep only the final composed form, not the replaced jamo"
        );

        history.undo(&mut state, TransientState::default());
        assert_eq!(
            state.view().node(para).unwrap().inline_text(),
            "",
            "a second undo after redo must clear again (round-trip stays stable)"
        );
    }

    #[test]
    fn span_add_span_undo_removes_bold() {
        let mut state = ProjectedState::empty();

        let x_dot = state.apply(seq_char(1, 'x')).unwrap().id;

        let add_span = EditOp::Span(SpanOp::AddSpan {
            start: Anchor {
                id: x_dot,
                bias: Bias::Before,
            },
            end: Anchor {
                id: x_dot,
                bias: Bias::After,
            },
            modifier: Modifier::Bold,
        });
        let ro = record_op(&mut state, add_span);

        assert_eq!(
            state
                .view()
                .leaf(x_dot)
                .unwrap()
                .effective()
                .get(&ModifierType::Bold),
            Some(&Modifier::Bold)
        );

        let mut history = UndoHistory::new(Duration::from_secs(1));
        history.record(single_entry(ro), Instant::now());
        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state
                .view()
                .leaf(x_dot)
                .unwrap()
                .effective()
                .get(&ModifierType::Bold),
            None,
            "undo of AddSpan must remove Bold from the leaf"
        );
    }

    #[test]
    fn span_remove_span_undo_restores_bold() {
        let mut state = ProjectedState::empty();

        let x_dot = state.apply(seq_char(1, 'x')).unwrap().id;

        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: x_dot,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: x_dot,
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }))
            .unwrap();

        assert_eq!(
            state
                .view()
                .leaf(x_dot)
                .unwrap()
                .effective()
                .get(&ModifierType::Bold),
            Some(&Modifier::Bold)
        );

        let remove_span = EditOp::Span(SpanOp::RemoveSpan {
            start: Anchor {
                id: x_dot,
                bias: Bias::Before,
            },
            end: Anchor {
                id: x_dot,
                bias: Bias::After,
            },
            modifier_type: ModifierType::Bold,
        });
        let ro = record_op(&mut state, remove_span);

        assert_eq!(
            state
                .view()
                .leaf(x_dot)
                .unwrap()
                .effective()
                .get(&ModifierType::Bold),
            None,
            "RemoveSpan must have removed Bold"
        );

        let mut history = UndoHistory::new(Duration::from_secs(1));
        history.record(single_entry(ro), Instant::now());
        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state
                .view()
                .leaf(x_dot)
                .unwrap()
                .effective()
                .get(&ModifierType::Bold),
            Some(&Modifier::Bold),
            "undo of RemoveSpan must restore Bold (proves prior SpanModifier capture)"
        );
    }

    #[test]
    fn block_modifier_set_undo_restores_to_none() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();

        assert_eq!(
            state
                .block_modifiers()
                .modifiers_of(para)
                .get(&ModifierType::Alignment),
            None
        );

        let ro = record_op(
            &mut state,
            EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: para,
                modifier: Modifier::Alignment {
                    value: editor_model::Alignment::Center,
                },
            }),
        );

        assert!(
            state
                .block_modifiers()
                .modifiers_of(para)
                .contains_key(&ModifierType::Alignment)
        );

        let mut history = UndoHistory::new(Duration::from_secs(1));
        history.record(single_entry(ro), Instant::now());
        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state
                .block_modifiers()
                .modifiers_of(para)
                .get(&ModifierType::Alignment),
            None,
            "undo of SetModifier(Alignment) must clear it back to None"
        );
    }

    #[test]
    fn node_style_set_undo_restores_to_prior() {
        let mut state = ProjectedState::empty();

        let x_dot = state.apply(seq_char(1, 'x')).unwrap().id;

        assert_eq!(state.node_styles().value_of(x_dot), None);

        let ro = record_op(
            &mut state,
            EditOp::NodeStyle(NodeLwwOp {
                target: x_dot,
                op: LwwRegOp::Set {
                    value: Some("my-style".to_string()),
                },
            }),
        );

        assert_eq!(
            state.node_styles().value_of(x_dot),
            Some("my-style".to_string())
        );

        let mut history = UndoHistory::new(Duration::from_secs(1));
        history.record(single_entry(ro), Instant::now());
        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state.node_styles().value_of(x_dot),
            None,
            "undo of NodeStyle Set must restore prior None"
        );
    }

    #[test]
    fn node_attr_set_undo_restores_prior_variant() {
        let mut state = ProjectedState::empty();
        let root = state.view().root().unwrap().dot().unwrap();

        let callout = state
            .apply(seq_block(1, NodeType::Callout, vec![root]))
            .unwrap()
            .id;
        state
            .apply(seq_block(2, NodeType::Paragraph, vec![root, callout]))
            .unwrap();

        state
            .apply(EditOp::NodeAttr(NodeAttrOp {
                target: callout,
                attr: NodeAttr::Callout {
                    attr: CalloutNodeAttr::Variant(CalloutVariant::Info),
                },
            }))
            .unwrap();

        let ro = record_op(
            &mut state,
            EditOp::NodeAttr(NodeAttrOp {
                target: callout,
                attr: NodeAttr::Callout {
                    attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
                },
            }),
        );

        let get_variant = |s: &ProjectedState| {
            s.node_attrs()
                .attrs_of(callout, NodeType::Callout)
                .to_plain()
                .to_attrs()
                .into_iter()
                .find_map(|a| {
                    if let NodeAttr::Callout {
                        attr: CalloutNodeAttr::Variant(v),
                    } = a
                    {
                        Some(v)
                    } else {
                        None
                    }
                })
        };

        assert_eq!(get_variant(&state), Some(CalloutVariant::Warning));

        let mut history = UndoHistory::new(Duration::from_secs(1));
        history.record(single_entry(ro), Instant::now());
        history.undo(&mut state, TransientState::default());

        assert_eq!(
            get_variant(&state),
            Some(CalloutVariant::Info),
            "undo of NodeAttr must restore prior variant (Info)"
        );
    }

    #[test]
    fn style_name_set_undo_restores_prior_name() {
        let mut state = ProjectedState::empty();
        let style_id = "s1".to_string();

        state
            .apply(EditOp::Style(StyleRegOp {
                style_id: style_id.clone(),
                op: StyleOp::Presence(OrMapOp::Set {
                    key: style_id.clone(),
                    value: (),
                }),
            }))
            .unwrap();

        let name_op_first = EditOp::Style(StyleRegOp {
            style_id: style_id.clone(),
            op: StyleOp::Name(LwwRegOp::Set {
                value: "first-name".to_string(),
            }),
        });
        state.apply(name_op_first).unwrap();

        assert_eq!(
            state
                .styles()
                .style_entry(&style_id)
                .map(|e| e.name.get().clone()),
            Some("first-name".to_string())
        );

        let ro = record_op(
            &mut state,
            EditOp::Style(StyleRegOp {
                style_id: style_id.clone(),
                op: StyleOp::Name(LwwRegOp::Set {
                    value: "second-name".to_string(),
                }),
            }),
        );

        assert_eq!(
            state
                .styles()
                .style_entry(&style_id)
                .map(|e| e.name.get().clone()),
            Some("second-name".to_string())
        );

        let mut history = UndoHistory::new(Duration::from_secs(1));
        history.record(single_entry(ro), Instant::now());
        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state
                .styles()
                .style_entry(&style_id)
                .map(|e| e.name.get().clone()),
            Some("first-name".to_string()),
            "undo of Style Name Set must restore the prior name"
        );
    }

    #[test]
    fn style_modifiers_add_undo_removes_modifier() {
        let mut state = ProjectedState::empty();
        let style_id = "s2".to_string();

        state
            .apply(EditOp::Style(StyleRegOp {
                style_id: style_id.clone(),
                op: StyleOp::Presence(OrMapOp::Set {
                    key: style_id.clone(),
                    value: (),
                }),
            }))
            .unwrap();

        let ro = record_op(
            &mut state,
            EditOp::Style(StyleRegOp {
                style_id: style_id.clone(),
                op: StyleOp::Modifiers(OrSetOp::Add {
                    elem: Modifier::Italic,
                }),
            }),
        );

        let has_italic = |s: &ProjectedState| {
            s.styles()
                .style_entry(&style_id)
                .map(|e| e.modifiers.contains(&Modifier::Italic))
                .unwrap_or(false)
        };

        assert!(has_italic(&state), "Italic must be present after Add");

        let mut history = UndoHistory::new(Duration::from_secs(1));
        history.record(single_entry(ro), Instant::now());
        history.undo(&mut state, TransientState::default());

        assert!(
            !has_italic(&state),
            "undo of Style Modifiers Add must remove Italic"
        );
    }

    #[test]
    fn coalescing_within_interval_merges_into_one_undo_unit() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();

        let now = Instant::now();
        let interval = Duration::from_millis(500);
        let mut history = UndoHistory::new(interval);

        let ro_a = record_op(&mut state, seq_char(1, 'a'));
        history.record(single_entry(ro_a), now);

        let ro_b = record_op(&mut state, seq_char(2, 'b'));
        history.record(single_entry(ro_b), now + Duration::from_millis(100));

        assert_eq!(state.view().node(para).unwrap().inline_text(), "ab");

        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state.view().node(para).unwrap().inline_text(),
            "",
            "one undo must revert both ops because they were within the merge interval"
        );
    }

    #[test]
    fn coalescing_beyond_interval_creates_separate_undo_units() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();

        let now = Instant::now();
        let interval = Duration::from_millis(500);
        let mut history = UndoHistory::new(interval);

        let ro_a = record_op(&mut state, seq_char(1, 'a'));
        history.record(single_entry(ro_a), now);

        let ro_b = record_op(&mut state, seq_char(2, 'b'));
        let t1 = now + Duration::from_millis(100);
        history.record(single_entry(ro_b), t1);

        let ro_c = record_op(&mut state, seq_char(3, 'c'));
        history.record(single_entry(ro_c), t1 + interval + Duration::from_millis(1));

        assert_eq!(state.view().node(para).unwrap().inline_text(), "abc");

        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state.view().node(para).unwrap().inline_text(),
            "ab",
            "first undo must only revert 'c' (separate unit, beyond interval from last push)"
        );

        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state.view().node(para).unwrap().inline_text(),
            "",
            "second undo must revert 'a' and 'b' together (merged unit)"
        );
    }

    #[test]
    fn tagged_entry_is_never_merged() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();

        let now = Instant::now();
        let interval = Duration::from_millis(500);
        let mut history = UndoHistory::new(interval);

        let ro_a = record_op(&mut state, seq_char(1, 'a'));
        let tagged = UndoEntry {
            ops: vec![ro_a],
            tag: Some(super::HistoryTag::AutoReplacement),
            transient: TransientState::default(),
        };
        history.record(tagged, now);

        let ro_b = record_op(&mut state, seq_char(2, 'b'));
        history.record(single_entry(ro_b), now + Duration::from_millis(10));

        assert_eq!(state.view().node(para).unwrap().inline_text(), "ab");

        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state.view().node(para).unwrap().inline_text(),
            "a",
            "tagged entry must not merge: first undo reverts only 'b'"
        );

        history.undo(&mut state, TransientState::default());

        assert_eq!(
            state.view().node(para).unwrap().inline_text(),
            "",
            "second undo reverts the tagged 'a' entry"
        );
    }

    // Regression: redoing a range deletion after a CONCURRENT op independently
    // deleted one element inside that range. The original deletion removed three
    // elements, but after undo only two are visible (the third stays deleted by
    // the concurrent op). Redo must re-delete exactly the still-visible targets,
    // not blindly re-delete the original count (which overruns the sequence and
    // panics with "del target exists" / "range delete out of bounds").
    #[test]
    fn redo_deletion_after_concurrent_inner_delete_redeletes_only_visible_targets() {
        use editor_crdt::{Changeset, Dot, Op, OpGraph};

        fn seed_block() -> EditOp {
            EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            })
        }
        fn ins(pos: usize, c: char) -> EditOp {
            EditOp::Seq(ListOp::Ins {
                pos,
                item: SeqItem::Char(c),
            })
        }

        // Actor 1 authors the shared base "abc" (seq flat: [P, a, b, c]).
        let mut ga = OpGraph::<EditOp>::with_actor(1);
        ga.add_mut(seed_block()).unwrap();
        ga.add_mut(ins(1, 'a')).unwrap();
        ga.add_mut(ins(2, 'b')).unwrap();
        ga.add_mut(ins(3, 'c')).unwrap();
        ga.commit_mut();
        let base: Vec<Changeset<EditOp>> = ga.changesets_as_vec();

        // Actor 2 shares the base, then concurrently deletes just 'b' (flat 2).
        let mut gb = OpGraph::<EditOp>::with_actor(2);
        for cs in &base {
            gb = gb.receive_changeset(cs.clone()).unwrap();
        }
        gb.add_mut(EditOp::Seq(ListOp::Del { pos: 2, len: 1 }))
            .unwrap();
        gb.commit_mut();
        let del_b_cs = gb.changesets_as_vec().last().unwrap().clone();

        // Actor 1 concurrently deletes the whole range "abc" (flat 1, len 3).
        let del_a = ga
            .add_mut(EditOp::Seq(ListOp::Del { pos: 1, len: 3 }))
            .unwrap()
            .id;
        ga.commit_mut();

        // Merge actor 2's concurrent deletion into actor 1's graph.
        ga = ga.receive_changeset(del_b_cs).unwrap();

        let mut state = ProjectedState::from_graph(ga).unwrap();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        assert_eq!(state.view().node(para).unwrap().inline_text(), "");

        // History holds actor 1's range deletion (Seq ops carry no prior).
        let mut history = UndoHistory::new(Duration::from_secs(0));
        let del_a_ro = RecordedOp {
            op: Op {
                id: del_a,
                parents: vec![],
                payload: EditOp::Seq(ListOp::Del { pos: 1, len: 3 }),
            },
            prior: None,
        };
        history.record(single_entry(del_a_ro), Instant::now());

        // Undo restores a and c; b stays deleted by the concurrent op.
        history
            .undo(&mut state, TransientState::default())
            .expect("undo applies");
        assert_eq!(state.view().node(para).unwrap().inline_text(), "ac");

        // Redo must re-delete the still-visible targets without panicking.
        history
            .redo(&mut state, TransientState::default())
            .expect("redo applies");
        assert_eq!(state.view().node(para).unwrap().inline_text(), "");
    }

    // Regression: undoing an insertion whose character was already deleted by a
    // *remote* op. The char is a tombstone, so `seq_flat_pos` still reports its
    // boundary position (== visible length when it was the last element). Emitting
    // `Del { pos, len: 1 }` there overruns the sequence ("del target exists").
    // Undoing an insertion of an already-gone char must be a no-op.
    #[test]
    fn undo_insert_of_remotely_deleted_trailing_char_is_noop() {
        use editor_crdt::{Changeset, Dot, OpGraph};

        fn seed_block() -> EditOp {
            EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            })
        }

        // Actor 1 seeds an empty paragraph and types a trailing 'x' (flat pos 1).
        let mut ga = OpGraph::<EditOp>::with_actor(1);
        ga.add_mut(seed_block()).unwrap();
        let ins_x = ga
            .add_mut(EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('x'),
            }))
            .unwrap();
        ga.commit_mut();
        let base: Vec<Changeset<EditOp>> = ga.changesets_as_vec();

        // Actor 2 receives that, then deletes 'x'. (A remote deletion that actor 1
        // cannot undo from its own history.)
        let mut gb = OpGraph::<EditOp>::with_actor(2);
        for cs in &base {
            gb = gb.receive_changeset(cs.clone()).unwrap();
        }
        gb.add_mut(EditOp::Seq(ListOp::Del { pos: 1, len: 1 }))
            .unwrap();
        gb.commit_mut();
        let del_x_cs = gb.changesets_as_vec().last().unwrap().clone();

        // Actor 1 receives the remote deletion: 'x' is now a tombstone.
        ga = ga.receive_changeset(del_x_cs).unwrap();
        let mut state = ProjectedState::from_graph(ga).unwrap();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        assert_eq!(state.view().node(para).unwrap().inline_text(), "");

        // Actor 1's history still holds its insertion of 'x'.
        let mut history = UndoHistory::new(Duration::from_secs(0));
        history.record(
            single_entry(RecordedOp {
                op: ins_x,
                prior: None,
            }),
            Instant::now(),
        );

        // Undoing the insertion of an already-removed char must not panic.
        history
            .undo(&mut state, TransientState::default())
            .expect("undo applies");
        assert_eq!(state.view().node(para).unwrap().inline_text(), "");
    }
}

/// Faithful end-to-end fuzz of the REAL `UndoHistory`/`invert`/`undo`/`redo`
/// under two-actor concurrency. Unlike a hand-modelled redo, this drives the
/// production code paths, so it guards every `invert` arm (Ins-undo, Undel-redo,
/// …) against out-of-bounds `Del`s and other panics. The property: no schedule
/// of edits + undo/redo + sync ever panics, and both actors converge once they
/// hold every changeset.
#[cfg(test)]
mod concurrency_proptest {
    use editor_common::time::Instant;
    use editor_crdt::sequence::checkout;
    use editor_crdt::{Changeset, Dot, ListOp, OpGraph};
    use editor_model::{EditOp, NodeType, SeqItem};
    use proptest::prelude::*;
    use std::time::Duration;

    use crate::projected_state::ProjectedState;
    use crate::undo::{RecordedOp, TransientState, UndoEntry, UndoHistory};

    #[derive(Clone, Debug)]
    enum Cmd {
        Type { on_a: bool, pos: u16, ch: u8 },
        Del { on_a: bool, pos: u16, len: u16 },
        Undo { on_a: bool },
        Redo { on_a: bool },
        Sync { into_a: bool },
    }

    fn mk_client(actor: u64, base: &[Changeset<EditOp>]) -> ProjectedState {
        let mut g = OpGraph::<EditOp>::with_actor(actor);
        for cs in base {
            g = g.receive_changeset(cs.clone()).expect("base applies");
        }
        ProjectedState::from_graph(g).expect("base projects")
    }

    fn seq_len(state: &ProjectedState) -> usize {
        checkout(state.seq()).len()
    }

    fn para_text(state: &ProjectedState, para: Dot) -> String {
        state
            .view()
            .node(para)
            .map(|n| n.inline_text())
            .unwrap_or_default()
    }

    fn record_single(hist: &mut UndoHistory, op: editor_crdt::Op<EditOp>) {
        hist.record(
            UndoEntry {
                ops: vec![RecordedOp { op, prior: None }],
                tag: None,
                transient: TransientState::default(),
            },
            Instant::now(),
        );
    }

    fn collect_new(
        registry: &mut Vec<Changeset<EditOp>>,
        seen: &mut std::collections::HashSet<Dot>,
        state: &ProjectedState,
    ) {
        for cs in state.graph().changesets_as_vec() {
            let key = cs.ops[0].id;
            if seen.insert(key) {
                registry.push(cs);
            }
        }
    }

    // Registry is in commit order (a valid topological order), so a changeset's
    // parents always precede it and are delivered first.
    fn deliver(registry: &[Changeset<EditOp>], state: &mut ProjectedState) {
        for cs in registry {
            if !state.graph().contains(&cs.ops[0].id) {
                if let Ok(next) = state.receive_changeset(cs.clone()) {
                    *state = next;
                }
            }
        }
    }

    fn run(cmds: &[Cmd]) {
        // Shared base: a third actor seeds the paragraph both clients build on.
        let mut base_graph = OpGraph::<EditOp>::with_actor(0);
        base_graph
            .add_mut(EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            }))
            .unwrap();
        base_graph.commit_mut();
        let base = base_graph.changesets_as_vec();

        let mut a = mk_client(1, &base);
        let mut b = mk_client(2, &base);
        let mut ha = UndoHistory::new(Duration::from_secs(0));
        let mut hb = UndoHistory::new(Duration::from_secs(0));
        let para = a
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();

        let mut registry: Vec<Changeset<EditOp>> = base.clone();
        let mut seen: std::collections::HashSet<Dot> = base.iter().map(|c| c.ops[0].id).collect();

        for cmd in cmds {
            match *cmd {
                Cmd::Type { on_a, pos, ch } => {
                    let (st, hist) = if on_a {
                        (&mut a, &mut ha)
                    } else {
                        (&mut b, &mut hb)
                    };
                    let pos = 1 + (pos as usize) % seq_len(st); // after the block marker
                    let ch = (b'a' + (ch % 26)) as char;
                    if let Ok(op) = st.apply(EditOp::Seq(ListOp::Ins {
                        pos,
                        item: SeqItem::Char(ch),
                    })) {
                        record_single(hist, op);
                        st.commit();
                    }
                }
                Cmd::Del { on_a, pos, len } => {
                    let (st, hist) = if on_a {
                        (&mut a, &mut ha)
                    } else {
                        (&mut b, &mut hb)
                    };
                    let slen = seq_len(st);
                    if slen < 2 {
                        continue; // only the block — no chars to delete
                    }
                    let pos = 1 + (pos as usize) % (slen - 1);
                    let len = 1 + (len as usize) % (slen - pos);
                    if let Ok(op) = st.apply(EditOp::Seq(ListOp::Del { pos, len })) {
                        record_single(hist, op);
                        st.commit();
                    }
                }
                Cmd::Undo { on_a } => {
                    let (st, hist) = if on_a {
                        (&mut a, &mut ha)
                    } else {
                        (&mut b, &mut hb)
                    };
                    hist.undo(st, TransientState::default());
                    st.commit();
                }
                Cmd::Redo { on_a } => {
                    let (st, hist) = if on_a {
                        (&mut a, &mut ha)
                    } else {
                        (&mut b, &mut hb)
                    };
                    hist.redo(st, TransientState::default());
                    st.commit();
                }
                Cmd::Sync { into_a } => {
                    collect_new(&mut registry, &mut seen, &a);
                    collect_new(&mut registry, &mut seen, &b);
                    if into_a {
                        deliver(&registry, &mut a);
                    } else {
                        deliver(&registry, &mut b);
                    }
                }
            }
        }

        // Full mutual sync, then both actors must converge.
        collect_new(&mut registry, &mut seen, &a);
        collect_new(&mut registry, &mut seen, &b);
        deliver(&registry, &mut a);
        deliver(&registry, &mut b);
        assert_eq!(
            para_text(&a, para),
            para_text(&b, para),
            "actors must converge after holding every changeset"
        );
    }

    fn arb_cmd() -> impl Strategy<Value = Cmd> {
        prop_oneof![
            5 => (any::<bool>(), any::<u16>(), any::<u8>())
                .prop_map(|(on_a, pos, ch)| Cmd::Type { on_a, pos, ch }),
            3 => (any::<bool>(), any::<u16>(), any::<u16>())
                .prop_map(|(on_a, pos, len)| Cmd::Del { on_a, pos, len }),
            3 => any::<bool>().prop_map(|on_a| Cmd::Undo { on_a }),
            3 => any::<bool>().prop_map(|on_a| Cmd::Redo { on_a }),
            4 => any::<bool>().prop_map(|into_a| Cmd::Sync { into_a }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 3000, ..ProptestConfig::default() })]

        #[test]
        fn real_undo_redo_under_concurrency_never_panics_and_converges(
            cmds in proptest::collection::vec(arb_cmd(), 0..40)
        ) {
            run(&cmds);
        }
    }
}
