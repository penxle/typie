use hashbrown::HashMap;

use super::{
    compare_navigation_band_entry, is_navigable_entry, navigable_above_at_x,
    navigable_above_at_x_filtered, navigable_below_at_x, navigable_below_at_x_filtered,
};
use crate::query::layout_index::{LayoutEntry, LayoutIndex};

const TARGET_EPSILON: f32 = 0.5;

pub(super) fn below<'a>(
    layout_index: &'a LayoutIndex,
    entry: &'a LayoutEntry,
    x: f32,
) -> Option<&'a LayoutEntry> {
    if let Some(scope) = layout_index.scope_for_entry(entry) {
        navigable_below_at_x_filtered(layout_index, entry.rect.bottom(), x, |candidate| {
            !std::ptr::eq(candidate, entry) && layout_index.entry_is_in_scope(candidate, scope)
        })
        .or_else(|| below_exiting_scope(layout_index, scope, x))
    } else {
        navigable_below_at_x(layout_index, entry.rect.bottom(), x)
    }
}

pub(super) fn above<'a>(
    layout_index: &'a LayoutIndex,
    entry: &'a LayoutEntry,
    x: f32,
) -> Option<&'a LayoutEntry> {
    if let Some(scope) = layout_index.scope_for_entry(entry) {
        navigable_above_at_x_filtered(layout_index, entry.rect.y, x, |candidate| {
            !std::ptr::eq(candidate, entry) && layout_index.entry_is_in_scope(candidate, scope)
        })
        .or_else(|| above_exiting_scope(layout_index, scope, x))
    } else {
        navigable_above_at_x(layout_index, entry.rect.y, x)
    }
}

fn below_exiting_scope<'a>(
    layout_index: &'a LayoutIndex,
    scope: &'a LayoutEntry,
    x: f32,
) -> Option<&'a LayoutEntry> {
    let boundary_y = scope.rect.bottom();
    let mut scoped_groups = Vec::new();
    let mut scoped_group_by_scope = HashMap::new();
    let mut targets = Vec::new();

    for entry in layout_index.entries() {
        if !is_navigable_entry(layout_index, entry) {
            continue;
        }
        let Some(candidate_scope) = layout_index.containing_scope_for_entry(entry) else {
            if entry.rect.y >= boundary_y - TARGET_EPSILON {
                targets.push(VerticalTarget {
                    band: entry,
                    landing: entry,
                });
            }
            continue;
        };
        // Adjacent table-row scope rects can touch or overlap at collapsed borders.
        // The scope top separates vertical scope bands; boundary_y still gates the
        // landing entry inside the chosen scope.
        if std::ptr::eq(candidate_scope, scope)
            || candidate_scope.rect.y <= scope.rect.y + TARGET_EPSILON
            || entry.rect.y < boundary_y - TARGET_EPSILON
        {
            continue;
        }
        push_scoped_landing_candidate(
            &mut scoped_groups,
            &mut scoped_group_by_scope,
            candidate_scope,
            entry,
        );
    }

    for group in scoped_groups {
        if let Some(landing) = closest_below(
            group.landings.into_iter().map(|entry| VerticalTarget {
                band: entry,
                landing: entry,
            }),
            x,
        ) {
            targets.push(VerticalTarget {
                band: group.scope,
                landing,
            });
        }
    }

    closest_below(targets.into_iter(), x)
}

