use super::TRACING_COLLECTOR;
use super::collector::TracingCollector;
use super::data::TracingSpanData;
use opentelemetry::trace::{SpanId, TraceId};
use std::collections::VecDeque;

pub struct TracingReporter {
    buffer: VecDeque<Vec<TracingSpanData>>,
    max_buffer_size: usize,
}

impl TracingReporter {
    pub fn new(max_buffer_size: usize) -> Self {
        Self {
            buffer: VecDeque::new(),
            max_buffer_size,
        }
    }

    pub fn set_tracing(&mut self, trace_id: TraceId, parent_span_id: SpanId) {
        self.drain_active();
        TRACING_COLLECTOR.with(|c| {
            *c.borrow_mut() = Some(TracingCollector::new(trace_id, parent_span_id));
        });
    }

    pub fn clear_tracing(&mut self) {
        self.drain_active();
    }

    fn drain_active(&mut self) {
        TRACING_COLLECTOR.with(|c| {
            if let Some(collector) = c.borrow_mut().take() {
                if !collector.spans.is_empty() {
                    if self.buffer.len() >= self.max_buffer_size {
                        self.buffer.pop_front();
                    }
                    self.buffer.push_back(collector.spans);
                }
            }
        });
    }

    pub fn drain(&mut self) -> Vec<Vec<TracingSpanData>> {
        self.drain_active();
        self.buffer.drain(..).collect()
    }
}

impl Default for TracingReporter {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_clear_trace_context() {
        let mut reporter = TracingReporter::new(10);
        let trace_id = TraceId::from_bytes([1; 16]);
        let span_id = SpanId::from_bytes([2; 8]);

        reporter.set_tracing(trace_id, span_id);
        let active = TRACING_COLLECTOR.with(|c| c.borrow().is_some());
        assert!(active);

        reporter.clear_tracing();
        let active = TRACING_COLLECTOR.with(|c| c.borrow().is_some());
        assert!(!active);
    }

    #[test]
    fn drain_returns_buffered_spans() {
        let mut reporter = TracingReporter::new(10);
        let trace_id = TraceId::from_bytes([1; 16]);
        let span_id = SpanId::from_bytes([2; 8]);

        reporter.set_tracing(trace_id, span_id);

        TRACING_COLLECTOR.with(|c| {
            if let Some(collector) = c.borrow_mut().as_mut() {
                collector.push(TracingSpanData {
                    trace_id: "test".into(),
                    span_id: "test".into(),
                    parent_span_id: "test".into(),
                    name: "test.span".into(),
                    kind: 1,
                    start_time: (0, 0),
                    end_time: (0, 1000),
                    duration: (0, 1000),
                    status: Default::default(),
                    attributes: vec![],
                    events: vec![],
                });
            }
        });

        reporter.clear_tracing();
        let traces = reporter.drain();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].len(), 1);
        assert_eq!(traces[0][0].name, "test.span");
    }

    #[test]
    fn max_buffer_evicts_oldest() {
        let mut reporter = TracingReporter::new(2);
        let trace_id = TraceId::from_bytes([1; 16]);
        let span_id = SpanId::from_bytes([2; 8]);

        for i in 0..3 {
            reporter.set_tracing(trace_id, span_id);
            TRACING_COLLECTOR.with(|c| {
                if let Some(collector) = c.borrow_mut().as_mut() {
                    collector.push(TracingSpanData {
                        trace_id: "test".into(),
                        span_id: "test".into(),
                        parent_span_id: "test".into(),
                        name: format!("span.{i}").into(),
                        kind: 1,
                        start_time: (0, 0),
                        end_time: (0, 0),
                        duration: (0, 0),
                        status: Default::default(),
                        attributes: vec![],
                        events: vec![],
                    });
                }
            });
            reporter.clear_tracing();
        }

        let traces = reporter.drain();
        assert_eq!(traces.len(), 2);
        assert_eq!(traces[0][0].name, "span.1");
        assert_eq!(traces[1][0].name, "span.2");
    }
}
