use editor_model::{Doc, Modifier, Node, NodeId};
use editor_transaction::Effect;
use editor_view::Viewport;
use hashbrown::{HashMap, HashSet};

use crate::editor::Editor;
use crate::event::EditorEvent;
use crate::message::*;

pub fn handle_system_event(editor: &mut Editor, event: SystemEvent) {
    match event {
        SystemEvent::Initialize => {
            editor.pending_fonts = collect_font_requests(&editor.state.doc);

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
            });

            editor.view.layout(&editor.state.doc);
        }

        SystemEvent::Resize {
            width,
            height,
            scale_factor,
        } => {
            editor
                .view
                .resize(Viewport::new(width, height, scale_factor));
        }

        SystemEvent::SetFocused(_focused) => {
            // stub
        }

        SystemEvent::FontsLoaded {
            family,
            weight,
            mappings,
        } => {
            {
                let mut resource = editor.resource.lock().unwrap();
                let requested_id = resource.font_registry.intern(&family);
                for mapping in &mappings {
                    let resolved_id = resource.font_registry.intern(&mapping.family);
                    for &cp in &mapping.codepoints {
                        resource.font_registry.add_codepoint_mapping(
                            requested_id,
                            weight,
                            cp,
                            resolved_id,
                            mapping.weight,
                        );
                    }
                }
            }

            let loaded_cps: HashSet<u32> = mappings
                .iter()
                .flat_map(|m| m.codepoints.iter().copied())
                .collect();

            let mut affected_nodes = Vec::new();
            let key = (family, weight);
            if let Some(nodes) = editor.pending_fonts.get_mut(&key) {
                nodes.retain(|node_id, pending_cps| {
                    let before = pending_cps.len();
                    pending_cps.retain(|cp| !loaded_cps.contains(cp));
                    if before != pending_cps.len() {
                        affected_nodes.push(*node_id);
                    }
                    !pending_cps.is_empty()
                });
                if nodes.is_empty() {
                    editor.pending_fonts.remove(&key);
                }
            }

            if editor
                .view
                .invalidate_nodes(&editor.state.doc, &affected_nodes)
            {
                editor.push_event(EditorEvent::RenderInvalidated);
            }
        }

        SystemEvent::SetExternalHeight { node_id, height } => {
            editor.view.set_external_height(node_id, height);
        }
    }
}

