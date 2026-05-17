use editor_crdt::{Op, OrMapOp, RgaOp, TextOp};
use editor_model::{Doc, DocOp, Modifier, ModifierType, Node, NodeId, NodeRef};
use editor_resource::{FontRegistry, Resolution};
use editor_transaction::Effect;
use hashbrown::{HashMap, HashSet};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::event::EditorEvent;
use crate::state_field::StateField;

pub(crate) type FontRequests = HashMap<(String, u16), HashMap<NodeId, HashSet<u32>>>;

fn resolve_font_for_node(node: &NodeRef<'_>) -> (String, u16) {
    let mut family: Option<String> = None;
    let mut weight: Option<u16> = None;

    for ancestor in node.ancestors() {
        for m in ancestor.modifiers() {
            match m {
                Modifier::FontFamily { value } if family.is_none() => {
                    family = Some(value.clone());
                }
                Modifier::FontWeight { value } if weight.is_none() => {
                    weight = Some(*value);
                }
                _ => {}
            }
        }

        if family.is_some() && weight.is_some() {
            break;
        }
    }

    // Root invariant: FontFamily/FontWeight always present
    (family.unwrap(), weight.unwrap())
}

fn collect_for_node(node: &NodeRef<'_>, font_registry: &FontRegistry, output: &mut FontRequests) {
    let (family, weight) = resolve_font_for_node(node);

    if let Node::Text(text_node) = node.node() {
        let codepoints: HashSet<u32> = text_node
            .text
            .to_string()
            .chars()
            .map(|c| c as u32)
            .collect();
        if !codepoints.is_empty() {
            output
                .entry((family, weight))
                .or_default()
                .entry(node.id())
                .or_default()
                .extend(codepoints);
        }
    } else if node.spec().is_textblock() {
        let Some(family_id) = font_registry.intern_id(&family) else {
            return;
        };
        if matches!(
            font_registry.resolve(family_id, weight, ' ' as u32),
            Resolution::Pending { .. }
        ) {
            output
                .entry((family, weight))
                .or_default()
                .entry(node.id())
                .or_default()
                .insert(' ' as u32);
        }
    }
}

pub(crate) fn collect_font_requests(doc: &Doc, font_registry: &FontRegistry) -> FontRequests {
    let mut result = FontRequests::new();
    for descendant in doc.root().unwrap().descendants() {
        collect_for_node(&descendant, font_registry, &mut result);
    }
    result
}

fn collect_subtree(
    doc: &Doc,
    node_id: NodeId,
    font_registry: &FontRegistry,
    output: &mut FontRequests,
) {
    let Some(node) = doc.node(node_id) else {
        return;
    };
    collect_for_node(&node, font_registry, output);
    for descendant in node.descendants() {
        collect_for_node(&descendant, font_registry, output);
    }
}

/// Inspects a batch of just-applied doc ops and returns the (family, weight, node_id, codepoints)
/// requests that may have become newly required as a consequence. Only the affected scopes are
/// traversed — text edits touch a single text node, font modifier changes touch one subtree —
/// so this stays O(edit-size) rather than O(doc-size) on every keystroke.
pub(crate) fn derive_font_updates_from_ops(
    doc: &Doc,
    font_registry: &FontRegistry,
    ops: &[Op<DocOp>],
) -> FontRequests {
    let mut affected_subtrees: HashSet<NodeId> = HashSet::new();
    let mut affected_text_nodes: HashSet<NodeId> = HashSet::new();

    for op in ops {
        match &op.payload {
            DocOp::Text {
                node_id,
                op: TextOp::InsertChar { .. },
            } => {
                affected_text_nodes.insert(*node_id);
            }
            DocOp::Modifier {
                node_id,
                op: OrMapOp::Set { key, .. },
            } => {
                if matches!(key, ModifierType::FontFamily | ModifierType::FontWeight) {
                    affected_subtrees.insert(*node_id);
                }
            }
            DocOp::Modifier {
                node_id,
                op: OrMapOp::Unset { .. },
            } => {
                // OrMapOp::Unset carries dot lists, not the modifier type, so the op alone
                // doesn't tell us whether a font modifier was the target. Re-collect the
                // subtree conservatively — it's at most one textblock's worth of work.
                affected_subtrees.insert(*node_id);
            }
            DocOp::Children {
                op: RgaOp::Insert { value, .. },
                ..
            } => {
                affected_subtrees.insert(*value);
            }
            _ => {}
        }
    }

    let mut output = FontRequests::new();
    for nid in &affected_text_nodes {
        if affected_subtrees.contains(nid) {
            continue;
        }
        if let Some(node) = doc.node(*nid) {
            collect_for_node(&node, font_registry, &mut output);
        }
    }
    for nid in affected_subtrees {
        collect_subtree(doc, nid, font_registry, &mut output);
    }
    output
}

