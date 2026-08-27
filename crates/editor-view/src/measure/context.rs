use std::cell::RefCell;

use crate::glyph_run::ShapedGlyphObservation;
use crate::measure::text::measure::{LineStrutExpansion, MeasuredLine};
use crate::view_state::{GapPhantom, PendingOverlay};
use editor_crdt::Dot;
use hashbrown::HashMap;

#[derive(Debug, Clone, Default)]
pub(crate) struct MeasureContext {
    pub fold_states: HashMap<Dot, bool>,
    pub external_heights: HashMap<Dot, f32>,
    pub gap_phantom: Option<GapPhantom>,
    pub pending_overlay: Option<PendingOverlay>,
    pub pending_caret_expansion: Option<LineStrutExpansion>,
    pub(crate) shaped_glyph_observations: RefCell<Vec<ShapedGlyphObservation>>,
}

impl MeasureContext {
    pub fn fold_expanded(&self, node: &Dot) -> bool {
        self.fold_states.get(node).copied().unwrap_or(false)
    }

    pub fn external_height(&self, node: &Dot) -> Option<f32> {
        self.external_heights.get(node).copied()
    }

    pub fn gap_phantom_index(&self, node: &Dot) -> Option<usize> {
        self.gap_phantom
            .as_ref()
            .filter(|gp| &gp.parent == node)
            .map(|gp| gp.index)
    }

    pub fn pending_for(&self, node: &Dot) -> Option<&editor_state::PendingModifiers> {
        self.pending_overlay
            .as_ref()
            .filter(|overlay| &overlay.position.node == node)
            .map(|overlay| &overlay.modifiers)
    }

    pub fn pending_caret_for(
        &self,
        node: &Dot,
    ) -> Option<(&editor_state::Position, &LineStrutExpansion)> {
        let overlay = self
            .pending_overlay
            .as_ref()
            .filter(|overlay| &overlay.position.node == node)?;
        Some((&overlay.position, self.pending_caret_expansion.as_ref()?))
    }

    pub fn observe_glyphs(
        &self,
        family_id: u16,
        weight: u16,
        glyph_ids: impl IntoIterator<Item = u32>,
    ) {
        let mut glyph_ids: Vec<u16> = glyph_ids
            .into_iter()
            .filter_map(|gid| u16::try_from(gid).ok())
            .filter(|&gid| gid != 0)
            .collect();
        glyph_ids.sort_unstable();
        glyph_ids.dedup();
        if glyph_ids.is_empty() {
            return;
        }
        self.shaped_glyph_observations
            .borrow_mut()
            .push(ShapedGlyphObservation {
                family_id,
                weight,
                glyph_ids,
            });
    }

    pub fn observe_lines(&self, lines: &[MeasuredLine]) {
        for line in lines {
            for run in &line.glyph_runs {
                self.observe_glyphs(
                    run.family_id,
                    run.weight,
                    run.glyphs.iter().map(|glyph| glyph.id),
                );
            }
            for annotation in &line.ruby_annotations {
                for run in &annotation.glyph_runs {
                    self.observe_glyphs(
                        run.family_id,
                        run.weight,
                        run.glyphs.iter().map(|glyph| glyph.id),
                    );
                }
            }
        }
    }

    pub fn take_shaped_glyph_observations(&self) -> Vec<ShapedGlyphObservation> {
        std::mem::take(&mut *self.shaped_glyph_observations.borrow_mut())
    }
}

pub(crate) fn measure_context(vs: &crate::view_state::ViewState) -> MeasureContext {
    MeasureContext {
        fold_states: vs.fold_states.clone(),
        external_heights: vs.external_heights.clone(),
        gap_phantom: vs.gap_phantom.as_ref().map(|gp| GapPhantom {
            parent: gp.parent,
            index: gp.index,
        }),
        pending_overlay: vs.pending_overlay.clone(),
        pending_caret_expansion: None,
        shaped_glyph_observations: RefCell::default(),
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;
    use editor_model::Modifier;
    use editor_state::PendingModifier;
    use hashbrown::HashMap;

    use crate::view_state::{GapPhantom, PendingOverlay, ViewState};

    use super::{MeasureContext, measure_context};

    #[test]
    fn fold_expanded_default_and_set() {
        let id = Dot::new(1, 1);
        let ctx = MeasureContext::default();
        assert!(!ctx.fold_expanded(&id));

        let ctx_true = MeasureContext {
            fold_states: HashMap::from([(id, true)]),
            ..Default::default()
        };
        assert!(ctx_true.fold_expanded(&id));
    }

    #[test]
    fn external_height_some_none() {
        let id = Dot::new(1, 1);
        let ctx = MeasureContext {
            external_heights: HashMap::from([(id, 150.0)]),
            ..Default::default()
        };
        assert_eq!(ctx.external_height(&id), Some(150.0));

        let other = Dot::new(1, 2);
        assert_eq!(ctx.external_height(&other), None);
    }

    #[test]
    fn gap_phantom_index_matches_only_parent() {
        let a = Dot::new(1, 1);
        let b = Dot::new(1, 2);
        let ctx = MeasureContext {
            gap_phantom: Some(GapPhantom {
                parent: a,
                index: 2,
            }),
            ..Default::default()
        };
        assert_eq!(ctx.gap_phantom_index(&a), Some(2));
        assert_eq!(ctx.gap_phantom_index(&b), None);
    }

    #[test]
    fn pending_for_matches_only_own_elem() {
        let a = Dot::new(1, 1);
        let b = Dot::new(1, 2);
        let modifiers = vec![PendingModifier::Set {
            modifier: Modifier::Bold,
        }];
        let ctx = MeasureContext {
            pending_overlay: Some(PendingOverlay {
                position: editor_state::Position::new(a, 0),
                modifiers: modifiers.clone(),
            }),
            ..Default::default()
        };
        assert_eq!(ctx.pending_for(&a), Some(&modifiers));
        assert_eq!(ctx.pending_for(&b), None);
    }

    #[test]
    fn measure_context_copies_state_fields() {
        let p1 = Dot::new(1, 1);
        let i1 = Dot::new(1, 2);
        let modifiers = vec![PendingModifier::Set {
            modifier: Modifier::Bold,
        }];
        let mut vs = ViewState::new();
        vs.fold_states.insert(p1, false);
        vs.external_heights.insert(i1, 200.0);
        vs.gap_phantom = Some(GapPhantom {
            parent: p1,
            index: 1,
        });
        vs.pending_overlay = Some(PendingOverlay {
            position: editor_state::Position::new(p1, 0),
            modifiers: modifiers.clone(),
        });

        let ctx = measure_context(&vs);

        assert_eq!(ctx.fold_states.get(&p1), Some(&false));
        assert_eq!(ctx.external_heights.get(&i1), Some(&200.0));
        assert_eq!(
            ctx.gap_phantom,
            Some(GapPhantom {
                parent: p1,
                index: 1
            })
        );
        assert_eq!(
            ctx.pending_overlay,
            Some(PendingOverlay {
                position: editor_state::Position::new(p1, 0),
                modifiers,
            })
        );
    }

    #[test]
    fn shaped_glyph_observations_are_deduplicated_per_run_and_drained() {
        let ctx = MeasureContext::default();

        ctx.observe_glyphs(3, 400, [7, 2, 7, 0]);

        assert_eq!(
            ctx.take_shaped_glyph_observations(),
            vec![crate::glyph_run::ShapedGlyphObservation {
                family_id: 3,
                weight: 400,
                glyph_ids: vec![2, 7],
            }]
        );
        assert!(ctx.take_shaped_glyph_observations().is_empty());
    }
}
