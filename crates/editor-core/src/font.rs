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
    let family = match node.effective_modifier(ModifierType::FontFamily) {
        Some(Modifier::FontFamily { value }) => value.clone(),
        _ => String::new(),
    };
    let weight = match node.effective_modifier(ModifierType::FontWeight) {
        Some(Modifier::FontWeight { value }) => *value,
        _ => 400,
    };

    (family, weight)
}

fn collect_for_node(node: &NodeRef<'_>, font_registry: &FontRegistry, output: &mut FontRequests) {
    let (family, weight) = resolve_font_for_node(node);

    if let Node::Text(text_node) = node.node() {
        let mut codepoints: HashSet<u32> = text_node
            .text
            .to_string()
            .chars()
            .map(|c| c as u32)
            .collect();
        // Ruby annotations are shaped with the base node's (family, weight)
        // (see editor_view::measure::text::ruby), so their codepoints belong to
        // the same request key. Without this, ruby glyphs never trigger a font
        // load and the view never re-invalidates when the chunk arrives.
        for m in node.modifiers() {
            if let Modifier::Ruby { text } = m {
                codepoints.extend(text.chars().map(|c| c as u32));
            }
        }
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
            DocOp::MoveText { to_node_id, .. } => {
                affected_text_nodes.insert(*to_node_id);
            }
            DocOp::Modifier {
                node_id,
                op: OrMapOp::Set { key, .. },
            } => {
                if matches!(key, ModifierType::FontFamily | ModifierType::FontWeight) {
                    affected_subtrees.insert(*node_id);
                } else if matches!(key, ModifierType::Ruby) {
                    // Ruby modifier only changes the ruby text on this single node;
                    // base font and descendant scope are unaffected.
                    affected_text_nodes.insert(*node_id);
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
            DocOp::Style { .. } | DocOp::NodeStyle { .. } => {
                return collect_font_requests(doc, font_registry);
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
                StateField::TableOverlays,
                StateField::Placeholder,
            ],
        });
        editor.push_event(EditorEvent::RenderInvalidated);
    }
}

#[cfg(test)]
mod base_style_tests {
    use super::*;
    use editor_macros::doc;
    use editor_model::Doc;
    use editor_resource::FontRegistry;

    fn doc_with_base_font() -> (Doc, NodeId) {
        let (doc, _p, t1) = doc! {
            styles { base: "기본" [font_family("Pretendard".to_string()), font_weight(400)] }
            root @base [] { p: paragraph { t1: text("Hi") } }
        };
        (doc, t1)
    }

    #[test]
    fn resolve_font_uses_base_style_without_panic() {
        let (doc, t1) = doc_with_base_font();
        let text = doc.node(t1).unwrap();
        let (family, weight) = resolve_font_for_node(&text);
        assert_eq!(family, "Pretendard");
        assert_eq!(weight, 400);
    }

    #[test]
    fn resolve_font_uses_effective_modifier() {
        let (doc, _p, t1) = doc! {
            styles { base: "기본" [font_family("Pretendard".to_string()), font_weight(400)] }
            root @base [] { p: paragraph { t1: text("Hi") } }
        };
        let (family, weight) = resolve_font_for_node(&doc.node(t1).unwrap());
        assert_eq!(family, "Pretendard");
        assert_eq!(weight, 400);
    }

    #[test]
    fn font_updates_triggered_by_style_op() {
        use editor_crdt::{Dot, Op, OrSetOp};
        use editor_model::{Modifier, StyleOp};

        let (doc, _t1) = doc_with_base_font();
        let registry = FontRegistry::new();

        let ops = vec![Op {
            id: Dot::new(1, 0),
            parents: vec![],
            payload: DocOp::Style {
                style_id: "base".into(),
                op: StyleOp::Modifiers(OrSetOp::Add {
                    elem: Modifier::FontFamily {
                        value: "Pretendard".into(),
                    },
                }),
            },
        }];

        let requests = derive_font_updates_from_ops(&doc, &registry, &ops);
        assert!(
            !requests.is_empty(),
            "a base-style font change must derive font requests"
        );
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
    fn collect_includes_ruby_text_codepoints() {
        let (doc, t1) = doc! {
            root [font_family("Pretendard".to_string()), font_weight(400)] {
                paragraph {
                    t1: text("AB") [ruby(text: "한자".to_string())]
                }
            }
        };

        let result = collect_font_requests(&doc, &editor_resource::FontRegistry::new());

        let key = ("Pretendard".to_string(), 400u16);
        let cps = &result[&key][&t1];
        assert!(cps.contains(&('A' as u32)));
        assert!(cps.contains(&('B' as u32)));
        assert!(cps.contains(&('한' as u32)));
        assert!(cps.contains(&('자' as u32)));
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

    #[test]
    fn derive_font_updates_includes_move_text_target_scope() {
        use editor_macros::state;
        use editor_model::DocOp;

        let (state, t1, t2) = state! {
            doc {
                root [font_family("Source".to_string()), font_weight(400)] {
                    paragraph {
                        t1: text("가")
                    }
                    paragraph {
                        t2: text("") [font_family("Target".to_string()), font_weight(700)]
                    }
                }
            }
            selection: (t1, 0)
        };
        let entry = state
            .doc
            .text_view(t1)
            .unwrap()
            .visible_entries()
            .next()
            .unwrap()
            .0;
        let (state, move_op) = state
            .apply(DocOp::MoveText {
                entry,
                to_node_id: t2,
                after: None,
            })
            .unwrap();

        let result = derive_font_updates_from_ops(
            &state.doc,
            &editor_resource::FontRegistry::new(),
            &[move_op],
        );

        let key = ("Target".to_string(), 700u16);
        assert!(
            result.contains_key(&key),
            "MoveText target scope should request target font; keys = {:?}",
            result.keys().collect::<Vec<_>>()
        );
        assert!(result[&key].contains_key(&t2));
        assert!(result[&key][&t2].contains(&('가' as u32)));
    }
}