fn above_exiting_scope<'a>(
    layout_index: &'a LayoutIndex,
    scope: &'a LayoutEntry,
    x: f32,
) -> Option<&'a LayoutEntry> {
    let boundary_y = scope.rect.y;
    let mut scoped_groups = Vec::new();
    let mut scoped_group_by_scope = HashMap::new();
    let mut targets = Vec::new();

    for entry in layout_index.entries() {
        if !is_navigable_entry(layout_index, entry) {
            continue;
        }
        let Some(candidate_scope) = layout_index.containing_scope_for_entry(entry) else {
            if entry.rect.bottom() <= boundary_y + TARGET_EPSILON {
                targets.push(VerticalTarget {
                    band: entry,
                    landing: entry,
                });
            }
            continue;
        };
        // Use scope top to reject same-row peer scopes even when collapsed borders
        // make the previous row's bottom overlap this scope boundary.
        if std::ptr::eq(candidate_scope, scope)
            || candidate_scope.rect.y >= scope.rect.y - TARGET_EPSILON
            || entry.rect.bottom() > boundary_y + TARGET_EPSILON
        {
            continue;
        }
        push_scoped_landing_candidate(
            &mut scoped_groups,
            &mut scoped_group_by_scope,
            candidate_scope,
            entry,
        );
    }

    for group in scoped_groups {
        if let Some(landing) = closest_above(
            group.landings.into_iter().map(|entry| VerticalTarget {
                band: entry,
                landing: entry,
            }),
            x,
        ) {
            targets.push(VerticalTarget {
                band: group.scope,
                landing,
            });
        }
    }

    closest_above(targets.into_iter(), x)
}

#[derive(Clone, Copy)]
struct VerticalTarget<'a> {
    // Geometry used to choose the next vertical band. For scoped targets this is
    // the scope entry; for scopeless targets it is the navigable entry itself.
    band: &'a LayoutEntry,
    landing: &'a LayoutEntry,
}

struct ScopedLandingCandidates<'a> {
    scope: &'a LayoutEntry,
    landings: Vec<&'a LayoutEntry>,
}

fn push_scoped_landing_candidate<'a>(
    groups: &mut Vec<ScopedLandingCandidates<'a>>,
    group_by_scope: &mut HashMap<*const LayoutEntry, usize>,
    scope: &'a LayoutEntry,
    landing: &'a LayoutEntry,
) {
    let key = scope as *const LayoutEntry;
    let group_idx = if let Some(&idx) = group_by_scope.get(&key) {
        idx
    } else {
        let idx = groups.len();
        group_by_scope.insert(key, idx);
        groups.push(ScopedLandingCandidates {
            scope,
            landings: Vec::new(),
        });
        idx
    };
    groups[group_idx].landings.push(landing);
}

fn closest_below<'a>(
    targets: impl Iterator<Item = VerticalTarget<'a>>,
    x: f32,
) -> Option<&'a LayoutEntry> {
    let candidates: Vec<_> = targets.collect();
    let top = candidates
        .iter()
        .copied()
        .min_by(|a, b| compare_below_band(a.band, b.band))?;
    let band_end = top.band.rect.bottom();
    candidates
        .into_iter()
        .filter(|target| target.band.rect.y < band_end)
        .min_by(|a, b| compare_vertical_target_band(a, b, x, true))
        .map(|target| target.landing)
}

fn closest_above<'a>(
    targets: impl Iterator<Item = VerticalTarget<'a>>,
    x: f32,
) -> Option<&'a LayoutEntry> {
    let candidates: Vec<_> = targets.collect();
    let bottom = candidates
        .iter()
        .copied()
        .min_by(|a, b| compare_above_band(a.band, b.band))?;
    let band_start = bottom.band.rect.y;
    candidates
        .into_iter()
        .filter(|target| target.band.rect.bottom() > band_start)
        .min_by(|a, b| compare_vertical_target_band(a, b, x, false))
        .map(|target| target.landing)
}

fn compare_below_band(a: &LayoutEntry, b: &LayoutEntry) -> std::cmp::Ordering {
    a.rect
        .y
        .total_cmp(&b.rect.y)
        .then(a.rect.x.total_cmp(&b.rect.x))
}

fn compare_above_band(a: &LayoutEntry, b: &LayoutEntry) -> std::cmp::Ordering {
    b.rect
        .bottom()
        .total_cmp(&a.rect.bottom())
        .then(a.rect.x.total_cmp(&b.rect.x))
}

fn compare_vertical_target_band(
    a: &VerticalTarget<'_>,
    b: &VerticalTarget<'_>,
    x: f32,
    forward: bool,
) -> std::cmp::Ordering {
    compare_navigation_band_entry(a.band, b.band, x, forward)
}
