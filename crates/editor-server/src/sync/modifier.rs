use std::collections::{HashMap, HashSet};

use editor_model::{Modifier, ModifierType, NodeId};

use crate::sync::conflict::{
    AttributeScope, BranchSide, ConflictBranch, ConflictKind, ConflictRecord, ConflictTarget,
};

// New variants default to parameter-form for safety.
fn has_parameter(m: &Modifier) -> bool {
    !matches!(
        m,
        Modifier::Bold | Modifier::Italic | Modifier::Underline | Modifier::Strikethrough
    )
}

pub fn merge_modifiers(
    node_id: NodeId,
    base: &[Modifier],
    ours: &[Modifier],
    theirs: &[Modifier],
) -> (Vec<Modifier>, Vec<ConflictRecord>) {
    fn into_map(list: &[Modifier]) -> HashMap<ModifierType, Modifier> {
        list.iter().map(|m| (m.as_type(), m.clone())).collect()
    }

    let base_map = into_map(base);
    let ours_map = into_map(ours);
    let theirs_map = into_map(theirs);

    let mut merged: HashMap<ModifierType, Modifier> = HashMap::new();
    let mut conflicts = Vec::new();

    let all_types: HashSet<ModifierType> = base_map
        .keys()
        .chain(ours_map.keys())
        .chain(theirs_map.keys())
        .copied()
        .collect();

    for ty in all_types {
        let in_base = base_map.get(&ty);
        let in_ours = ours_map.get(&ty);
        let in_theirs = theirs_map.get(&ty);

        match (in_ours, in_theirs) {
            (Some(o), Some(t)) if o == t => {
                merged.insert(ty, o.clone());
            }
            (Some(o), Some(t)) => {
                if !has_parameter(o) {
                    merged.insert(ty, o.clone());
                } else if in_base.is_some_and(|b| b == o) {
                    merged.insert(ty, t.clone());
                } else if in_base.is_some_and(|b| b == t) {
                    merged.insert(ty, o.clone());
                } else {
                    let target = ConflictTarget::Attribute {
                        scope: AttributeScope::Node { node_id },
                        name: format!("modifier:{}", <&'static str>::from(ty)),
                    };
                    conflicts.push(ConflictRecord {
                        kind: ConflictKind::Attribute,
                        target,
                        base_value: in_base.map(|m| serde_json::to_value(m).unwrap().into()),
                        branches: vec![
                            ConflictBranch {
                                side: BranchSide::Ours,
                                value: serde_json::to_value(o).unwrap().into(),
                            },
                            ConflictBranch {
                                side: BranchSide::Theirs,
                                value: serde_json::to_value(t).unwrap().into(),
                            },
                        ],
                        auto_resolved: BranchSide::Ours,
                    });
                    merged.insert(ty, o.clone());
                }
            }
            (Some(o), None) => {
                if !in_base.is_some_and(|b| b == o) {
                    merged.insert(ty, o.clone());
                }
            }
            (None, Some(t)) => {
                if !in_base.is_some_and(|b| b == t) {
                    merged.insert(ty, t.clone());
                }
            }
            (None, None) => {}
        }
    }

    let mut merged_list: Vec<Modifier> = merged.into_values().collect();
    merged_list.sort_by_key(|m| m.as_type());
    (merged_list, conflicts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_model::{Modifier, NodeId};

    #[test]
    fn both_add_different_set_modifiers_merges_union() {
        let (m, c) = merge_modifiers(NodeId::new(), &[], &[Modifier::Bold], &[Modifier::Italic]);
        assert!(m.contains(&Modifier::Bold));
        assert!(m.contains(&Modifier::Italic));
        assert!(c.is_empty());
    }

    #[test]
    fn both_change_link_href_creates_conflict() {
        let base = vec![Modifier::Link { href: "a".into() }];
        let ours = vec![Modifier::Link { href: "b".into() }];
        let theirs = vec![Modifier::Link { href: "c".into() }];
        let (_m, c) = merge_modifiers(NodeId::new(), &base, &ours, &theirs);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].kind, ConflictKind::Attribute);
    }

    #[test]
    fn one_side_removes_other_unchanged_removes() {
        let base = vec![Modifier::Bold];
        let ours: Vec<Modifier> = vec![];
        let theirs = vec![Modifier::Bold];
        let (m, c) = merge_modifiers(NodeId::new(), &base, &ours, &theirs);
        assert!(!m.contains(&Modifier::Bold));
        assert!(c.is_empty());
    }

    #[test]
    fn ours_unchanged_from_base_takes_theirs_no_conflict() {
        let base = vec![Modifier::TextColor { value: "A".into() }];
        let ours = base.clone();
        let theirs = vec![Modifier::TextColor { value: "B".into() }];
        let (m, c) = merge_modifiers(NodeId::new(), &base, &ours, &theirs);
        assert!(c.is_empty());
        assert!(m.contains(&Modifier::TextColor { value: "B".into() }));
    }

    #[test]
    fn theirs_unchanged_from_base_takes_ours_no_conflict() {
        let base = vec![Modifier::TextColor { value: "A".into() }];
        let ours = vec![Modifier::TextColor { value: "B".into() }];
        let theirs = base.clone();
        let (m, c) = merge_modifiers(NodeId::new(), &base, &ours, &theirs);
        assert!(c.is_empty());
        assert!(m.contains(&Modifier::TextColor { value: "B".into() }));
    }
}
