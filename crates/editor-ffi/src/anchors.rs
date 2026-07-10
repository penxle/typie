use editor_model::{ChildView, DocView, PlainDoc};
use editor_state::{Position, Selection, StableSelection, State};

#[derive(Debug, thiserror::Error)]
pub enum AnchorError {
    #[error("failed to load plain document: {0}")]
    Load(String),
    #[error("failed to encode changesets: {0}")]
    Encode(String),
    #[error("invalid anchor path: {0:?}")]
    InvalidPath(Vec<u32>),
}

pub fn graph_with_anchors(
    plain: &PlainDoc,
    paths: &[Vec<u32>],
) -> Result<(Vec<u8>, Vec<StableSelection>), AnchorError> {
    let state = State::from_plain(plain).map_err(|e| AnchorError::Load(format!("{e:?}")))?;

    let changesets = state.graph().changesets_as_vec();
    let graph = if changesets.is_empty() {
        Vec::new()
    } else {
        editor_codec::encode_changesets(editor_codec::ReencodableChangesets::from_local_ops(
            changesets,
        ))
        .map_err(|e| AnchorError::Encode(e.to_string()))?
    };

    let view = state.view();
    let anchors = paths
        .iter()
        .map(|path| capture_block_anchor(&view, path))
        .collect::<Result<Vec<_>, _>>()?;

    Ok((graph, anchors))
}

fn capture_block_anchor(view: &DocView<'_>, path: &[u32]) -> Result<StableSelection, AnchorError> {
    let err = || AnchorError::InvalidPath(path.to_vec());
    let mut node = view.root().ok_or_else(err)?;

    for &idx in path.iter().take(path.len().saturating_sub(1)) {
        node = match node.child_at(idx as usize) {
            Some(ChildView::Block(b)) => b,
            _ => return Err(err()),
        };
    }

    let (host_dot, from, to) = match path.last() {
        None => {
            let dot = node.dot().ok_or_else(err)?;
            (dot, 0, node.child_count())
        }
        Some(&last) => {
            let last = last as usize;
            if node.child_at(last).is_none() {
                return Err(err());
            }
            let dot = node.dot().ok_or_else(err)?;
            (dot, last, last + 1)
        }
    };

    let sel = Selection::new(Position::new(host_dot, from), Position::new(host_dot, to));
    match sel.resolve(view) {
        Some(r) if !r.is_collapsed() => Ok(StableSelection::capture(&sel, view)),
        _ => Err(err()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use editor_model::{
        PlainDoc, PlainNode, PlainNodeEntry, PlainParagraphNode, PlainRootNode, PlainTextNode,
    };

    use super::*;

    fn entry(node: PlainNode, children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
        PlainNodeEntry {
            node,
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children,
        }
    }

    fn para(text: &str) -> PlainNodeEntry {
        entry(
            PlainNode::Paragraph(PlainParagraphNode {}),
            vec![entry(
                PlainNode::Text(PlainTextNode { text: text.into() }),
                Vec::new(),
            )],
        )
    }

    fn doc(children: Vec<PlainNodeEntry>) -> PlainDoc {
        PlainDoc {
            root: PlainNodeEntry {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children,
            },
        }
    }

    #[test]
    fn returns_one_anchor_per_path() {
        let plain = doc(vec![para("hello"), para("world")]);
        let (graph, anchors) = graph_with_anchors(&plain, &[vec![0], vec![1]]).unwrap();
        assert!(!graph.is_empty());
        assert_eq!(anchors.len(), 2);
        assert_ne!(anchors[0], anchors[1]);
    }

    #[test]
    fn empty_path_anchors_root() {
        let plain = doc(vec![para("hello")]);
        let (_, anchors) = graph_with_anchors(&plain, &[Vec::new()]).unwrap();
        assert_eq!(anchors.len(), 1);
    }

    #[test]
    fn invalid_path_is_error() {
        let plain = doc(vec![para("hello")]);
        assert!(matches!(
            graph_with_anchors(&plain, &[vec![5]]),
            Err(AnchorError::InvalidPath(_))
        ));
    }

    #[test]
    fn nested_structural_paths_anchor() {
        use editor_model::{PlainTableCellNode, PlainTableNode, PlainTableRowNode};

        let table = entry(
            PlainNode::Table(PlainTableNode {
                border_style: Default::default(),
                proportion: 100,
            }),
            vec![entry(
                PlainNode::TableRow(PlainTableRowNode {}),
                vec![entry(
                    PlainNode::TableCell(PlainTableCellNode {
                        col_width: None,
                        background_color: None,
                    }),
                    vec![para("cell")],
                )],
            )],
        );
        let plain = doc(vec![table, para("tail")]);

        let paths = [vec![0], vec![0, 0], vec![0, 0, 0], vec![0, 0, 0, 0]];
        let (_, anchors) = graph_with_anchors(&plain, &paths).unwrap();
        assert_eq!(anchors.len(), 4);
        for pair in anchors.windows(2) {
            assert_ne!(pair[0], pair[1]);
        }
    }

    #[test]
    fn anchor_resolves_after_graph_roundtrip() {
        let plain = doc(vec![para("hello"), para("world")]);
        let (graph, anchors) = graph_with_anchors(&plain, &[vec![1]]).unwrap();

        let cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&graph[..])
                .unwrap()
                .into_graph_input();
        let state = crate::graph::build_state_tolerant(cs).unwrap();
        let view = state.view();
        let ctx = editor_state::StableResolveCtx::from_live(&view, state.projected.seq_checkout());
        let sel = anchors[0].resolve(&ctx).unwrap();
        let resolved = sel.resolve(&view).unwrap();
        assert!(!resolved.is_collapsed());
    }
}
