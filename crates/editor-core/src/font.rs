use editor_crdt::{Dot, Op};
use editor_model::{ChildView, DocView, EditOp, Modifier, ModifierType, NodeView};
use editor_resource::{FontRegistry, Resolution};
use editor_transaction::Effect;
use hashbrown::{HashMap, HashSet};
use std::collections::BTreeMap;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::event::EditorEvent;
use crate::state_field::StateField;

pub(crate) type FontRequests = HashMap<(String, u16), HashMap<Dot, HashSet<u32>>>;

fn font_from_effective(eff: &BTreeMap<ModifierType, Modifier>) -> (String, u16) {
    let family = match eff.get(&ModifierType::FontFamily) {
        Some(Modifier::FontFamily { value }) => value.clone(),
        _ => String::new(),
    };
    let weight = match eff.get(&ModifierType::FontWeight) {
        Some(Modifier::FontWeight { value }) => *value,
        _ => 400,
    };
    (family, weight)
}

fn collect_for_block(block: &NodeView, font_registry: &FontRegistry, output: &mut FontRequests) {
    let block_id = block.id();
    let mut has_char = false;
    for child in block.children() {
        let ChildView::Leaf(leaf) = child else {
            continue;
        };
        let Some(ch) = leaf.as_char() else {
            continue;
        };
        has_char = true;
        let eff = leaf.effective();
        let (family, weight) = font_from_effective(eff);
        let mut codepoints: HashSet<u32> = HashSet::new();
        codepoints.insert(ch as u32);
        // Ruby annotations are shaped with the base leaf's (family, weight), so
        // their codepoints belong to the same request key.
        if let Some(Modifier::Ruby { text }) = eff.get(&ModifierType::Ruby) {
            codepoints.extend(text.chars().map(|c| c as u32));
        }
        output
            .entry((family, weight))
            .or_default()
            .entry(block_id)
            .or_default()
            .extend(codepoints);
    }
    if !has_char && block.spec().is_textblock() {
        let (family, weight) = font_from_effective(block.effective());
        if let Some(family_id) = font_registry.intern_id(&family)
            && matches!(
                font_registry.resolve(family_id, weight, ' ' as u32),
                Resolution::Pending { .. }
            )
        {
            output
                .entry((family, weight))
                .or_default()
                .entry(block_id)
                .or_default()
                .insert(' ' as u32);
        }
    }
}

pub(crate) fn collect_block_recursive(
    block: &NodeView,
    font_registry: &FontRegistry,
    output: &mut FontRequests,
) {
    collect_for_block(block, font_registry, output);
    for child in block.child_blocks() {
        collect_block_recursive(&child, font_registry, output);
    }
}

pub(crate) fn collect_subtree_block_dots(block: &NodeView, output: &mut Vec<Dot>) {
    output.push(block.id());
    for child in block.child_blocks() {
        collect_subtree_block_dots(&child, output);
    }
}

pub(crate) fn collect_font_requests(view: &DocView, font_registry: &FontRegistry) -> FontRequests {
    let mut result = FontRequests::new();
    if let Some(root) = view.root() {
        collect_block_recursive(&root, font_registry, &mut result);
    }
    result
}

// eg-walker ops reference seq positions/dots rather than block ids, so deriving
// the affected blocks incrementally isn't a cheap lookup. The View already does a
// full re-measure per edit, so a full rescan here is consistent.
pub(crate) fn derive_font_updates_from_ops(
    view: &DocView,
    font_registry: &FontRegistry,
    _ops: &[Op<EditOp>],
) -> FontRequests {
    collect_font_requests(view, font_registry)
}

pub(crate) fn reresolve_fonts(editor: &mut Editor) -> Result<(), EditorError> {
    {
        let resource = editor.resource.lock().unwrap();
        let view = editor.state.view();
        editor.pending_fonts = collect_font_requests(&view, &resource.font_registry);
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

fn invalidate_font_affected(editor: &mut Editor, affected_nodes: &[Dot]) {
    if affected_nodes.is_empty() {
        return;
    }
    let mut any = false;
    {
        let view = editor.state.view();
        for node_id in affected_nodes {
            if let Some(nv) = view.node(*node_id) {
                any |= editor.view.invalidate_measure_with_ancestors(&nv);
            }
        }
    }
    if any {
        editor.view.invalidate(&editor.state);
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
        editor.invalidate_render();
    }
}

pub(crate) fn retry_pending_on_load(editor: &mut Editor, family: &str, base_loaded: bool) {
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
            let mut node_affected = false;
            pending_cps.retain(|cp| {
                match resource
                    .font_registry
                    .resolve(req_family_id, *req_weight, *cp)
                {
                    Resolution::Ready(_) => {
                        node_affected = true;
                        false
                    }
                    Resolution::Pending {
                        needs_base: false, ..
                    } if base_loaded => {
                        node_affected = true;
                        true
                    }
                    _ => true,
                }
            });
            if node_affected {
                affected_nodes.push(*node_id);
            }
        }
        nodes.retain(|_, cps| !cps.is_empty());
    }
    editor.pending_fonts.retain(|_, nodes| !nodes.is_empty());
    drop(resource);

    invalidate_font_affected(editor, &affected_nodes);
}

#[cfg(test)]
mod base_style_tests {
    use super::*;
    use editor_macros::state;
    use editor_resource::FontRegistry;

