use serde_json::Value;

use crate::sync::conflict::{
    BranchSide, ConflictBranch, ConflictKind, ConflictRecord, ConflictTarget,
};

pub fn merge_attribute(
    target: ConflictTarget,
    base: Option<&Value>,
    ours: Option<&Value>,
    theirs: Option<&Value>,
) -> (Option<Value>, Option<ConflictRecord>) {
    if ours == theirs {
        return (ours.cloned(), None);
    }
    if ours == base {
        return (theirs.cloned(), None);
    }
    if theirs == base {
        return (ours.cloned(), None);
    }

    match (ours, theirs) {
        (Some(o), Some(t)) => {
            let record = ConflictRecord {
                kind: ConflictKind::Attribute,
                target,
                base_value: base.cloned(),
                branches: vec![
                    ConflictBranch {
                        side: BranchSide::Ours,
                        value: o.clone(),
                    },
                    ConflictBranch {
                        side: BranchSide::Theirs,
                        value: t.clone(),
                    },
                ],
                auto_resolved: BranchSide::Ours,
            };
            (Some(o.clone()), Some(record))
        }
        // Spec treats asymmetric absence as "the present side wins" rather than a conflict.
        (None, Some(t)) => (Some(t.clone()), None),
        (Some(o), None) => (Some(o.clone()), None),
        (None, None) => unreachable!("ours == theirs early return covers (None, None)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::conflict::AttributeScope;
    use serde_json::json;

    fn target() -> ConflictTarget {
        ConflictTarget::Attribute {
            scope: AttributeScope::Node {
                node_id: editor_model::NodeId::new(),
            },
            name: "test".into(),
        }
    }

    #[test]
    fn all_same_returns_value_no_conflict() {
        let (m, c) = merge_attribute(
            target(),
            Some(&json!("x")),
            Some(&json!("x")),
            Some(&json!("x")),
        );
        assert_eq!(m, Some(json!("x")));
        assert!(c.is_none());
    }

    #[test]
    fn ours_only_changes() {
        let (m, c) = merge_attribute(
            target(),
            Some(&json!("base")),
            Some(&json!("ours")),
            Some(&json!("base")),
        );
        assert_eq!(m, Some(json!("ours")));
        assert!(c.is_none());
    }

    #[test]
    fn theirs_only_changes() {
        let (m, c) = merge_attribute(
            target(),
            Some(&json!("base")),
            Some(&json!("base")),
            Some(&json!("theirs")),
        );
        assert_eq!(m, Some(json!("theirs")));
        assert!(c.is_none());
    }

    #[test]
    fn both_change_to_same_value_no_conflict() {
        let (m, c) = merge_attribute(
            target(),
            Some(&json!("base")),
            Some(&json!("same")),
            Some(&json!("same")),
        );
        assert_eq!(m, Some(json!("same")));
        assert!(c.is_none());
    }

    #[test]
    fn asymmetric_none_with_changed_other_side_is_not_conflict() {
        let (m, c) = merge_attribute(target(), Some(&json!("x")), None, Some(&json!("y")));
        assert_eq!(m, Some(json!("y")));
        assert!(c.is_none());
    }

    #[test]
    fn both_change_to_different_values_creates_conflict() {
        let (m, c) = merge_attribute(
            target(),
            Some(&json!("base")),
            Some(&json!("o")),
            Some(&json!("t")),
        );
        assert_eq!(m, Some(json!("o")));
        let c = c.unwrap();
        assert_eq!(c.kind, ConflictKind::Attribute);
        assert_eq!(c.branches.len(), 2);
        assert_eq!(c.base_value, Some(json!("base")));
    }
}
