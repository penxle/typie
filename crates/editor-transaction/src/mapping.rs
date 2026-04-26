use editor_model::NodeId;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mapping {
    actions: Vec<MapAction>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MapAction {
    Insert {
        parent: NodeId,
        start: usize,
        count: usize,
        subtree_id: NodeId,
    },

    Remove {
        parent: NodeId,
        start: usize,
        count: usize,
    },

    TextInsert {
        node: NodeId,
        offset: usize,
        len: usize,
        text: String,
    },

    TextRemove {
        node: NodeId,
        offset: usize,
        len: usize,
    },

    NodeDeleted {
        node: NodeId,
    },
}

impl Mapping {
    pub fn identity() -> Self {
        Self { actions: vec![] }
    }

    pub fn single(action: MapAction) -> Self {
        Self {
            actions: vec![action],
        }
    }

    pub fn compose(&self, other: &Mapping) -> Mapping {
        let mut actions = Vec::with_capacity(self.actions.len() + other.actions.len());
        actions.extend(self.actions.iter().cloned());
        actions.extend(other.actions.iter().cloned());
        Mapping { actions }
    }

    pub fn invert(&self) -> Mapping {
        let actions = self
            .actions
            .iter()
            .rev()
            .filter_map(|a| a.inverse())
            .collect();
        Mapping { actions }
    }

    pub(crate) fn actions(&self) -> &[MapAction] {
        &self.actions
    }

    pub(crate) fn push(&mut self, action: MapAction) {
        self.actions.push(action);
    }

    pub(crate) fn truncate_to(&mut self, len: usize) {
        self.actions.truncate(len);
    }
}

impl MapAction {
    pub(crate) fn inverse(&self) -> Option<MapAction> {
        match self {
            MapAction::Insert {
                parent,
                start,
                count,
                ..
            } => Some(MapAction::Remove {
                parent: *parent,
                start: *start,
                count: *count,
            }),
            MapAction::Remove {
                parent,
                start,
                count,
            } => Some(MapAction::Insert {
                parent: *parent,
                start: *start,
                count: *count,
                subtree_id: NodeId::ROOT,
            }),
            MapAction::TextInsert {
                node, offset, len, ..
            } => Some(MapAction::TextRemove {
                node: *node,
                offset: *offset,
                len: *len,
            }),
            MapAction::TextRemove { node, offset, len } => Some(MapAction::TextInsert {
                node: *node,
                offset: *offset,
                len: *len,
                text: String::new(),
            }),
            MapAction::NodeDeleted { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_has_no_actions() {
        let m = Mapping::identity();
        assert!(m.actions().is_empty());
    }

    #[test]
    fn single_wraps_one_action() {
        let n = NodeId::new();
        let m = Mapping::single(MapAction::NodeDeleted { node: n });
        assert_eq!(m.actions().len(), 1);
    }

    #[test]
    fn compose_concatenates_actions() {
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let a = Mapping::single(MapAction::NodeDeleted { node: n1 });
        let b = Mapping::single(MapAction::NodeDeleted { node: n2 });
        let c = a.compose(&b);
        assert_eq!(c.actions().len(), 2);
    }

    #[test]
    fn compose_with_identity_left_neutral() {
        let n = NodeId::new();
        let m = Mapping::single(MapAction::NodeDeleted { node: n });
        let r = Mapping::identity().compose(&m);
        assert_eq!(r, m);
    }

    #[test]
    fn compose_with_identity_right_neutral() {
        let n = NodeId::new();
        let m = Mapping::single(MapAction::NodeDeleted { node: n });
        let r = m.compose(&Mapping::identity());
        assert_eq!(r, m);
    }

    #[test]
    fn compose_associative() {
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let n3 = NodeId::new();
        let a = Mapping::single(MapAction::NodeDeleted { node: n1 });
        let b = Mapping::single(MapAction::NodeDeleted { node: n2 });
        let c = Mapping::single(MapAction::NodeDeleted { node: n3 });
        let lhs = a.compose(&b).compose(&c);
        let rhs = a.compose(&b.compose(&c));
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn invert_of_identity_is_identity() {
        assert_eq!(Mapping::identity().invert(), Mapping::identity());
    }

    #[test]
    fn invert_reverses_action_order() {
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let m = Mapping::single(MapAction::TextInsert {
            node: n1,
            offset: 0,
            len: 1,
            text: "a".into(),
        })
        .compose(&Mapping::single(MapAction::TextInsert {
            node: n2,
            offset: 0,
            len: 1,
            text: "b".into(),
        }));
        let inv = m.invert();
        assert!(matches!(inv.actions()[0], MapAction::TextRemove { node, .. } if node == n2));
        assert!(matches!(inv.actions()[1], MapAction::TextRemove { node, .. } if node == n1));
    }

    #[test]
    fn invert_drops_node_deleted() {
        let n = NodeId::new();
        let m = Mapping::single(MapAction::NodeDeleted { node: n });
        assert!(m.invert().actions().is_empty());
    }

    #[test]
    fn invert_text_insert_to_text_remove() {
        let n = NodeId::new();
        let m = Mapping::single(MapAction::TextInsert {
            node: n,
            offset: 3,
            len: 2,
            text: "ab".into(),
        });
        let inv = m.invert();
        assert_eq!(
            inv.actions(),
            &[MapAction::TextRemove {
                node: n,
                offset: 3,
                len: 2
            }]
        );
    }

    #[test]
    fn invert_insert_to_remove() {
        let p = NodeId::new();
        let s = NodeId::new();
        let m = Mapping::single(MapAction::Insert {
            parent: p,
            start: 0,
            count: 1,
            subtree_id: s,
        });
        let inv = m.invert();
        assert_eq!(
            inv.actions(),
            &[MapAction::Remove {
                parent: p,
                start: 0,
                count: 1
            }]
        );
    }
}
