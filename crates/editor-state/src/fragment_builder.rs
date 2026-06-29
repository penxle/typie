use std::collections::BTreeMap;

use editor_crdt::{CrdtError, Dot, ListOp, LwwRegOp, Op, OpGraph};
use editor_model::{EditOp, Modifier, ModifierType, NodeLwwOp, SeqItem};

use crate::edit_commands::add_modifier_span;

pub(crate) trait OpSink {
    fn emit(&mut self, op: EditOp) -> Result<Dot, CrdtError>;
}

pub(crate) struct GraphSink<'a> {
    graph: &'a mut OpGraph<EditOp>,
}

impl<'a> GraphSink<'a> {
    pub(crate) fn new(graph: &'a mut OpGraph<EditOp>) -> Self {
        Self { graph }
    }
}

impl OpSink for GraphSink<'_> {
    fn emit(&mut self, op: EditOp) -> Result<Dot, CrdtError> {
        self.graph.add_mut(op).map(|o: Op<EditOp>| o.id)
    }
}

pub(crate) fn emit_text_run(
    sink: &mut impl OpSink,
    seq_pos: &mut usize,
    text: &str,
    modifiers: &BTreeMap<ModifierType, Modifier>,
    style: Option<&str>,
) -> Result<(), CrdtError> {
    let mut char_dots: Vec<Dot> = Vec::new();
    for ch in text.chars() {
        let dot = sink.emit(EditOp::Seq(ListOp::Ins {
            pos: *seq_pos,
            item: SeqItem::Char(ch),
        }))?;
        char_dots.push(dot);
        *seq_pos += 1;
    }
    if let (Some(&first), Some(&last)) = (char_dots.first(), char_dots.last()) {
        for modifier in modifiers.values() {
            sink.emit(add_modifier_span(first, last, modifier.clone()))?;
        }
        if let Some(style_id) = style {
            for &dot in &char_dots {
                sink.emit(EditOp::NodeStyle(NodeLwwOp {
                    target: dot,
                    op: LwwRegOp::Set {
                        value: Some(style_id.to_string()),
                    },
                }))?;
            }
        }
    }
    Ok(())
}
