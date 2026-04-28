use editor_model::NodeId;

use crate::sync::conflict::{
    BranchSide, ConflictBranch, ConflictKind, ConflictRecord, ConflictTarget,
};

pub fn merge_children_order(
    parent_id: NodeId,
    base: &[NodeId],
    ours: &[NodeId],
    theirs: &[NodeId],
) -> (Vec<NodeId>, Option<ConflictRecord>) {
    if ours == theirs {
        return (ours.to_vec(), None);
    }
    if ours == base {
        return (theirs.to_vec(), None);
    }
    if theirs == base {
        return (ours.to_vec(), None);
    }

    let record = ConflictRecord {
        kind: ConflictKind::Order,
        target: ConflictTarget::Order { parent_id },
        base_value: Some(serde_json::to_value(base).unwrap()),
        branches: vec![
            ConflictBranch {
                side: BranchSide::Ours,
                value: serde_json::to_value(ours).unwrap(),
            },
            ConflictBranch {
                side: BranchSide::Theirs,
                value: serde_json::to_value(theirs).unwrap(),
            },
        ],
        auto_resolved: BranchSide::Ours,
    };
    (ours.to_vec(), Some(record))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: usize) -> Vec<NodeId> {
        (0..n).map(|_| NodeId::new()).collect()
    }

    #[test]
    fn same_reorder_no_conflict() {
        let v = ids(3);
        let base = vec![v[0], v[1], v[2]];
        let reordered = vec![v[0], v[2], v[1]];
        let (m, c) = merge_children_order(NodeId::new(), &base, &reordered, &reordered);
        assert_eq!(m, reordered);
        assert!(c.is_none());
    }

    #[test]
    fn one_side_unchanged_takes_other() {
        let v = ids(3);
        let base = vec![v[0], v[1], v[2]];
        let reordered = vec![v[2], v[0], v[1]];
        let (m, c) = merge_children_order(NodeId::new(), &base, &base, &reordered);
        assert_eq!(m, reordered);
        assert!(c.is_none());
    }

    #[test]
    fn different_reorders_conflict() {
        let v = ids(3);
        let parent = NodeId::new();
        let base = vec![v[0], v[1], v[2]];
        let ours = vec![v[1], v[0], v[2]];
        let theirs = vec![v[2], v[0], v[1]];
        let (m, c) = merge_children_order(parent, &base, &ours, &theirs);
        let c = c.unwrap();
        assert_eq!(c.kind, ConflictKind::Order);
        assert_eq!(c.target, ConflictTarget::Order { parent_id: parent });
        assert_eq!(c.branches.len(), 2);
        assert_eq!(c.base_value, Some(serde_json::to_value(&base).unwrap()));
        assert_eq!(c.auto_resolved, BranchSide::Ours);
        assert_eq!(m, ours);
    }
}
