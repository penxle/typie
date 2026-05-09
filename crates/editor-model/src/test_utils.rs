use crate::{Doc, Modifier, NodeEntry, NodeId, PlainNode};

fn project_modifiers(entry: &NodeEntry) -> Vec<Modifier> {
    let mut mods: Vec<Modifier> = entry.modifiers.iter().map(|(_, v)| v.clone()).collect();
    mods.sort_by_key(|m| m.as_type());
    mods
}

fn project_children(entry: &NodeEntry) -> Vec<NodeId> {
    entry.children.iter().copied().collect()
}

pub fn default_modifiers() -> Vec<Modifier> {
    vec![
        Modifier::FontFamily {
            value: "Pretendard".to_string(),
        },
        Modifier::FontSize { value: 1200 },
        Modifier::FontWeight { value: 400 },
        Modifier::TextColor {
            value: "black".to_string(),
        },
        Modifier::BackgroundColor {
            value: "none".to_string(),
        },
        Modifier::LetterSpacing { value: 0 },
        Modifier::LineHeight { value: 160 },
        Modifier::ParagraphIndent { value: 100 },
        Modifier::BlockGap { value: 100 },
    ]
}

pub fn default_modifiers_with(overrides: Vec<Modifier>) -> Vec<Modifier> {
    let override_types: Vec<_> = overrides.iter().map(|m| m.as_type()).collect();
    let mut mods: Vec<_> = default_modifiers()
        .into_iter()
        .filter(|m| !override_types.contains(&m.as_type()))
        .collect();
    mods.extend(overrides);
    mods
}

fn collect_entries_dfs(
    doc: &Doc,
    node_id: NodeId,
    nodes: &mut Vec<PlainNode>,
    modifiers: &mut Vec<Vec<Modifier>>,
) {
    if let Some(entry) = doc.get_entry(node_id) {
        nodes.push(entry.node.to_plain());
        modifiers.push(project_modifiers(entry));
        for child_id in project_children(entry) {
            collect_entries_dfs(doc, child_id, nodes, modifiers);
        }
    }
}

pub fn assert_doc_eq_impl(actual: &Doc, expected: &Doc) {
    let mut nodes1 = Vec::new();
    let mut mods1 = Vec::new();
    let mut nodes2 = Vec::new();
    let mut mods2 = Vec::new();
    collect_entries_dfs(actual, NodeId::ROOT, &mut nodes1, &mut mods1);
    collect_entries_dfs(expected, NodeId::ROOT, &mut nodes2, &mut mods2);

    assert_eq!(
        nodes1.len(),
        nodes2.len(),
        "Documents have different number of nodes: {} vs {}",
        nodes1.len(),
        nodes2.len(),
    );

    for (i, (n1, n2)) in nodes1.iter().zip(nodes2.iter()).enumerate() {
        assert_eq!(
            n1, n2,
            "Node at index {} differs:\nActual: {:?}\nExpected: {:?}",
            i, n1, n2,
        );
    }

    for (i, (m1, m2)) in mods1.iter().zip(mods2.iter()).enumerate() {
        assert_eq!(
            m1, m2,
            "Modifiers at node index {} differ:\nActual: {:?}\nExpected: {:?}",
            i, m1, m2,
        );
    }
}

#[macro_export]
macro_rules! assert_doc_eq {
    ($actual:expr, $expected:expr) => {
        $crate::assert_doc_eq_impl(&$actual, &$expected)
    };
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    #[test]
    fn assert_doc_eq_identical_docs() {
        let (doc1, ..) = doc! {
            root { paragraph { text("Hello") } }
        };
        let (doc2, ..) = doc! {
            root { paragraph { text("Hello") } }
        };
        crate::assert_doc_eq!(doc1, doc2);
    }

    #[test]
    #[should_panic(expected = "Node at index")]
    fn assert_doc_eq_different_docs() {
        let (doc1, ..) = doc! {
            root { paragraph { text("Hello") } }
        };
        let (doc2, ..) = doc! {
            root { paragraph { text("World") } }
        };
        crate::assert_doc_eq!(doc1, doc2);
    }
}
