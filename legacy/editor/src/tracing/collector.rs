use opentelemetry::trace::{SpanId, TraceId};

use super::data::TracingSpanData;

pub struct TracingCollector {
    pub trace_id: TraceId,
    pub parent_span_id: SpanId,
    next_span_id: u64,
    pub spans: Vec<TracingSpanData>,
}

impl TracingCollector {
    pub fn new(trace_id: TraceId, parent_span_id: SpanId) -> Self {
        Self {
            trace_id,
            parent_span_id,
            next_span_id: 0,
            spans: Vec::new(),
        }
    }

    pub fn next_span_id(&mut self) -> SpanId {
        self.next_span_id += 1;
        SpanId::from_bytes(self.next_span_id.to_be_bytes())
    }

    pub fn push(&mut self, span: TracingSpanData) {
        self.spans.push(span);
    }
}
