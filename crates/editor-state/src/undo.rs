use std::time::Duration;

use editor_common::HistoryTag;
use editor_common::time::Instant;

use editor_crdt::{ListOp, LwwRegOp, Op, OrMapOp, OrSetOp};
use editor_model::{
    EditOp, Marker, Modifier, ModifierAttrOp, NodeAttr, NodeAttrOp, NodeLwwOp, NodeType, SpanOp,
    StyleOp, StyleRegOp,
};

use crate::Selection;
use crate::projected_state::ProjectedState;
use crate::{Composition, PendingModifiers, PendingStyle};

/// Editor state restored alongside a doc undo/redo: the caret/selection and the
/// transient IME/pending overlays that the op-level history does not encode as
/// document ops. Recorded as the pre-transaction state; restored on undo.
#[derive(Clone, Default, PartialEq)]
pub struct TransientState {
    pub selection: Option<Selection>,
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
        for ro in entry.ops.into_iter().rev() {
            let Some(inv_payload) = invert(state, &ro) else {
                continue;
            };
            let prior_for_redo = capture_prior(state, &inv_payload);
            let Ok(inv_op) = state.apply(inv_payload) else {
                break;
            };
            applied.push(inv_op.clone());
            redo_ops.push(RecordedOp {
                op: inv_op,
                prior: prior_for_redo,
            });
        }
        redo_ops.reverse();
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
        for ro in entry.ops.into_iter().rev() {
            let Some(inv_payload) = invert(state, &ro) else {
                continue;
            };
            let prior_for_undo = capture_prior(state, &inv_payload);
            let Ok(inv_op) = state.apply(inv_payload) else {
                break;
            };
            applied.push(inv_op.clone());
            undo_ops.push(RecordedOp {
                op: inv_op,
                prior: prior_for_undo,
            });
        }
        undo_ops.reverse();
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

pub fn invert(state: &ProjectedState, ro: &RecordedOp) -> Option<EditOp> {
    let dot = ro.op.id;
    match &ro.op.payload {
        EditOp::Seq(ListOp::Ins { .. }) => {
            let pos = state.seq_flat_pos(dot)?;
            Some(EditOp::Seq(ListOp::Del { pos, len: 1 }))
        }
        EditOp::Seq(ListOp::Del { .. }) => Some(EditOp::Seq(ListOp::Undel { del: dot })),
        EditOp::Seq(ListOp::Undel { del }) => {
            let (pos, len) = state.del_target_span(*del)?;
            Some(EditOp::Seq(ListOp::Del { pos, len }))
        }
        EditOp::Span(SpanOp::AddSpan {
            start,
            end,
            modifier,
        }) => Some(EditOp::Span(SpanOp::RemoveSpan {
            start: *start,
            end: *end,
            modifier_type: modifier.as_type(),
        })),
        EditOp::Span(SpanOp::RemoveSpan { start, end, .. }) => {
            let modifier = match &ro.prior {
                Some(PriorValue::SpanModifier(m)) => m.clone(),
                _ => return None,
            };
            Some(EditOp::Span(SpanOp::AddSpan {
                start: *start,
                end: *end,
                modifier,
            }))
        }
        EditOp::BlockModifier(ModifierAttrOp::SetModifier { target, modifier }) => {
            match &ro.prior {
                Some(PriorValue::BlockModifier(Some(prior_m))) => {
                    Some(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                        target: *target,
                        modifier: prior_m.clone(),
                    }))
                }
                Some(PriorValue::BlockModifier(None)) => {
                    Some(EditOp::BlockModifier(ModifierAttrOp::ClearModifier {
                        target: *target,
                        key: modifier.as_type(),
                    }))
                }
                _ => None,
            }
        }
        EditOp::BlockModifier(ModifierAttrOp::ClearModifier { target, key: _ }) => {
            match &ro.prior {
                Some(PriorValue::BlockModifier(Some(prior_m))) => {
                    Some(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                        target: *target,
                        modifier: prior_m.clone(),
                    }))
                }
                Some(PriorValue::BlockModifier(None)) => None,
                _ => None,
            }
        }
        EditOp::NodeStyle(NodeLwwOp { target, .. }) => {
            let prior = match &ro.prior {
                Some(PriorValue::NodeStyle(v)) => v.clone(),
                _ => return None,
            };
            Some(EditOp::NodeStyle(NodeLwwOp {
                target: *target,
                op: LwwRegOp::Set { value: prior },
            }))
        }
        EditOp::NodeMarker(NodeLwwOp { target, .. }) => {
            let prior = match &ro.prior {
                Some(PriorValue::NodeMarker(v)) => v.clone(),
                _ => return None,
            };
            Some(EditOp::NodeMarker(NodeLwwOp {
                target: *target,
                op: LwwRegOp::Set { value: prior },
            }))
        }
        EditOp::NodeAttr(NodeAttrOp { target, .. }) => {
            let prior_attr = match &ro.prior {
                Some(PriorValue::NodeAttr(a)) => a.clone(),
                _ => return None,
            };
            Some(EditOp::NodeAttr(NodeAttrOp {
                target: *target,
                attr: prior_attr,
            }))
        }
        EditOp::Style(StyleRegOp {
            style_id,
            op: StyleOp::Name(_),
        }) => {
            let prior_name = match &ro.prior {
                Some(PriorValue::StyleName(n)) => n.clone(),
                _ => return None,
            };
            Some(EditOp::Style(StyleRegOp {
                style_id: style_id.clone(),
                op: StyleOp::Name(LwwRegOp::Set { value: prior_name }),
            }))
        }
        EditOp::Style(StyleRegOp {
            style_id,
            op: StyleOp::Modifiers(OrSetOp::Add { .. }),
        }) => Some(EditOp::Style(StyleRegOp {
            style_id: style_id.clone(),
            op: StyleOp::Modifiers(OrSetOp::Remove { observed: dot }),
        })),
        EditOp::Style(StyleRegOp {
            style_id,
            op: StyleOp::Modifiers(OrSetOp::Remove { .. }),
        }) => {
            let prior_mod = match &ro.prior {
                Some(PriorValue::StyleModifier(m)) => m.clone(),
                _ => return None,
            };
            Some(EditOp::Style(StyleRegOp {
                style_id: style_id.clone(),
                op: StyleOp::Modifiers(OrSetOp::Add { elem: prior_mod }),
            }))
        }
        EditOp::Style(StyleRegOp {
            style_id,
            op: StyleOp::Presence(OrMapOp::Set { key: _, .. }),
        }) => Some(EditOp::Style(StyleRegOp {
            style_id: style_id.clone(),
            op: StyleOp::Presence(OrMapOp::Unset {
                observed: vec![dot],
            }),
        })),
        EditOp::Style(StyleRegOp {
            style_id,
            op: StyleOp::Presence(OrMapOp::Unset { .. }),
        }) => Some(EditOp::Style(StyleRegOp {
            style_id: style_id.clone(),
            op: StyleOp::Presence(OrMapOp::Set {
                key: style_id.clone(),
                value: (),
            }),
        })),
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
}