    #[test]
    fn collect_requests_base_style_font_without_panic() {
        let (state, p1) = state! {
            doc {
                styles { base: "기본" [font_family("Pretendard".to_string()), font_weight(400)] }
                root @base [] { p1: paragraph { text("Hi") } }
            }
            selection: (p1, 0)
        };
        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());
        let key = ("Pretendard".to_string(), 400u16);
        assert!(
            result.contains_key(&key),
            "base style font must be requested; keys = {:?}",
            result.keys().collect::<Vec<_>>()
        );
        assert!(result[&key].contains_key(&p1));
    }

    #[test]
    fn collect_uses_effective_weight_override() {
        let (state, p1) = state! {
            doc {
                styles { base: "기본" [font_family("Pretendard".to_string()), font_weight(400)] }
                root @base [] { p1: paragraph { text("Hi") [font_weight(700)] } }
            }
            selection: (p1, 0)
        };
        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());
        // The per-run weight override wins over the inherited base-style weight.
        let key = ("Pretendard".to_string(), 700u16);
        assert!(
            result.contains_key(&key),
            "effective weight override must be requested; keys = {:?}",
            result.keys().collect::<Vec<_>>()
        );
        assert!(result[&key].contains_key(&p1));
    }

    #[test]
    fn derive_font_updates_rescans_doc() {
        let (state, _p1) = state! {
            doc {
                styles { base: "기본" [font_family("Pretendard".to_string()), font_weight(400)] }
                root @base [] { p1: paragraph { text("Hi") } }
            }
            selection: (p1, 0)
        };
        let view = state.view();
        // derive is a full rescan in the eg-walker model (ops are not consulted), so
        // an empty op slice still yields the document's current font requests.
        let requests = derive_font_updates_from_ops(&view, &FontRegistry::new(), &[]);
        assert!(
            !requests.is_empty(),
            "a doc with a base-style font must derive font requests"
        );
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;

    #[test]
    fn collect_from_single_text_node() {
        let (state, p1) = state! {
            doc {
                root [font_family("Arial".to_string()), font_weight(400)] {
                    p1: paragraph { text("AB") }
                }
            }
            selection: (p1, 0)
        };

        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());

        let key = ("Arial".to_string(), 400u16);
        assert!(result.contains_key(&key));

        let nodes = &result[&key];
        assert!(nodes.contains_key(&p1));

        let cps = &nodes[&p1];
        assert!(cps.contains(&('A' as u32)));
        assert!(cps.contains(&('B' as u32)));
    }

    #[test]
    fn collect_inherits_font_from_ancestor() {
        let (state, p1) = state! {
            doc {
                root [font_family("Pretendard".to_string()), font_weight(400)] {
                    p1: paragraph {
                        text("A")
                        text("B") [font_weight(700)]
                    }
                }
            }
            selection: (p1, 0)
        };

        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());

        assert!(result.contains_key(&("Pretendard".to_string(), 400)));
        assert!(result.contains_key(&("Pretendard".to_string(), 700)));
        // Both text runs share the same containing paragraph.
        assert!(result[&("Pretendard".to_string(), 400)].contains_key(&p1));
        assert!(result[&("Pretendard".to_string(), 700)].contains_key(&p1));
    }

    #[test]
    fn collect_groups_codepoints_per_node() {
        let (state, p1, p2) = state! {
            doc {
                root [font_family("Arial".to_string()), font_weight(400)] {
                    p1: paragraph { text("AB") }
                    p2: paragraph { text("CD") }
                }
            }
            selection: (p1, 0)
        };

        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());
        let nodes = &result[&("Arial".to_string(), 400)];

        assert_eq!(nodes.len(), 2);
        assert!(nodes[&p1].contains(&('A' as u32)));
        assert!(nodes[&p2].contains(&('C' as u32)));
    }

    #[test]
    fn collect_includes_ruby_text_codepoints() {
        let (state, p1) = state! {
            doc {
                root [font_family("Pretendard".to_string()), font_weight(400)] {
                    p1: paragraph {
                        text("AB") [ruby(text: "한자".to_string())]
                    }
                }
            }
            selection: (p1, 0)
        };

        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());

        let key = ("Pretendard".to_string(), 400u16);
        let cps = &result[&key][&p1];
        assert!(cps.contains(&('A' as u32)));
        assert!(cps.contains(&('B' as u32)));
        assert!(cps.contains(&('한' as u32)));
        assert!(cps.contains(&('자' as u32)));
    }

    #[test]
    fn collect_requests_fold_title_weight_override() {
        let (state, ft1) = state! {
            doc {
                root [font_family("Pretendard".to_string()), font_weight(400)] {
                    fold {
                        ft1: fold_title { text("1234") }
                        fold_content { paragraph { text("c") } }
                    }
                    paragraph {}
                }
            }
            selection: (ft1, 0)
        };

        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());

        // FoldTitle imposes weight 500 on its text; the render path requests that
        // weight, so font collection must request it too (else the glyphs never load).
        let key = ("Pretendard".to_string(), 500u16);
        assert!(
            result.contains_key(&key),
            "missing (Pretendard, 500); keys = {:?}",
            result.keys().collect::<Vec<_>>()
        );
        assert!(result[&key].contains_key(&ft1));
    }

    #[test]
    fn derive_font_updates_rescans_target_font_scope() {
        let (state, _p1, p2) = state! {
            doc {
                root [font_family("Source".to_string()), font_weight(400)] {
                    p1: paragraph { text("나") }
                    p2: paragraph {
                        text("가") [font_family("Target".to_string()), font_weight(700)]
                    }
                }
            }
            selection: (p1, 0)
        };

        let view = state.view();
        // Full rescan picks up the Target-fonted block's codepoints regardless of which op triggered it.
        let result = derive_font_updates_from_ops(&view, &FontRegistry::new(), &[]);

        let key = ("Target".to_string(), 700u16);
        assert!(
            result.contains_key(&key),
            "target scope should request target font; keys = {:?}",
            result.keys().collect::<Vec<_>>()
        );
        assert!(result[&key].contains_key(&p2));
        assert!(result[&key][&p2].contains(&('가' as u32)));
    }
}
