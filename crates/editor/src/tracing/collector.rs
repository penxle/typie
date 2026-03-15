use opentelemetry::trace::{SpanId, TraceId};

use super::data::TracingSpanData;

pub(crate) struct TracingCollector {
    pub(crate) trace_id: TraceId,
    pub(crate) parent_span_id: SpanId,
    next_span_id: u64,
    pub(crate) spans: Vec<TracingSpanData>,
}

impl TracingCollector {
    pub(crate) fn new(trace_id: TraceId, parent_span_id: SpanId) -> Self {
        Self {
            trace_id,
            parent_span_id,
            next_span_id: 0,
            spans: Vec::new(),
        }
    }

    pub(crate) fn next_span_id(&mut self) -> SpanId {
        self.next_span_id += 1;
        SpanId::from_bytes(self.next_span_id.to_be_bytes())
    }

    pub(crate) fn push(&mut self, span: TracingSpanData) {
        self.spans.push(span);
    }
}
