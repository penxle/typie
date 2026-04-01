use serde::Serialize;
use std::borrow::Cow;

#[derive(Debug, Clone, Serialize)]
pub struct TracingSpanData {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: String,
    pub name: Cow<'static, str>,
    pub kind: u8,
    pub start_time: (u64, u32),
    pub end_time: (u64, u32),
    pub duration: (u64, u32),
    pub status: SpanStatus,
    pub attributes: Vec<SpanAttribute>,
    pub events: Vec<SpanEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpanStatus {
    pub code: u8,
    pub message: Cow<'static, str>,
}

impl Default for SpanStatus {
    fn default() -> Self {
        Self {
            code: 0,
            message: Cow::Borrowed(""),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SpanAttribute {
    pub key: Cow<'static, str>,
    pub value: SpanAttributeValue,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SpanAttributeValue {
    String(String),
    Bool(bool),
    I64(i64),
    F64(f64),
}

#[derive(Debug, Clone, Serialize)]
pub struct SpanEvent {
    pub name: Cow<'static, str>,
    pub time: (u64, u32), // (seconds, subsec_nanos)
    pub attributes: Vec<SpanAttribute>,
}