pub(crate) fn collect_font_requests(
    doc: &Doc,
) -> HashMap<(String, u16), HashMap<NodeId, HashSet<u32>>> {
    let mut result: HashMap<(String, u16), HashMap<NodeId, HashSet<u32>>> = HashMap::new();

    for descendant in doc.root().descendants() {
        let Node::Text(text_node) = descendant.node() else {
            continue;
        };

        let mut family: Option<String> = None;
        let mut weight: Option<u16> = None;

        for ancestor in descendant.ancestors() {
            for m in ancestor.modifiers() {
                match m {
                    Modifier::FontFamily(f) if family.is_none() => {
                        family = Some(f.clone());
                    }
                    Modifier::FontWeight(w) if weight.is_none() => {
                        weight = Some(*w);
                    }
                    _ => {}
                }
            }

            if family.is_some() && weight.is_some() {
                break;
            }
        }

        // Root invariant: FontFamily/FontWeight always present
        let family = family.unwrap();
        let weight = weight.unwrap();

        let codepoints: HashSet<u32> = text_node.text.chars().map(|c| c as u32).collect();
        if !codepoints.is_empty() {
            result
                .entry((family, weight))
                .or_default()
                .entry(descendant.id())
                .or_default()
                .extend(codepoints);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use editor_macros::{doc, state};

    use super::*;

    #[test]
    fn collect_from_single_text_node() {
        let (doc, t1) = doc! {
            root [font_family("Arial".to_string()), font_weight(400)] {
                paragraph { t1: text("AB") }
            }
        };

        let result = collect_font_requests(&doc);

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

        let result = collect_font_requests(&doc);

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

        let result = collect_font_requests(&doc);
        let nodes = &result[&("Arial".to_string(), 400)];

        assert_eq!(nodes.len(), 2);
        assert!(nodes[&t1].contains(&('A' as u32)));
        assert!(nodes[&t2].contains(&('C' as u32)));
    }

    #[test]
    fn initialize_populates_pending_fonts() {
        let (state, t1) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("AB") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        editor.apply(Message::System(SystemEvent::Initialize));

        let key = ("TestFont".to_string(), 400u16);
        assert!(editor.pending_fonts.contains_key(&key));
        assert!(editor.pending_fonts[&key].contains_key(&t1));
    }

    #[test]
    fn initialize_emits_font_missing_event() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        let events = editor.apply(Message::System(SystemEvent::Initialize));

        let has_font_missing = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::FontMissing { family, weight }
                    if family == "TestFont" && *weight == 400
            )
        });
        assert!(has_font_missing);
    }

    #[test]
    fn fonts_loaded_removes_codepoints_from_pending() {
        let (state, t1) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("AB") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        editor.apply(Message::System(SystemEvent::Initialize));

        editor.apply(Message::System(SystemEvent::FontsLoaded {
            family: "TestFont".to_string(),
            weight: 400,
            mappings: vec![FontMapping {
                family: "TestFont".to_string(),
                weight: 400,
                codepoints: vec!['A' as u32],
            }],
        }));

        let key = ("TestFont".to_string(), 400u16);
        assert!(editor.pending_fonts.contains_key(&key));
        assert!(editor.pending_fonts[&key][&t1].contains(&('B' as u32)));
        assert!(!editor.pending_fonts[&key][&t1].contains(&('A' as u32)));
    }

    #[test]
    fn fonts_loaded_removes_node_when_all_cps_loaded() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        editor.apply(Message::System(SystemEvent::Initialize));

        editor.apply(Message::System(SystemEvent::FontsLoaded {
            family: "TestFont".to_string(),
            weight: 400,
            mappings: vec![FontMapping {
                family: "TestFont".to_string(),
                weight: 400,
                codepoints: vec!['A' as u32],
            }],
        }));

        assert!(
            !editor
                .pending_fonts
                .contains_key(&("TestFont".to_string(), 400))
        );
    }

    #[test]
    fn fonts_loaded_does_not_invalidate_unaffected_node() {
        let (state, t1, t2) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                    paragraph { t2: text("B") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        editor.apply(Message::System(SystemEvent::Initialize));

        let events = editor.apply(Message::System(SystemEvent::FontsLoaded {
            family: "TestFont".to_string(),
            weight: 400,
            mappings: vec![FontMapping {
                family: "TestFont".to_string(),
                weight: 400,
                codepoints: vec!['A' as u32],
            }],
        }));

        let key = ("TestFont".to_string(), 400u16);
        assert!(
            !editor
                .pending_fonts
                .get(&key)
                .map_or(false, |n| n.contains_key(&t1))
        );
        assert!(
            editor
                .pending_fonts
                .get(&key)
                .map_or(false, |n| n.contains_key(&t2))
        );

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
    }

    #[test]
    fn fonts_loaded_emits_render_invalidated() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        editor.apply(Message::System(SystemEvent::Initialize));

        let events = editor.apply(Message::System(SystemEvent::FontsLoaded {
            family: "TestFont".to_string(),
            weight: 400,
            mappings: vec![FontMapping {
                family: "TestFont".to_string(),
                weight: 400,
                codepoints: vec!['A' as u32],
            }],
        }));

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
    }

    #[test]
    fn fonts_loaded_no_event_for_unknown_font() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        editor.apply(Message::System(SystemEvent::Initialize));

        let events = editor.apply(Message::System(SystemEvent::FontsLoaded {
            family: "UnknownFont".to_string(),
            weight: 400,
            mappings: vec![FontMapping {
                family: "UnknownFont".to_string(),
                weight: 400,
                codepoints: vec!['A' as u32],
            }],
        }));

        assert!(
            !events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
    }
}
