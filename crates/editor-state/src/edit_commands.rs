use editor_crdt::Dot;
use editor_model::{Anchor, Bias, EditOp, Modifier, SpanOp};

pub(crate) fn add_modifier_span(first: Dot, last: Dot, modifier: Modifier) -> EditOp {
    EditOp::Span(SpanOp::AddSpan {
        start: Anchor {
            id: first,
            bias: Bias::Before,
        },
        end: Anchor {
            id: last,
            bias: Bias::After,
        },
        modifier,
    })
}
