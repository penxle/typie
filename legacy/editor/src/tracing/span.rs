use crate::tracing::TRACING_COLLECTOR;
use crate::tracing::data::{
    SpanAttribute, SpanAttributeValue, SpanEvent, SpanStatus, TracingSpanData,
};
use opentelemetry::KeyValue;
use opentelemetry::trace::{self, SpanContext, SpanId, SpanKind};
use std::borrow::Cow;
use std::time::SystemTime;

static INVALID_SPAN_CONTEXT: SpanContext = SpanContext::NONE;

pub struct TracingSpan {
    pub(crate) inner: Option<TracingSpanInner>,
}

pub(crate) struct TracingSpanInner {
    pub(crate) span_context: SpanContext,
    pub(crate) parent_span_id: SpanId,
    pub(crate) name: Cow<'static, str>,
    pub(crate) kind: SpanKind,
    pub(crate) start_time: SystemTime,
    pub(crate) status: trace::Status,
    pub(crate) attributes: Vec<KeyValue>,
    pub(crate) events: Vec<(Cow<'static, str>, SystemTime, Vec<KeyValue>)>,
}

impl TracingSpan {
    pub(crate) fn non_recording() -> Self {
        Self { inner: None }
    }
}

impl Drop for TracingSpan {
    fn drop(&mut self) {
        if self.inner.is_some() {
            trace::Span::end(self);
        }
    }
}

fn system_time_to_tuple(t: SystemTime) -> (u64, u32) {
    t.duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| (d.as_secs(), d.subsec_nanos()))
        .unwrap_or((0, 0))
}

fn convert_attribute(kv: &KeyValue) -> SpanAttribute {
    SpanAttribute {
        key: Cow::Owned(kv.key.as_str().to_owned()),
        value: match &kv.value {
            opentelemetry::Value::Bool(v) => SpanAttributeValue::Bool(*v),
            opentelemetry::Value::I64(v) => SpanAttributeValue::I64(*v),
            opentelemetry::Value::F64(v) => SpanAttributeValue::F64(*v),
            opentelemetry::Value::String(v) => SpanAttributeValue::String(v.as_str().to_owned()),
            opentelemetry::Value::Array(_) => SpanAttributeValue::String("[array]".to_owned()),
            _ => SpanAttributeValue::String(format!("{}", kv.value)),
        },
    }
}

fn convert_status(status: &trace::Status) -> SpanStatus {
    match status {
        trace::Status::Unset => SpanStatus::default(),
        trace::Status::Ok => SpanStatus {
            code: 1,
            message: Cow::Borrowed(""),
        },
        trace::Status::Error { description } => SpanStatus {
            code: 2,
            message: Cow::Owned(description.to_string()),
        },
    }
}

impl trace::Span for TracingSpan {
    fn add_event_with_timestamp<T>(
        &mut self,
        name: T,
        timestamp: SystemTime,
        attributes: Vec<KeyValue>,
    ) where
        T: Into<Cow<'static, str>>,
    {
        if let Some(inner) = &mut self.inner {
            inner.events.push((name.into(), timestamp, attributes));
        }
    }

    fn span_context(&self) -> &SpanContext {
        self.inner
            .as_ref()
            .map(|i| &i.span_context)
            .unwrap_or(&INVALID_SPAN_CONTEXT)
    }

    fn is_recording(&self) -> bool {
        self.inner.is_some()
    }

    fn set_attribute(&mut self, attribute: KeyValue) {
        if let Some(inner) = &mut self.inner {
            inner.attributes.push(attribute);
        }
    }

    fn set_status(&mut self, status: trace::Status) {
        if let Some(inner) = &mut self.inner {
            inner.status = status;
        }
    }

    fn update_name<T>(&mut self, new_name: T)
    where
        T: Into<Cow<'static, str>>,
    {
        if let Some(inner) = &mut self.inner {
            inner.name = new_name.into();
        }
    }

    fn add_link(&mut self, _span_context: SpanContext, _attributes: Vec<KeyValue>) {}

    fn end(&mut self) {
        self.end_with_timestamp(crate::tracing::now());
    }

    fn end_with_timestamp(&mut self, timestamp: SystemTime) {
        if let Some(inner) = self.inner.take() {
            let kind = match inner.kind {
                SpanKind::Internal => 0,
                SpanKind::Server => 1,
                SpanKind::Client => 2,
                SpanKind::Producer => 3,
                SpanKind::Consumer => 4,
            };

            let span_data = TracingSpanData {
                trace_id: format!(
                    "{:032x}",
                    u128::from_be_bytes(inner.span_context.trace_id().to_bytes())
                ),
                span_id: format!(
                    "{:016x}",
                    u64::from_be_bytes(inner.span_context.span_id().to_bytes())
                ),
                parent_span_id: format!(
                    "{:016x}",
                    u64::from_be_bytes(inner.parent_span_id.to_bytes())
                ),
                name: inner.name,
                kind,
                start_time: system_time_to_tuple(inner.start_time),
                end_time: system_time_to_tuple(timestamp),
                duration: timestamp
                    .duration_since(inner.start_time)
                    .map(|d| (d.as_secs(), d.subsec_nanos()))
                    .unwrap_or((0, 0)),
                status: convert_status(&inner.status),
                attributes: inner.attributes.iter().map(convert_attribute).collect(),
                events: inner
                    .events
                    .into_iter()
                    .map(|(name, ts, attrs)| SpanEvent {
                        name,
                        time: system_time_to_tuple(ts),
                        attributes: attrs.iter().map(convert_attribute).collect(),
                    })
                    .collect(),
            };
            TRACING_COLLECTOR.with(|c| {
                if let Some(collector) = c.borrow_mut().as_mut() {
                    collector.push(span_data);
                }
            });
        }
    }
}
