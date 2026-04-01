use crate::{Doc, Modifier, Node, NodeId};

pub fn default_modifiers() -> Vec<Modifier> {
    vec![
        Modifier::FontFamily("Pretendard".to_string()),
        Modifier::FontSize(1200),
        Modifier::FontWeight(400),
        Modifier::TextColor("black".to_string()),
        Modifier::BackgroundColor("none".to_string()),
        Modifier::LetterSpacing(0),
        Modifier::LineHeight(160),
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
    nodes: &mut Vec<Node>,
    modifiers: &mut Vec<Vec<Modifier>>,
) {
    if let Some(entry) = doc.get_entry(node_id) {
        nodes.push(entry.node.clone());
        let mut mods = entry.modifiers.clone();
        mods.sort_by_key(|m| m.as_type());
        modifiers.push(mods);
        for &child_id in entry.children.iter() {
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

    use super::*;

    #[test]
    fn default_modifiers_contains_expected() {
        let mods = default_modifiers();
        assert_eq!(mods.len(), 7);
        assert!(
            mods.iter()
                .any(|m| matches!(m, Modifier::FontFamily(f) if f == "Pretendard"))
        );
        assert!(mods.iter().any(|m| matches!(m, Modifier::FontSize(1200))));
    }

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
