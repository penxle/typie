use crate::model::NodeId;
use rustc_hash::FxHashSet;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Default)]
pub(crate) struct FrameDiagnostics {
    inner: Rc<RefCell<FrameDiagnosticsState>>,
}

#[derive(Default)]
struct FrameDiagnosticsState {
    next_layout_revision: u64,
    last_layout_pass: Option<LayoutPassSnapshot>,
}

#[derive(Default)]
pub(crate) struct LayoutPassRecorder {
    recomputed_nodes: FxHashSet<NodeId>,
}

#[derive(Clone)]
pub(crate) struct LayoutPassSnapshot {
    pub(crate) revision: u64,
    pub(crate) recomputed_nodes: Rc<FxHashSet<NodeId>>,
}

impl FrameDiagnostics {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn commit_layout_pass(&self, pass: LayoutPassRecorder) {
        let mut state = self.inner.borrow_mut();
        state.next_layout_revision = state.next_layout_revision.wrapping_add(1);
        state.last_layout_pass = Some(LayoutPassSnapshot {
            revision: state.next_layout_revision,
            recomputed_nodes: Rc::new(pass.into_recomputed_nodes()),
        });
    }

    pub(crate) fn clear_layout_pass(&self) {
        self.inner.borrow_mut().last_layout_pass = None;
    }

    pub(crate) fn layout_pass_snapshot(&self) -> Option<LayoutPassSnapshot> {
        self.inner.borrow().last_layout_pass.clone()
    }
}

impl LayoutPassRecorder {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn record_recomputed(&mut self, node_id: NodeId) {
        self.recomputed_nodes.insert(node_id);
    }

    pub(crate) fn into_recomputed_nodes(self) -> FxHashSet<NodeId> {
        self.recomputed_nodes
    }
}
