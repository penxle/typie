use crate::model::{CONTINUOUS_PAGE_MARGIN, LayoutMode};
use crate::runtime::{Effect, Runtime};
use crate::types::Theme;

impl Runtime {
    pub(crate) fn handle_initialize(
        &mut self,
        theme: Theme,
        viewport_width: f32,
        viewport_height: f32,
        scale_factor: f64,
    ) -> Vec<Effect> {
        self.renderer.set_theme(theme);
        self.layout_engine
            .set_viewport(viewport_width, viewport_height);

        let layout_mode = self.doc().settings().layout_mode;
        self.sync_layout_width(layout_mode);
        self.layout_engine.set_scale_factor(scale_factor);

        let mut effects = vec![
            Effect::DocChanged,
            Effect::SelectionChanged,
            Effect::ExternalElementChanged,
            Effect::SettingsChanged,
            Effect::LayoutChanged,
        ];

        let fonts = self.collect_doc_fonts_from_nodes([crate::model::NodeId::ROOT]);

        for (family, weight, font_codepoints) in fonts {
            effects.push(Effect::FontDetected {
                family,
                weight,
                codepoints: font_codepoints.into_iter().collect(),
            });
        }

        effects
    }

    pub(crate) fn handle_set_layout_mode(&mut self, mode: LayoutMode) -> Vec<Effect> {
        self.transact(|tr| tr.set_layout_mode(mode))
    }

    pub(crate) fn handle_resize(
        &mut self,
        viewport_width: f32,
        viewport_height: f32,
        scale_factor: f64,
    ) -> Vec<Effect> {
        let viewport_changed = self.layout_engine.viewport_width() != viewport_width;
        self.layout_engine
            .set_viewport(viewport_width, viewport_height);

        let layout_mode = self.doc().settings().layout_mode;
        let width_changed = self.sync_layout_width(layout_mode);
        let scale_changed = self.layout_engine.scale_factor() != scale_factor;
        self.layout_engine.set_scale_factor(scale_factor);

        if width_changed || scale_changed || viewport_changed {
            vec![Effect::FullLayoutInvalidation, Effect::LayoutChanged]
        } else {
            vec![]
        }
    }

    pub(crate) fn sync_layout_width(&mut self, layout_mode: LayoutMode) -> bool {
        let new_width = match layout_mode {
            LayoutMode::Paginated { page_width, .. } => page_width,
            LayoutMode::Continuous { max_width } => {
                let margin = CONTINUOUS_PAGE_MARGIN;
                let max_page_width = max_width + 2.0 * margin;
                self.layout_engine.viewport_width().min(max_page_width)
            }
        };
        let width_changed = self.layout_engine.width() != new_width;
        if width_changed {
            self.layout_engine.set_width(new_width);
        }
        width_changed
    }

    pub(crate) fn handle_set_theme(&mut self, theme: Theme) -> Vec<Effect> {
        self.renderer.set_theme(theme);
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