pub(crate) fn reresolve_fonts(editor: &mut Editor) -> Result<(), EditorError> {
    {
        let resource = editor.resource.lock().unwrap();
        editor.pending_fonts = collect_font_requests(&editor.state.doc, &resource.font_registry);
    }

    let requests: Vec<_> = editor
        .pending_fonts
        .iter()
        .map(|((family, weight), nodes)| {
            let all_cps: Vec<u32> = nodes
                .values()
                .flatten()
                .copied()
                .collect::<HashSet<u32>>()
                .into_iter()
                .collect();
            (family.clone(), *weight, all_cps)
        })
        .collect();

    editor.transact(|tr| {
        for (family, weight, codepoints) in requests {
            tr.push_effect(Effect::LoadFont {
                family,
                weight,
                codepoints,
            });
        }
        Ok(())
    })
}

pub(crate) fn retry_pending_on_load(editor: &mut Editor, family: &str) {
    let resource = editor.resource.lock().unwrap();
    if resource.font_registry.intern_id(family).is_none() {
        return;
    }
    let mut affected_nodes = Vec::new();

    for ((req_family, req_weight), nodes) in editor.pending_fonts.iter_mut() {
        let Some(req_family_id) = resource.font_registry.intern_id(req_family) else {
            continue;
        };
        for (node_id, pending_cps) in nodes.iter_mut() {
            pending_cps.retain(|cp| {
                let is_ready = matches!(
                    resource
                        .font_registry
                        .resolve(req_family_id, *req_weight, *cp),
                    Resolution::Ready(_)
                );
                if is_ready {
                    affected_nodes.push(*node_id);
                    false
                } else {
                    true
                }
            });
        }
        nodes.retain(|_, cps| !cps.is_empty());
    }
    editor.pending_fonts.retain(|_, nodes| !nodes.is_empty());
    drop(resource);

    if editor
        .view
        .invalidate_nodes(&editor.state.doc, &affected_nodes)
    {
        // Real font metrics can introduce soft-wrap (page heights grow) and shift
        // line ascent/descent (caret coordinates change), so the host must re-query
        // both — otherwise the canvas stays sized for the pre-load layout.
        editor.push_event(EditorEvent::StateChanged {
            fields: vec![
                StateField::Cursor,
                StateField::PageSizes,
                StateField::ExternalElements,
            ],
        });
        editor.push_event(EditorEvent::RenderInvalidated);
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;

    #[test]
    fn collect_from_single_text_node() {
        let (doc, t1) = doc! {
            root [font_family("Arial".to_string()), font_weight(400)] {
                paragraph { t1: text("AB") }
            }
        };

        let result = collect_font_requests(&doc, &editor_resource::FontRegistry::new());

        let key = ("Arial".to_string(), 400u16);
        assert!(result.contains_key(&key));

        let nodes = &result[&key];
        assert!(nodes.contains_key(&t1));

        let cps = &nodes[&t1];
        assert!(cps.contains(&('A' as u32)));
        assert!(cps.contains(&('B' as u32)));
    }

    #[test]
    fn collect_inherits_font_from_ancestor() {
        let (doc, t1, t2) = doc! {
            root [font_family("Pretendard".to_string()), font_weight(400)] {
                paragraph {
                    t1: text("A")
                    t2: text("B") [font_weight(700)]
                }
            }
        };

        let result = collect_font_requests(&doc, &editor_resource::FontRegistry::new());

        assert!(result.contains_key(&("Pretendard".to_string(), 400)));
        assert!(result.contains_key(&("Pretendard".to_string(), 700)));
        assert!(result[&("Pretendard".to_string(), 400)].contains_key(&t1));
        assert!(result[&("Pretendard".to_string(), 700)].contains_key(&t2));
    }

    #[test]
    fn collect_groups_codepoints_per_node() {
        let (doc, t1, t2) = doc! {
            root [font_family("Arial".to_string()), font_weight(400)] {
                paragraph { t1: text("AB") }
                paragraph { t2: text("CD") }
            }
        };

        let result = collect_font_requests(&doc, &editor_resource::FontRegistry::new());
        let nodes = &result[&("Arial".to_string(), 400)];

        assert_eq!(nodes.len(), 2);
        assert!(nodes[&t1].contains(&('A' as u32)));
        assert!(nodes[&t2].contains(&('C' as u32)));
    }

    #[test]
    fn collect_requests_fold_title_weight_override() {
        let (doc, t1) = doc! {
            root [font_family("Pretendard".to_string()), font_weight(400)] {
                fold {
                    fold_title { t1: text("1234") }
                    fold_content { paragraph { text("c") } }
                }
                paragraph {}
            }
        };

        let result = collect_font_requests(&doc, &editor_resource::FontRegistry::new());

        // FoldTitle imposes weight 500 on its text; the render path requests that
        // weight, so font collection must request it too (else the glyphs never load).
        let key = ("Pretendard".to_string(), 500u16);
        assert!(
            result.contains_key(&key),
            "missing (Pretendard, 500); keys = {:?}",
            result.keys().collect::<Vec<_>>()
        );
        assert!(result[&key].contains_key(&t1));
    }
}
