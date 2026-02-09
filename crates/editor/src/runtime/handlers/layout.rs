use crate::model::{FontFamilyMark, FontWeightMark, LayoutMode, Mark, Node, NodeId};
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

        self.collect_from_node(
            NodeId::ROOT,
            &mut fonts,
            &mut codepoints,
            FontFamilyMark::default().family,
            FontWeightMark::default().weight,
        );

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
        default_family: String,
        default_weight: u16,
    ) {
        let Some(node_ref) = self.doc().node(node_id) else {
            return;
        };

        let overrides = node_ref.node().font_overrides();
        let child_family = overrides.family.unwrap_or(default_family.clone());
        let child_weight = overrides.weight.unwrap_or(default_weight);

        if let Node::Text(text_node) = node_ref.node() {
            for (text, marks) in text_node.text.get_rich_text_segments() {
                let mut family = default_family.clone();
                let mut weight = default_weight;

                for mark in &marks {
                    match mark {
                        Mark::FontFamily(f) => family = f.family.clone(),
                        Mark::FontWeight(w) => weight = w.weight,
                        _ => {}
                    }
                }

                let font_cps = fonts.entry((family, weight)).or_default();
                for ch in text.chars() {
                    codepoints.insert(ch as u32);
                    font_cps.insert(ch as u32);
                }
            }
        }

        for child in node_ref.children() {
            self.collect_from_node(
                child.node_id(),
                fonts,
                codepoints,
                child_family.clone(),
                child_weight,
            );
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
                let margin = crate::model::CONTINUOUS_PAGE_MARGIN;
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
