//! Selection-aware routing for Slice replacement.

use editor_clipboard::Slice;
use editor_state::{Selection, State};

use super::linear_fitter::{LinearFitOutcome, LinearFitPlan, fit_linear_slice};
use super::table_fitter::{
    CellFillPlan, TableGridPlan, fit_cell_fill, fit_table_grid, is_pure_table_grid_slice,
    selection_is_wholly_in_one_cell,
};
use crate::CommandError;

pub enum FitOutcome {
    Plan(SliceFitPlan),
    NoOp,
    NoFit,
}

pub struct SliceFitPlan {
    pub(crate) kind: SliceFitPlanKind,
}

pub(crate) enum SliceFitPlanKind {
    Linear(LinearFitPlan),
    TableGrid(TableGridPlan),
    CellFill(CellFillPlan),
}

pub fn fit_slice(
    state: &State,
    selection: Selection,
    slice: Slice,
) -> Result<FitOutcome, CommandError> {
    if slice.is_empty() {
        return Ok(FitOutcome::NoOp);
    }

    let Some(preflight) = slice.preflight() else {
        drop_slice_iteratively(slice);
        return Ok(FitOutcome::NoFit);
    };

    let view = state.view();
    let selection = if selection.is_collapsed() {
        selection.resolve(&view).ok_or_else(|| {
            CommandError::Corrupted("cannot resolve Slice insertion position".into())
        })?;
        selection
    } else {
        selection.normalize(&view).ok_or_else(|| {
            CommandError::Corrupted("cannot resolve Slice replacement selection".into())
        })?
    };
    let destination_depth = {
        let resolved = selection.resolve(&view).ok_or_else(|| {
            CommandError::Corrupted("cannot resolve normalized Slice selection".into())
        })?;
        resolved
            .from()
            .path()
            .len()
            .max(resolved.to().path().len())
            .saturating_sub(1)
    };
    if !preflight.fits_at_destination_depth(destination_depth) {
        drop_slice_iteratively(slice);
        return Ok(FitOutcome::NoFit);
    }

    if is_pure_table_grid_slice(&slice) {
        if selection
            .resolve(&view)
            .is_some_and(|resolved| resolved.as_cell_rect().is_some())
            || selection_is_wholly_in_one_cell(&view, selection)
        {
            return Ok(match fit_table_grid(&view, selection, &slice)? {
                Some(plan) => FitOutcome::Plan(SliceFitPlan {
                    kind: SliceFitPlanKind::TableGrid(plan),
                }),
                None => FitOutcome::NoFit,
            });
        }
    }

    if selection
        .resolve(&view)
        .is_some_and(|resolved| resolved.as_cell_rect().is_some())
    {
        if preflight.contains_table {
            return Ok(FitOutcome::NoFit);
        }
        return Ok(match fit_cell_fill(&view, selection, &slice) {
            Some(plan) => FitOutcome::Plan(SliceFitPlan {
                kind: SliceFitPlanKind::CellFill(plan),
            }),
            None => FitOutcome::NoFit,
        });
    }

    if preflight.contains_table && surviving_table_destination(&view, selection) {
        return Ok(FitOutcome::NoFit);
    }

    match fit_linear_slice(&view, selection, slice)? {
        LinearFitOutcome::Plan(plan) => Ok(FitOutcome::Plan(SliceFitPlan {
            kind: SliceFitPlanKind::Linear(plan),
        })),
        LinearFitOutcome::NoOp => Ok(FitOutcome::NoOp),
        LinearFitOutcome::NoFit => Ok(FitOutcome::NoFit),
    }
}

fn drop_slice_iteratively(mut slice: Slice) {
    let mut stack = std::mem::take(&mut slice.content);
    while let Some(mut fragment) = stack.pop() {
        stack.append(&mut fragment.children);
    }
}

fn surviving_table_destination(view: &editor_model::DocView, selection: Selection) -> bool {
    let Some(resolved) = selection.resolve(view) else {
        return false;
    };
    for endpoint in [resolved.from().node(), resolved.to().node()] {
        let Some(node) = view.node(endpoint) else {
            continue;
        };
        if node.ancestors().any(|ancestor| {
            ancestor.node_type() == editor_model::NodeType::Table
                && !resolved.contains_subtree(&ancestor)
        }) {
            return true;
        }
    }
    false
}
