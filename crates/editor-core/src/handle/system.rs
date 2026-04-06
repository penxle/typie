use editor_model::{Doc, Modifier, Node, NodeId};
use editor_transaction::Effect;
use editor_view::Viewport;
use hashbrown::{HashMap, HashSet};
use strum::IntoEnumIterator;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::event::EditorEvent;
use crate::message::*;
use crate::state_field::StateField;

pub fn handle_system_event(editor: &mut Editor, event: SystemEvent) -> Result<(), EditorError> {
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
            })?;

            editor.view.layout(&editor.state.doc);
            editor.push_event(EditorEvent::StateChanged {
                fields: StateField::iter().collect(),
            });
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

        SystemEvent::SetFocused { .. } => {
            // stub
        }

        SystemEvent::FontManifestLoaded { family, weight } => {
            let has_pending = editor.pending_fonts.contains_key(&(family.clone(), weight));
            if !has_pending {
                return Ok(());
            }

            let codepoints: Vec<u32> = editor
                .pending_fonts
                .get(&(family.clone(), weight))
                .map(|nodes| {
                    nodes
                        .values()
                        .flatten()
                        .copied()
                        .collect::<HashSet<u32>>()
                        .into_iter()
                        .collect()
                })
                .unwrap_or_default();

            editor.resolve_fonts(&family, weight, &codepoints);
        }

        SystemEvent::FontBaseLoaded { family, weight } => {
            let resource = editor.resource.lock().unwrap();

            let Some(family_id) = resource.font_registry.intern_id(&family) else {
                return Ok(());
            };

            let loaded = (family_id, weight);
            let mut affected_nodes = Vec::new();

            for ((family, weight), nodes) in editor.pending_fonts.iter_mut() {
                let Some(family_id) = resource.font_registry.intern_id(family) else {
                    continue;
                };

                for (node_id, pending_cps) in nodes.iter_mut() {
                    pending_cps.retain(|cp| {
                        let resolved = resource
                            .font_registry
                            .codepoint_map(family_id, *weight)
                            .and_then(|m| m.get(cp).copied());

                        if resolved == Some(loaded) {
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
                editor.push_event(EditorEvent::RenderInvalidated);
            }
        }

        SystemEvent::FontChunkLoaded { .. } => {
            editor.push_event(EditorEvent::RenderInvalidated);
        }

        SystemEvent::SetExternalHeight { node_id, height } => {
            editor.view.set_external_height(node_id, height);
        }
    }
    Ok(())
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
    use crate::event::FontData;

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
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        let key = ("TestFont".to_string(), 400u16);
        assert!(editor.pending_fonts.contains_key(&key));
        assert!(editor.pending_fonts[&key].contains_key(&t1));
    }

    #[test]
    fn initialize_emits_font_manifest_missing_event() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        let events = editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        let has_manifest_missing = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::FontManifestMissing { family, weight }
                    if family == "TestFont" && *weight == 400
            )
        });
        assert!(has_manifest_missing);
    }

    #[test]
    fn font_data_loaded_removes_codepoints_from_pending() {
        let (state, t1) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("AB") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        // Pre-register codepoint mapping for 'A' only
        {
            let mut resource = editor.resource.lock().unwrap();
            let id = resource.font_registry.intern("TestFont");
            resource
                .font_registry
                .add_codepoint_mapping(id, 400, 'A' as u32, id, 400);
        }

        editor.apply(Message::System {
            event: SystemEvent::FontBaseLoaded {
                family: "TestFont".to_string(),
                weight: 400,
            },
        });

        let key = ("TestFont".to_string(), 400u16);
        assert!(editor.pending_fonts.contains_key(&key));
        assert!(editor.pending_fonts[&key][&t1].contains(&('B' as u32)));
        assert!(!editor.pending_fonts[&key][&t1].contains(&('A' as u32)));
    }

    #[test]
    fn font_data_loaded_removes_node_when_all_cps_loaded() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        // Pre-register codepoint mapping for 'A'
        {
            let mut resource = editor.resource.lock().unwrap();
            let id = resource.font_registry.intern("TestFont");
            resource
                .font_registry
                .add_codepoint_mapping(id, 400, 'A' as u32, id, 400);
        }

        editor.apply(Message::System {
            event: SystemEvent::FontBaseLoaded {
                family: "TestFont".to_string(),
                weight: 400,
            },
        });

        assert!(
            !editor
                .pending_fonts
                .contains_key(&("TestFont".to_string(), 400))
        );
    }

    #[test]
    fn font_data_loaded_does_not_invalidate_unaffected_node() {
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
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        // Pre-register codepoint mapping for 'A' only
        {
            let mut resource = editor.resource.lock().unwrap();
            let id = resource.font_registry.intern("TestFont");
            resource
                .font_registry
                .add_codepoint_mapping(id, 400, 'A' as u32, id, 400);
        }

        let events = editor.apply(Message::System {
            event: SystemEvent::FontBaseLoaded {
                family: "TestFont".to_string(),
                weight: 400,
            },
        });

        let key = ("TestFont".to_string(), 400u16);
        assert!(
            !editor
                .pending_fonts
                .get(&key)
                .is_some_and(|n| n.contains_key(&t1))
        );
        assert!(
            editor
                .pending_fonts
                .get(&key)
                .is_some_and(|n| n.contains_key(&t2))
        );

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
    }

    #[test]
    fn font_data_loaded_emits_render_invalidated() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        // Pre-register codepoint mapping for 'A'
        {
            let mut resource = editor.resource.lock().unwrap();
            let id = resource.font_registry.intern("TestFont");
            resource
                .font_registry
                .add_codepoint_mapping(id, 400, 'A' as u32, id, 400);
        }

        let events = editor.apply(Message::System {
            event: SystemEvent::FontBaseLoaded {
                family: "TestFont".to_string(),
                weight: 400,
            },
        });

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
    }

    #[test]
    fn full_font_pipeline_with_manifest_and_fallback() {
        let (state, t1) = state! {
            doc {
                root [font_family("Primary".to_string()), font_weight(400)] {
                    paragraph { t1: text("AB") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);

        // Step 1: Initialize — should emit FontManifestMissing (no manifest yet)
        let events = editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        assert!(events.iter().any(|e| matches!(
            e,
            EditorEvent::FontManifestMissing { family, weight }
                if family == "Primary" && *weight == 400
        )));

        // Step 2: Load manifest into resource
        {
            let mut resource = editor.resource.lock().unwrap();
            let id = resource.font_registry.intern("Primary");

            // Build chunk_map: block 0x00 exists, 0x41 ('A') -> chunk 0, 0x42 ('B') -> 0xFF (not covered)
            let mut chunk_map = vec![0xffu8; 256];
            chunk_map[0] = 0; // block 0x00 -> L2 index 0
            let mut l2 = [0xffu8; 256];
            l2[0x41] = 0; // 'A' -> chunk 0
            chunk_map.extend_from_slice(&l2);

            let manifest = editor_resource::FontManifest::new(4, chunk_map, vec![]);
            resource.font_registry.add_manifest(id, 400, manifest);

            // Set up fallback that covers B (0x42)
            let mut fb_chunk_map = vec![0xffu8; 256];
            fb_chunk_map[0] = 0;
            let mut fb_l2 = [0xffu8; 256];
            fb_l2[0x42] = 0; // 'B' -> chunk 0
            fb_chunk_map.extend_from_slice(&fb_l2);

            let fb_manifest = editor_resource::FontManifest::new(2, fb_chunk_map, vec![]);

            resource
                .font_registry
                .set_fallback_entries(vec![editor_resource::FallbackFontEntry {
                    family_name: "Fallback".into(),
                    fonts: vec![editor_resource::FallbackFont {
                        weight: 400,
                        manifest: fb_manifest,
                    }],
                }]);

            // Register Primary 400 as available weight
            let mut families = hashbrown::HashMap::default();
            families.insert("Primary".into(), vec![400u16]);
            resource.font_registry.update(families);
        }

        // Step 3: Send FontManifestLoaded
        let events = editor.apply(Message::System {
            event: SystemEvent::FontManifestLoaded {
                family: "Primary".to_string(),
                weight: 400,
            },
        });

        // Should emit FontDataMissing for Primary and Fallback
        let primary_event = events.iter().find(|e| {
            matches!(
                e,
                EditorEvent::FontDataMissing { family, weight, .. }
                    if family == "Primary" && *weight == 400
            )
        });
        let fallback_event = events.iter().find(|e| {
            matches!(
                e,
                EditorEvent::FontDataMissing { family, weight, .. }
                    if family == "Fallback" && *weight == 400
            )
        });
        assert!(primary_event.is_some(), "should request primary font data");
        assert!(
            fallback_event.is_some(),
            "should request fallback font data"
        );

        // Verify Primary FontDataMissing: required=[Base, Chunk(0)], prefetch has remaining chunks
        if let Some(EditorEvent::FontDataMissing {
            required, prefetch, ..
        }) = primary_event
        {
            assert_eq!(required.len(), 2, "primary required: Base + Chunk(0)");
            assert!(matches!(required[0], FontData::Base));
            assert!(matches!(required[1], FontData::Chunk { index: 0 }));
            // Primary font has chunk_count=4, required uses chunk 0, so prefetch=[1,2,3]
            assert_eq!(prefetch.len(), 3, "primary prefetch: 3 remaining chunks");
        }

        // Verify Fallback FontDataMissing: required=[Base, Chunk(0)], prefetch=[]
        if let Some(EditorEvent::FontDataMissing {
            required, prefetch, ..
        }) = fallback_event
        {
            assert_eq!(required.len(), 2, "fallback required: Base + Chunk(0)");
            assert!(matches!(required[0], FontData::Base));
            assert!(matches!(required[1], FontData::Chunk { index: 0 }));
            assert!(prefetch.is_empty(), "fallback should have no prefetch");
        }

        // Step 4: FontBaseLoaded for Primary — should invalidate T1
        let events = editor.apply(Message::System {
            event: SystemEvent::FontBaseLoaded {
                family: "Primary".to_string(),
                weight: 400,
            },
        });
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );

        // A is resolved, B still pending
        let key = ("Primary".to_string(), 400u16);
        assert!(editor.pending_fonts.get(&key).is_some_and(|n| {
            n.get(&t1)
                .is_some_and(|cps| cps.contains(&('B' as u32)) && !cps.contains(&('A' as u32)))
        }));

        // Step 5: FontBaseLoaded for Fallback — should resolve B
        let events = editor.apply(Message::System {
            event: SystemEvent::FontBaseLoaded {
                family: "Fallback".to_string(),
                weight: 400,
            },
        });
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
        assert!(!editor.pending_fonts.contains_key(&key));
    }

    #[test]
    fn font_data_loaded_no_event_for_unknown_font() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        let events = editor.apply(Message::System {
            event: SystemEvent::FontBaseLoaded {
                family: "UnknownFont".to_string(),
                weight: 400,
            },
        });

        assert!(
            !events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
    }
}
