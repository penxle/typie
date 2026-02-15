use crate::model::{Annotation, CONTINUOUS_PAGE_MARGIN, LayoutMode, Node, NodeId, Style};
use crate::runtime::{Effect, Runtime};
use crate::types::Theme;
use rustc_hash::FxHashSet;

impl Runtime {
    pub(crate) fn handle_initialize(&mut self, theme: Theme) -> Vec<Effect> {
        self.renderer.set_theme(theme);

        let mut effects = vec![
            Effect::DocChanged,
            Effect::SelectionChanged,
            Effect::ExternalElementChanged,
            Effect::SettingsChanged,
            Effect::LayoutChanged,
        ];

        let (fonts, codepoints) = self.collect_doc_fonts_and_codepoints();

        for (family, weight, font_codepoints) in fonts {
            effects.push(Effect::FontDetected {
                family,
                weight,
                // codepoints: std::iter::chain(font_codepoints.into_iter(), std::iter::once(0x200B))
                //     .collect(),
                codepoints: font_codepoints.into_iter().collect(),
            });
        }

        effects.push(Effect::CodepointDetected {
            // codepoints: std::iter::chain(codepoints.into_iter(), std::iter::once(0x200B)).collect(),
            codepoints: codepoints.into_iter().collect(),
        });

        effects
    }

    fn collect_doc_fonts_and_codepoints(
        &self,
    ) -> (Vec<(String, u16, FxHashSet<u32>)>, FxHashSet<u32>) {
        let mut fonts: rustc_hash::FxHashMap<(String, u16), FxHashSet<u32>> =
            rustc_hash::FxHashMap::default();
        let mut codepoints: FxHashSet<u32> = FxHashSet::default();

        self.collect_from_node(NodeId::ROOT, &mut fonts, &mut codepoints);

        let fonts = fonts
            .into_iter()
            .map(|((family, weight), cps)| (family, weight, cps))
            .collect();

        (fonts, codepoints)
    }

    fn collect_from_node(
        &self,
        node_id: NodeId,
        fonts: &mut rustc_hash::FxHashMap<(String, u16), FxHashSet<u32>>,
        codepoints: &mut FxHashSet<u32>,
    ) {
        let Some(node_ref) = self.doc().node(node_id) else {
            return;
        };

        if let Node::Text(text_node) = node_ref.node() {
            let defaults = self.doc().default_attrs();
            let overrides = node_ref
                .parent()
                .map(|p| p.node().style_overrides())
                .unwrap_or_default();
            let ruby_family = defaults.font_family().to_string();
            let ruby_weight = defaults.font_weight();

            for seg in text_node.text.get_segments() {
                let family = seg
                    .styles
                    .iter()
                    .find_map(|s| match s {
                        Style::FontFamily(f) => Some(f.family.clone()),
                        _ => None,
                    })
                    .or_else(|| {
                        overrides.iter().find_map(|s| match s {
                            Style::FontFamily(f) => Some(f.family.clone()),
                            _ => None,
                        })
                    })
                    .unwrap_or_else(|| defaults.font_family().to_string());
                let weight = seg
                    .styles
                    .iter()
                    .find_map(|s| match s {
                        Style::FontWeight(w) => Some(w.weight),
                        _ => None,
                    })
                    .or_else(|| {
                        overrides.iter().find_map(|s| match s {
                            Style::FontWeight(w) => Some(w.weight),
                            _ => None,
                        })
                    })
                    .unwrap_or_else(|| defaults.font_weight());

                let font_cps = fonts.entry((family, weight)).or_default();
                for ch in seg.text.chars() {
                    codepoints.insert(ch as u32);
                    font_cps.insert(ch as u32);
                }

                for annotation in &seg.annotations {
                    if let Annotation::Ruby(ruby_ann) = annotation {
                        let ruby_font_cps =
                            fonts.entry((ruby_family.clone(), ruby_weight)).or_default();
                        for ch in ruby_ann.text.chars() {
                            codepoints.insert(ch as u32);
                            ruby_font_cps.insert(ch as u32);
                        }
                    }
                }
            }
        }

        for child in node_ref.children() {
            self.collect_from_node(child.node_id(), fonts, codepoints);
        }
    }

    pub(crate) fn handle_set_layout_mode(&mut self, mode: LayoutMode) -> Vec<Effect> {
        let _ = self.state.doc.update_settings(|s| s.layout_mode = mode);

        let new_width = self.calculate_page_width(mode);
        self.set_width(new_width);

        vec![
            Effect::LayoutChanged,
            Effect::SettingsChanged,
            Effect::DocChanged,
        ]
    }

    pub(crate) fn handle_resize(
        &mut self,
        viewport_width: f32,
        viewport_height: f32,
        scale_factor: f64,
    ) -> Vec<Effect> {
        let viewport_changed = self.viewport_width != viewport_width;
        self.viewport_width = viewport_width;
        self.viewport_height = viewport_height;

        let layout_mode = self.doc().settings().layout_mode;
        let new_width = self.calculate_page_width(layout_mode);

        let width_changed = self.width != new_width;
        let scale_changed = self.scale_factor != scale_factor;

        self.set_width(new_width);
        self.set_scale_factor(scale_factor);

        if width_changed || scale_changed || viewport_changed {
            vec![Effect::LayoutChanged]
        } else {
            vec![]
        }
    }

    fn calculate_page_width(&self, layout_mode: LayoutMode) -> f32 {
        match layout_mode {
            LayoutMode::Paginated { page_width, .. } => page_width,
            LayoutMode::Continuous { max_width } => {
                let margin = CONTINUOUS_PAGE_MARGIN;
                let max_page_width = max_width + 2.0 * margin;
                self.viewport_width.min(max_page_width)
            }
        }
    }

    pub(crate) fn handle_set_theme(&mut self, theme: Theme) -> Vec<Effect> {
        self.renderer.set_theme(theme);
        vec![Effect::LayoutChanged]
    }

    pub(crate) fn handle_fonts_loaded(&mut self) -> Vec<Effect> {
        self.layout_cache.borrow_mut().invalidate_all();
        vec![Effect::LayoutChanged]
    }

    pub(crate) fn handle_set_focused(&mut self, focused: bool) -> Vec<Effect> {
        if self.is_focused != focused {
            self.is_focused = focused;
            vec![Effect::LayoutChanged]
        } else {
            vec![]
        }
    }
}
