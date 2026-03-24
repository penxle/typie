use std::sync::{Arc, Mutex};

use crate::icu_data::load_icu_data;
use crate::model::{
    CONTINUOUS_PAGE_MARGIN, Doc, DocExportMode, LayoutMode, Node, NodeId, ParagraphNode,
};
use crate::runtime::{Message, Runtime, State};
use crate::state::{Position, Selection};
use crate::types::Affinity;

#[derive(Debug, uniffi::Error)]
pub enum EditorError {
    General { msg: String },
}

impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::General { msg } => write!(f, "{msg}"),
        }
    }
}

impl From<String> for EditorError {
    fn from(msg: String) -> Self {
        Self::General { msg }
    }
}

impl From<anyhow::Error> for EditorError {
    fn from(e: anyhow::Error) -> Self {
        Self::General { msg: e.to_string() }
    }
}

#[derive(uniffi::Object)]
pub struct EditorEngine;

#[uniffi::export]
impl EditorEngine {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn load_icu_data(&self, data: Vec<u8>) -> Result<(), EditorError> {
        load_icu_data(&data).map_err(EditorError::from)
    }

    pub fn create_editor(
        &self,
        scale_factor: f64,
        snapshot: Option<Vec<u8>>,
    ) -> Result<Arc<Editor>, EditorError> {
        let inner = match snapshot {
            Some(data) if !data.is_empty() => EditorInner::with_snapshot(scale_factor, data)?,
            _ => EditorInner::new(scale_factor),
        };
        Ok(Arc::new(Editor {
            inner: Mutex::new(inner),
        }))
    }
}

struct EditorInner {
    runtime: Runtime,
}

impl EditorInner {
    fn new(scale_factor: f64) -> Self {
        let doc = std::rc::Rc::new(Doc::new());
        let width = Self::get_width(&doc);

        let root = doc
            .node(NodeId::ROOT)
            .expect("Doc::new: ROOT node must exist after construction");
        let paragraph_id = root
            .as_mut()
            .insert_child(0, Node::Paragraph(ParagraphNode::default()))
            .expect("Doc::new: failed to insert initial paragraph");

        Self::create(scale_factor, doc, paragraph_id, width)
    }

    fn with_snapshot(scale_factor: f64, snapshot: Vec<u8>) -> Result<Self, EditorError> {
        let doc = std::rc::Rc::new(Doc::from_snapshot(snapshot).map_err(|e| e.to_string())?);
        let width = Self::get_width(&doc);
        Ok(Self::create(scale_factor, doc, NodeId::ROOT, width))
    }

    fn create(scale_factor: f64, doc: std::rc::Rc<Doc>, cursor_node: NodeId, width: f32) -> Self {
        let state = State::new(
            doc,
            Selection::collapsed(Position::new(cursor_node, 0, Affinity::default())),
        );
        let mut runtime = Runtime::new(width, scale_factor, state);
        runtime.layout();
        Self { runtime }
    }

    fn get_width(doc: &Doc) -> f32 {
        match doc.settings().layout_mode {
            LayoutMode::Paginated { page_width, .. } => page_width,
            LayoutMode::Continuous { max_width, .. } => max_width + 2.0 * CONTINUOUS_PAGE_MARGIN,
        }
    }
}

#[derive(uniffi::Object)]
pub struct Editor {
    inner: Mutex<EditorInner>,
}

// Safety: Mutex로 내부적으로 동기화하므로 Send+Sync을 보장한다.
// EditorInner 자체는 Send+Sync이 아니지만 (Rc<Doc>),
// Mutex를 통해 단일 접근만 허용하므로 안전하다.
// EditorInner는 반드시 Mutex를 통해서만 접근해야 한다.
unsafe impl Send for Editor {}
unsafe impl Sync for Editor {}

#[uniffi::export]
impl Editor {
    pub fn dispatch(&self, message_json: String) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let message: Message = serde_json::from_str(&message_json)
            .map_err(|e| format!("Failed to parse message: {e}"))?;
        inner.runtime.enqueue_message(message);
        Ok(())
    }

    pub fn tick(&self) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.runtime.tick();
        Ok(())
    }

    pub fn export_snapshot(&self) -> Result<Vec<u8>, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner
            .runtime
            .export(DocExportMode::Snapshot)
            .map_err(EditorError::from)
    }
}
