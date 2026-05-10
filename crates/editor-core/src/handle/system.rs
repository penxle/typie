use editor_view::Viewport;
use strum::IntoEnumIterator;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::event::EditorEvent;
use crate::font;
use crate::message::*;
use crate::state_field::StateField;

pub fn handle_system_event(editor: &mut Editor, event: SystemEvent) -> Result<(), EditorError> {
    match event {
        SystemEvent::Initialize => {
            font::reresolve_fonts(editor)?;
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
            let changed = editor.view.resize(
                Viewport::new(width, height, scale_factor),
                &editor.state.doc,
            );
            if changed {
                editor.push_event(EditorEvent::StateChanged {
                    fields: vec![StateField::PageSizes],
                });
                editor.push_event(EditorEvent::RenderInvalidated);
            }
        }

        SystemEvent::SetFocused { .. } => {
            // stub
        }

        SystemEvent::FontBaseLoaded { family, weight: _ } => {
            font::retry_pending_on_load(editor, &family);
        }

        SystemEvent::FontChunkLoaded {
            family,
            weight: _,
            chunk_id: _,
        } => {
            font::retry_pending_on_load(editor, &family);
        }

        SystemEvent::SetExternalHeight { node_id, height } => {
            editor.view.set_external_height(node_id, height);
        }

        SystemEvent::FontsChanged => {
            font::reresolve_fonts(editor)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::event::FontData;

    fn test_config_single_chunk(
        family: &str,
        weight: u16,
        hash: &str,
        start: u32,
        end: u32,
    ) -> Vec<editor_resource::FontFamily> {
        vec![editor_resource::FontFamily {
            name: family.into(),
            source: editor_resource::FontFamilySource::Default,
            weights: vec![editor_resource::FontWeight {
                value: weight,
                hash: hash.into(),
                chunks: vec![vec![start, end]],
            }],
        }]
    }

    fn fake_base_bytes() -> Vec<u8> {
        editor_resource::compress_zstd(include_bytes!(
            "../../../editor-resource/assets/placeholder.ttf"
        ))
    }

    fn fake_chunk_bytes() -> Vec<u8> {
        // num_entries = 0 header — no glyph patches. add_font_chunk just marks the chunk loaded.
        editor_resource::compress_zstd(&0u32.to_be_bytes())
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
    fn initialize_emits_font_data_missing_event() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        // Populate FontConfig so resolve_codepoint_mappings can find the primary.
        {
            let mut resource = editor.resource.lock().unwrap();
            resource.set_fonts(test_config_single_chunk("TestFont", 400, "h1", 0x41, 0x41));
        }

        let events = editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        let has_data_missing = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::FontDataMissing { family, weight, required, .. }
                    if family == "TestFont"
                        && *weight == 400
                        && required.len() == 2
                        && matches!(required[0], FontData::Base)
                        && matches!(required[1], FontData::Chunk { id: 0 })
            )
        });
        assert!(has_data_missing);
    }

    #[test]
    fn font_base_loaded_removes_codepoints_from_pending() {
        let (state, t1) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("AB") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        // Config covers 'A' only (chunk 0). 'B' has no chunk → stays Missing.
        {
            let mut resource = editor.resource.lock().unwrap();
            resource.set_fonts(test_config_single_chunk("TestFont", 400, "h", 0x41, 0x41));
        }
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        {
            let mut resource = editor.resource.lock().unwrap();
            resource
                .add_font_base("TestFont", 400, &fake_base_bytes())
                .unwrap();
            resource
                .add_font_chunk("TestFont", 400, 0, &fake_chunk_bytes())
                .unwrap();
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
    fn font_base_loaded_removes_node_when_all_cps_loaded() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        {
            let mut resource = editor.resource.lock().unwrap();
            resource.set_fonts(test_config_single_chunk("TestFont", 400, "h", 0x41, 0x41));
        }
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        {
            let mut resource = editor.resource.lock().unwrap();
            resource
                .add_font_base("TestFont", 400, &fake_base_bytes())
                .unwrap();
            resource
                .add_font_chunk("TestFont", 400, 0, &fake_chunk_bytes())
                .unwrap();
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
    fn font_base_loaded_does_not_invalidate_unaffected_node() {
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
        // Config covers 'A' only — 'B' stays unresolved for this family.
        {
            let mut resource = editor.resource.lock().unwrap();
            resource.set_fonts(test_config_single_chunk("TestFont", 400, "h", 0x41, 0x41));
        }
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        {
            let mut resource = editor.resource.lock().unwrap();
            resource
                .add_font_base("TestFont", 400, &fake_base_bytes())
                .unwrap();
            resource
                .add_font_chunk("TestFont", 400, 0, &fake_chunk_bytes())
                .unwrap();
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
    fn empty_paragraph_tracked_and_invalidated_on_font_load() {
        let (state, p1, p2) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    p1: paragraph
                    p2: paragraph
                }
            }
            selection: (p1, 0)
        };

        let mut editor = Editor::new_test(state);
        // Config covers ' ' (space) so strut resolves Pending before load.
        {
            let mut resource = editor.resource.lock().unwrap();
            resource.set_fonts(test_config_single_chunk("TestFont", 400, "h", 0x20, 0x20));
        }
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        let key = ("TestFont".to_string(), 400u16);
        assert!(
            editor.pending_fonts[&key].contains_key(&p1),
            "empty paragraph must be in pending_fonts before font load"
        );
        assert!(editor.pending_fonts[&key].contains_key(&p2));

        {
            let mut resource = editor.resource.lock().unwrap();
            resource
                .add_font_base("TestFont", 400, &fake_base_bytes())
                .unwrap();
            resource
                .add_font_chunk("TestFont", 400, 0, &fake_chunk_bytes())
                .unwrap();
        }

        let events = editor.apply(Message::System {
            event: SystemEvent::FontBaseLoaded {
                family: "TestFont".to_string(),
                weight: 400,
            },
        });

        assert!(
            !editor.pending_fonts.contains_key(&key),
            "all entries should clear once the space codepoint becomes Ready"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
    }

    #[test]
    fn font_base_loaded_emits_page_sizes_and_cursor_state_change() {
        // Font load triggers re-layout (soft-wrap can appear once real font metrics
        // replace the placeholder strut). The host re-queries page_sizes / cursor only
        // when StateChanged carries those fields, so omitting them leaves the canvas
        // sized for the pre-load layout while the engine is already painting at the
        // post-load (taller) layout — bottom of the document gets clipped.
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        {
            let mut resource = editor.resource.lock().unwrap();
            resource.set_fonts(test_config_single_chunk("TestFont", 400, "h", 0x41, 0x41));
        }
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        {
            let mut resource = editor.resource.lock().unwrap();
            resource
                .add_font_base("TestFont", 400, &fake_base_bytes())
                .unwrap();
            resource
                .add_font_chunk("TestFont", 400, 0, &fake_chunk_bytes())
                .unwrap();
        }

        let events = editor.apply(Message::System {
            event: SystemEvent::FontBaseLoaded {
                family: "TestFont".to_string(),
                weight: 400,
            },
        });

        let has_page_sizes = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::StateChanged { fields }
                    if fields.contains(&StateField::PageSizes)
            )
        });
        assert!(
            has_page_sizes,
            "font load that triggers re-layout must emit StateChanged with PageSizes"
        );

        let has_cursor = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::StateChanged { fields }
                    if fields.contains(&StateField::Cursor)
            )
        });
        assert!(
            has_cursor,
            "font load that changes line metrics must emit StateChanged with Cursor"
        );
    }

    #[test]
    fn font_base_loaded_emits_render_invalidated() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        {
            let mut resource = editor.resource.lock().unwrap();
            resource.set_fonts(test_config_single_chunk("TestFont", 400, "h", 0x41, 0x41));
        }
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        {
            let mut resource = editor.resource.lock().unwrap();
            resource
                .add_font_base("TestFont", 400, &fake_base_bytes())
                .unwrap();
            resource
                .add_font_chunk("TestFont", 400, 0, &fake_chunk_bytes())
                .unwrap();
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
    fn full_font_pipeline_with_chunks_and_fallback() {
        let (state, t1) = state! {
            doc {
                root [font_family("Primary".to_string()), font_weight(400)] {
                    paragraph { t1: text("AB") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);

        // Step 1: Load config via set_fonts — Primary covers 'A' (chunk 0 of 4),
        // Fallback covers 'B' (chunk 0 of 1).
        {
            let mut resource = editor.resource.lock().unwrap();
            let families = vec![
                editor_resource::FontFamily {
                    name: "Primary".into(),
                    source: editor_resource::FontFamilySource::Default,
                    weights: vec![editor_resource::FontWeight {
                        value: 400,
                        hash: "primary-400".into(),
                        chunks: vec![
                            vec![0x41, 0x41],
                            vec![0x61, 0x61],
                            vec![0x62, 0x62],
                            vec![0x63, 0x63],
                        ],
                    }],
                },
                editor_resource::FontFamily {
                    name: "Fallback".into(),
                    source: editor_resource::FontFamilySource::Fallback,
                    weights: vec![editor_resource::FontWeight {
                        value: 400,
                        hash: "fallback-400".into(),
                        chunks: vec![vec![0x42, 0x42]],
                    }],
                },
            ];
            resource.set_fonts(families);
        }

        // Step 2: Initialize — should emit FontDataMissing for Primary and Fallback.
        let events = editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

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

        // Verify Primary: required=[Base, Chunk(0)], prefetch=[Chunk(1), Chunk(2), Chunk(3)]
        if let Some(EditorEvent::FontDataMissing {
            required, prefetch, ..
        }) = primary_event
        {
            assert_eq!(required.len(), 2, "primary required: Base + Chunk(0)");
            assert!(matches!(required[0], FontData::Base));
            assert!(matches!(required[1], FontData::Chunk { id: 0 }));
            assert_eq!(prefetch.len(), 3, "primary prefetch: 3 remaining chunks");
        }

        // Verify Fallback: required=[Base, Chunk(0)], prefetch=[] (non-primary, no prefetch)
        if let Some(EditorEvent::FontDataMissing {
            required, prefetch, ..
        }) = fallback_event
        {
            assert_eq!(required.len(), 2, "fallback required: Base + Chunk(0)");
            assert!(matches!(required[0], FontData::Base));
            assert!(matches!(required[1], FontData::Chunk { id: 0 }));
            assert!(prefetch.is_empty(), "fallback should have no prefetch");
        }

        // Step 3: Load Primary base + chunk 0, then fire FontBaseLoaded — 'A' resolves, 'B' still pending.
        {
            let mut resource = editor.resource.lock().unwrap();
            resource
                .add_font_base("Primary", 400, &fake_base_bytes())
                .unwrap();
            resource
                .add_font_chunk("Primary", 400, 0, &fake_chunk_bytes())
                .unwrap();
        }
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

        let key = ("Primary".to_string(), 400u16);
        assert!(editor.pending_fonts.get(&key).is_some_and(|n| {
            n.get(&t1)
                .is_some_and(|cps| cps.contains(&('B' as u32)) && !cps.contains(&('A' as u32)))
        }));

        // Step 4: Load Fallback base + chunk 0, then fire FontBaseLoaded — 'B' resolves.
        {
            let mut resource = editor.resource.lock().unwrap();
            resource
                .add_font_base("Fallback", 400, &fake_base_bytes())
                .unwrap();
            resource
                .add_font_chunk("Fallback", 400, 0, &fake_chunk_bytes())
                .unwrap();
        }
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
    fn set_modifier_font_family_emits_font_data_missing_for_new_family() {
        let (state, ..) = state! {
            doc {
                root [font_family("Arial".to_string()), font_weight(400)] {
                    paragraph { t1: text("AB") }
                }
            }
            selection: (t1, 0) -> (t1, 2)
        };

        let mut editor = Editor::new_test(state);
        {
            let mut resource = editor.resource.lock().unwrap();
            let families = vec![
                editor_resource::FontFamily {
                    name: "Arial".into(),
                    source: editor_resource::FontFamilySource::Default,
                    weights: vec![editor_resource::FontWeight {
                        value: 400,
                        hash: "arial-400".into(),
                        chunks: vec![vec![0x41, 0x42]],
                    }],
                },
                editor_resource::FontFamily {
                    name: "Pretendard".into(),
                    source: editor_resource::FontFamilySource::Default,
                    weights: vec![editor_resource::FontWeight {
                        value: 400,
                        hash: "pretendard-400".into(),
                        chunks: vec![vec![0x41, 0x42]],
                    }],
                },
            ];
            resource.set_fonts(families);
            resource
                .add_font_base("Arial", 400, &fake_base_bytes())
                .unwrap();
            resource
                .add_font_chunk("Arial", 400, 0, &fake_chunk_bytes())
                .unwrap();
        }
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        // Switch the selection's font family to Pretendard. Pretendard has no bytes
        // loaded — host must be told to fetch them.
        let events = editor.apply(Message::Modifier {
            op: ModifierOp::Set {
                modifier: editor_model::Modifier::FontFamily {
                    value: "Pretendard".to_string(),
                },
            },
        });

        let has_pretendard_missing = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::FontDataMissing { family, weight, required, .. }
                    if family == "Pretendard"
                        && *weight == 400
                        && required.iter().any(|d| matches!(d, FontData::Base))
            )
        });
        assert!(
            has_pretendard_missing,
            "set_modifier(FontFamily) must emit FontDataMissing for the newly-required family"
        );
    }

    #[test]
    fn insert_text_emits_font_data_missing_for_new_codepoint_chunk() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 1)
        };

        let mut editor = Editor::new_test(state);
        {
            let mut resource = editor.resource.lock().unwrap();
            // chunk 0 covers 'A' (0x41), chunk 1 covers 'B' (0x42).
            let families = vec![editor_resource::FontFamily {
                name: "TestFont".into(),
                source: editor_resource::FontFamilySource::Default,
                weights: vec![editor_resource::FontWeight {
                    value: 400,
                    hash: "testfont-400".into(),
                    chunks: vec![vec![0x41, 0x41], vec![0x42, 0x42]],
                }],
            }];
            resource.set_fonts(families);
            resource
                .add_font_base("TestFont", 400, &fake_base_bytes())
                .unwrap();
            resource
                .add_font_chunk("TestFont", 400, 0, &fake_chunk_bytes())
                .unwrap();
        }
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        // 'A' is now Ready (chunk 0 loaded). Insert 'B' which lives in unloaded chunk 1.
        let events = editor.apply(Message::Insertion {
            op: InsertionOp::Text {
                text: "B".to_string(),
            },
        });

        let has_chunk_1_missing = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::FontDataMissing { family, weight, required, .. }
                    if family == "TestFont"
                        && *weight == 400
                        && required.iter().any(|d| matches!(d, FontData::Chunk { id: 1 }))
            )
        });
        assert!(
            has_chunk_1_missing,
            "insert_text must emit FontDataMissing for the new codepoint's chunk"
        );
    }

    #[test]
    fn font_base_loaded_does_not_clear_cp_without_chunk() {
        let (state, t1) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        {
            let mut resource = editor.resource.lock().unwrap();
            resource.set_fonts(test_config_single_chunk("TestFont", 400, "h", 0x41, 0x41));
        }
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        // Load base only — chunk 0 still missing.
        {
            let mut resource = editor.resource.lock().unwrap();
            resource
                .add_font_base("TestFont", 400, &fake_base_bytes())
                .unwrap();
        }

        editor.apply(Message::System {
            event: SystemEvent::FontBaseLoaded {
                family: "TestFont".to_string(),
                weight: 400,
            },
        });

        let key = ("TestFont".to_string(), 400u16);
        let still_pending = editor
            .pending_fonts
            .get(&key)
            .and_then(|n| n.get(&t1))
            .is_some_and(|cps| cps.contains(&('A' as u32)));
        assert!(
            still_pending,
            "cp 'A' must remain pending until its chunk is also loaded"
        );
    }

    #[test]
    fn font_chunk_loaded_clears_cp_and_invalidates_node() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);
        {
            let mut resource = editor.resource.lock().unwrap();
            resource.set_fonts(test_config_single_chunk("TestFont", 400, "h", 0x41, 0x41));
        }
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        // Load base first — cp still pending (chunk not yet loaded).
        {
            let mut resource = editor.resource.lock().unwrap();
            resource
                .add_font_base("TestFont", 400, &fake_base_bytes())
                .unwrap();
        }
        editor.apply(Message::System {
            event: SystemEvent::FontBaseLoaded {
                family: "TestFont".to_string(),
                weight: 400,
            },
        });

        // Then load chunk and fire FontChunkLoaded — chunk event is what should transition cp to Ready.
        {
            let mut resource = editor.resource.lock().unwrap();
            resource
                .add_font_chunk("TestFont", 400, 0, &fake_chunk_bytes())
                .unwrap();
        }
        let events = editor.apply(Message::System {
            event: SystemEvent::FontChunkLoaded {
                family: "TestFont".to_string(),
                weight: 400,
                chunk_id: 0,
            },
        });

        let key = ("TestFont".to_string(), 400u16);
        assert!(
            !editor.pending_fonts.contains_key(&key),
            "cp must clear once chunk arrives"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated)),
            "chunk load must invalidate render"
        );
    }

    #[test]
    fn font_base_loaded_no_event_for_unknown_font() {
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

    #[test]
    fn fonts_changed_emits_font_data_missing_after_late_set_fonts() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);

        let initial_events = editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        let has_missing_on_initialize = initial_events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::FontDataMissing { family, .. } if family == "TestFont"
            )
        });
        assert!(
            !has_missing_on_initialize,
            "Initialize must not emit FontDataMissing when family is absent from registry"
        );

        {
            let mut resource = editor.resource.lock().unwrap();
            resource.set_fonts(test_config_single_chunk("TestFont", 400, "h1", 0x41, 0x41));
        }

        let events = editor.apply(Message::System {
            event: SystemEvent::FontsChanged,
        });

        let has_missing = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::FontDataMissing { family, weight, required, .. }
                    if family == "TestFont"
                        && *weight == 400
                        && required.len() == 2
                        && matches!(required[0], FontData::Base)
                        && matches!(required[1], FontData::Chunk { id: 0 })
            )
        });
        assert!(
            has_missing,
            "FontsChanged after set_fonts must emit FontDataMissing for the newly registered family"
        );
    }

    #[test]
    fn fonts_changed_is_idempotent_when_ready() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };

        let mut editor = Editor::new_test(state);

        {
            let mut resource = editor.resource.lock().unwrap();
            resource.set_fonts(test_config_single_chunk("TestFont", 400, "h1", 0x41, 0x41));
            resource
                .add_font_base("TestFont", 400, &fake_base_bytes())
                .unwrap();
            resource
                .add_font_chunk("TestFont", 400, 0, &fake_chunk_bytes())
                .unwrap();
        }

        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        let events = editor.apply(Message::System {
            event: SystemEvent::FontsChanged,
        });

        let has_missing = events.iter().any(
            |e| matches!(e, EditorEvent::FontDataMissing { family, .. } if family == "TestFont"),
        );
        assert!(
            !has_missing,
            "FontsChanged must not emit FontDataMissing when all required bytes are loaded"
        );
    }

    #[test]
    fn resize_paginated_emits_no_render_invalidated() {
        let (state, _t1) = state! {
            doc {
                root (
                    layout_mode: LayoutMode::Paginated {
                        page_width: 400,
                        page_height: 600,
                        page_margin_top: 20,
                        page_margin_bottom: 20,
                        page_margin_left: 20,
                        page_margin_right: 20,
                    }
                ) [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("hello") }
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        // Establish initial layout fingerprint so the assertion-target resize is a no-op,
        // not a first-time layout computation.
        editor.apply(Message::System {
            event: SystemEvent::Resize {
                width: 1000.0,
                height: 700.0,
                scale_factor: 1.0,
            },
        });

        let events = editor.apply(Message::System {
            event: SystemEvent::Resize {
                width: 1200.0,
                height: 800.0,
                scale_factor: 1.0,
            },
        });

        let has_render_invalidated = events
            .iter()
            .any(|e| matches!(e, EditorEvent::RenderInvalidated));
        assert!(
            !has_render_invalidated,
            "paginated mode must not emit RenderInvalidated on resize"
        );
    }

    #[test]
    fn resize_continuous_width_change_emits_render_invalidated() {
        let (state, _t1) = state! {
            doc {
                root (
                    layout_mode: LayoutMode::Continuous { max_width: 800 }
                ) [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("hello") }
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        // Establish initial layout fingerprint at effective_width=800 so the second resize
        // (shrinking effective_width to 500) is recognized as a real layout-affecting change.
        editor.apply(Message::System {
            event: SystemEvent::Resize {
                width: 1000.0,
                height: 600.0,
                scale_factor: 1.0,
            },
        });

        // Shrink viewport to 500 → effective_viewport_width becomes min(800, 500) = 500 → fingerprint changes.
        let events = editor.apply(Message::System {
            event: SystemEvent::Resize {
                width: 500.0,
                height: 600.0,
                scale_factor: 1.0,
            },
        });

        let has_render_invalidated = events
            .iter()
            .any(|e| matches!(e, EditorEvent::RenderInvalidated));
        assert!(
            has_render_invalidated,
            "continuous mode must emit RenderInvalidated when effective width shrinks"
        );
    }
}
