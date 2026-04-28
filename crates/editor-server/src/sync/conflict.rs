use editor_model::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    Attribute,
    Text,
    Lifecycle,
    Position,
    Order,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConflictTarget {
    Attribute {
        scope: AttributeScope,
        name: String,
    },
    Text {
        node_id: NodeId,
        range_start: usize,
        range_end: usize,
    },
    Lifecycle {
        node_id: NodeId,
        parent_id: NodeId,
    },
    Position {
        node_id: NodeId,
    },
    Order {
        parent_id: NodeId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum AttributeScope {
    Node { node_id: NodeId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchSide {
    Ours,
    Theirs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictBranch {
    pub side: BranchSide,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictRecord {
    pub kind: ConflictKind,
    pub target: ConflictTarget,
    pub base_value: Option<serde_json::Value>,
    pub branches: Vec<ConflictBranch>,
    /// Merge sets a default; caller overrides via LWW (committedAt) since merge has no clock.
    pub auto_resolved: BranchSide,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn conflict_record_serializes_with_target_kind() {
        let record = ConflictRecord {
            kind: ConflictKind::Attribute,
            target: ConflictTarget::Attribute {
                scope: AttributeScope::Node {
                    node_id: NodeId::new(),
                },
                name: "page_layout".into(),
            },
            base_value: Some(json!("portrait")),
            branches: vec![
                ConflictBranch {
                    side: BranchSide::Ours,
                    value: json!("landscape"),
                },
                ConflictBranch {
                    side: BranchSide::Theirs,
                    value: json!("portrait_wide"),
                },
            ],
            auto_resolved: BranchSide::Theirs,
        };
        let s = serde_json::to_string(&record).unwrap();
        assert!(s.contains("\"kind\":\"attribute\""));
        assert!(s.contains("\"side\":\"ours\""));

        let back: ConflictRecord = serde_json::from_str(&s).unwrap();
        assert_eq!(back, record);
    }

    #[test]
    fn text_conflict_roundtrips() {
        let node_id = NodeId::new();
        let r = ConflictRecord {
            kind: ConflictKind::Text,
            target: ConflictTarget::Text {
                node_id,
                range_start: 3,
                range_end: 7,
            },
            base_value: Some(json!("hello world")),
            branches: vec![
                ConflictBranch {
                    side: BranchSide::Ours,
                    value: json!("hello rust"),
                },
                ConflictBranch {
                    side: BranchSide::Theirs,
                    value: json!("hello swift"),
                },
            ],
            auto_resolved: BranchSide::Ours,
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: ConflictRecord = serde_json::from_str(&s).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn lifecycle_conflict_roundtrips() {
        let node_id = NodeId::new();
        let parent_id = NodeId::new();
        let r = ConflictRecord {
            kind: ConflictKind::Lifecycle,
            target: ConflictTarget::Lifecycle { node_id, parent_id },
            base_value: None,
            branches: vec![
                ConflictBranch {
                    side: BranchSide::Ours,
                    value: json!(null),
                },
                ConflictBranch {
                    side: BranchSide::Theirs,
                    value: json!("deleted"),
                },
            ],
            auto_resolved: BranchSide::Theirs,
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: ConflictRecord = serde_json::from_str(&s).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn position_conflict_roundtrips() {
        let node_id = NodeId::new();
        let r = ConflictRecord {
            kind: ConflictKind::Position,
            target: ConflictTarget::Position { node_id },
            base_value: Some(json!({"parent": "abc"})),
            branches: vec![
                ConflictBranch {
                    side: BranchSide::Ours,
                    value: json!({"parent": "def"}),
                },
                ConflictBranch {
                    side: BranchSide::Theirs,
                    value: json!({"parent": "ghi"}),
                },
            ],
            auto_resolved: BranchSide::Ours,
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: ConflictRecord = serde_json::from_str(&s).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn order_conflict_roundtrips() {
        let parent_id = NodeId::new();
        let r = ConflictRecord {
            kind: ConflictKind::Order,
            target: ConflictTarget::Order { parent_id },
            base_value: Some(json!(["a", "b", "c"])),
            branches: vec![
                ConflictBranch {
                    side: BranchSide::Ours,
                    value: json!(["b", "a", "c"]),
                },
                ConflictBranch {
                    side: BranchSide::Theirs,
                    value: json!(["a", "c", "b"]),
                },
            ],
            auto_resolved: BranchSide::Theirs,
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: ConflictRecord = serde_json::from_str(&s).unwrap();
        assert_eq!(back, r);
    }
}
