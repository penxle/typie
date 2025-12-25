use crate::model::{LayoutMode, Mark, Node, NodeId};
use crate::runtime::{Effect, Runtime};
use crate::types::{Theme, WritingSystem};
use crate::utils::detect_writing_systems;
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

        let (fonts, writing_systems) = self.collect_doc_fonts_and_writing_systems();

        for (family, weight) in fonts {
            effects.push(Effect::FontUsageChanged { family, weight });
        }

        if !writing_systems.is_empty() {
            effects.push(Effect::WritingSystemsUsageChanged {
                systems: writing_systems.into_iter().collect(),
            });
        }

        effects
    }

    fn collect_doc_fonts_and_writing_systems(
        &self,
    ) -> (Vec<(String, u16)>, FxHashSet<WritingSystem>) {
        let mut fonts: FxHashSet<(String, u16)> = FxHashSet::default();
        let mut writing_systems: FxHashSet<WritingSystem> = FxHashSet::default();

        self.collect_from_node(NodeId::ROOT, &mut fonts, &mut writing_systems);

        (fonts.into_iter().collect(), writing_systems)
    }

    fn collect_from_node(
        &self,
        node_id: NodeId,
        fonts: &mut FxHashSet<(String, u16)>,
        writing_systems: &mut FxHashSet<WritingSystem>,
    ) {
        let Some(node_ref) = self.doc().node(node_id) else {
            return;
        };

        if let Node::Text(text_node) = node_ref.node() {
            let text_content = text_node.text.to_string();
            for ws in detect_writing_systems(&text_content) {
                writing_systems.insert(ws);
            }

            for (_, marks) in text_node.text.get_rich_text_segments() {
                let mut family = String::from("Pretendard");
                let mut weight = 400u16;

                for mark in &marks {
                    match mark {
                        Mark::FontFamily(f) => family = f.family.clone(),
                        Mark::FontWeight(w) => weight = w.weight,
                        _ => {}
                    }
                }

                fonts.insert((family, weight));
            }
        }

        for child in node_ref.children() {
            self.collect_from_node(child.node_id(), fonts, writing_systems);
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

    pub(crate) fn handle_resize(&mut self, viewport_width: f32, scale_factor: f64) -> Vec<Effect> {
        let viewport_changed = self.viewport_width != viewport_width;
        self.viewport_width = viewport_width;

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
}
