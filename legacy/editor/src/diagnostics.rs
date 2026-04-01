use crate::model::NodeId;
use rustc_hash::FxHashSet;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Default)]
pub struct FrameDiagnostics {
    inner: Rc<RefCell<FrameDiagnosticsState>>,
}

#[derive(Default)]
struct FrameDiagnosticsState {
    next_layout_revision: u64,
    last_layout_pass: Option<LayoutPassSnapshot>,
}

#[derive(Default)]
pub struct LayoutPassRecorder {
    recomputed_nodes: FxHashSet<NodeId>,
}

#[derive(Clone)]
pub struct LayoutPassSnapshot {
    pub revision: u64,
    pub recomputed_nodes: Rc<FxHashSet<NodeId>>,
}

impl FrameDiagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commit_layout_pass(&self, pass: LayoutPassRecorder) {
        let mut state = self.inner.borrow_mut();
        state.next_layout_revision = state.next_layout_revision.wrapping_add(1);
        state.last_layout_pass = Some(LayoutPassSnapshot {
            revision: state.next_layout_revision,
            recomputed_nodes: Rc::new(pass.into_recomputed_nodes()),
        });
    }

    pub fn clear_layout_pass(&self) {
        self.inner.borrow_mut().last_layout_pass = None;
    }

    pub fn layout_pass_snapshot(&self) -> Option<LayoutPassSnapshot> {
        self.inner.borrow().last_layout_pass.clone()
    }
}

impl LayoutPassRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_recomputed(&mut self, node_id: NodeId) {
        self.recomputed_nodes.insert(node_id);
    }

    pub fn into_recomputed_nodes(self) -> FxHashSet<NodeId> {
        self.recomputed_nodes
    }
}
