use crate::tracing::TRACING_COLLECTOR;
use crate::tracing::span::{TracingSpan, TracingSpanInner};
use opentelemetry::Context;
use opentelemetry::trace::{
    self, SpanBuilder, SpanContext, TraceContextExt, TraceFlags, TraceState,
};

pub struct TracingTracer;

impl trace::Tracer for TracingTracer {
    type Span = TracingSpan;

    fn build_with_context(&self, builder: SpanBuilder, parent_cx: &Context) -> TracingSpan {
        let span_info = TRACING_COLLECTOR.with(|c| {
            let mut borrow = c.borrow_mut();
            let collector = borrow.as_mut()?;
            let parent_span_id = if parent_cx.span().span_context().is_valid() {
                parent_cx.span().span_context().span_id()
            } else {
                collector.parent_span_id
            };
            let span_id = collector.next_span_id();
            Some((collector.trace_id, parent_span_id, span_id))
        });

        let Some((trace_id, parent_span_id, span_id)) = span_info else {
            return TracingSpan::non_recording();
        };

        let span_context = SpanContext::new(
            trace_id,
            span_id,
            TraceFlags::SAMPLED,
            false,
            TraceState::default(),
        );

        TracingSpan {
            inner: Some(TracingSpanInner {
                span_context,
                parent_span_id,
                name: builder.name,
                kind: builder
                    .span_kind
                    .unwrap_or(opentelemetry::trace::SpanKind::Internal),
                start_time: builder.start_time.unwrap_or_else(super::now),
                status: trace::Status::Unset,
                attributes: builder.attributes.unwrap_or_default(),
                events: vec![],
            }),
        }
    }
}
