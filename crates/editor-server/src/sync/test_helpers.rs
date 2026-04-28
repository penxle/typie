use std::collections::HashSet;

use editor_model::{Doc, NodeId};

pub(crate) fn assert_doc_consistent(doc: &Doc) {
    for (id, entry) in doc.nodes.iter() {
        if *id == NodeId::ROOT {
            assert!(
                entry.parent.is_none(),
                "root must not have parent, got {:?}",
                entry.parent
            );
        } else {
            let parent_id = entry
                .parent
                .unwrap_or_else(|| panic!("non-root node {:?} has no parent", id));
            let parent = doc.nodes.get(&parent_id).unwrap_or_else(|| {
                panic!(
                    "node {:?} claims parent {:?} which is not in nodes",
                    id, parent_id
                )
            });
            assert!(
                parent.children.iter().any(|c| c == id),
                "node {:?} claims parent {:?} but {:?}.children does not contain {:?} (children: {:?})",
                id,
                parent_id,
                parent_id,
                id,
                parent.children,
            );
        }

        let mut seen: HashSet<NodeId> = HashSet::new();
        for child_id in &entry.children {
            assert!(
                seen.insert(*child_id),
                "node {:?}.children contains duplicate id {:?} (children: {:?})",
                id,
                child_id,
                entry.children,
            );
            let child = doc.nodes.get(child_id).unwrap_or_else(|| {
                panic!(
                    "node {:?} lists child {:?} which is not in nodes",
                    id, child_id
                )
            });
            assert_eq!(
                child.parent,
                Some(*id),
                "node {:?} lists child {:?} but {:?}.parent = {:?} (expected {:?})",
                id,
                child_id,
                child_id,
                child.parent,
                id,
            );
        }
    }
}
