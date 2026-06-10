use editor_common::Rect;
use editor_model::NodeId;
use editor_state::{Affinity, Position};
use rstar::{AABB, RTree, RTreeObject};
use std::collections::HashMap;

use crate::page::{LayoutPage, PageRect};
use crate::paginate::*;

type LayoutEntryId = usize;

#[derive(Debug)]
pub(crate) struct LayoutIndex {
    tree: LayoutTree,
    pages: Vec<LayoutPage>,
    entries: Vec<LayoutEntry>,
    boxes_by_node_id: HashMap<NodeId, LayoutEntryId>,
    spatial: RTree<SpatialEntry>,
}

#[derive(Debug, Clone)]
pub(crate) struct LayoutEntry {
    pub(crate) rect: Rect,
    path: Vec<usize>,
    ancestors: Vec<NodeId>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LayoutPoint {
    pub(crate) page_idx: usize,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) page_y_start: f32,
}

#[derive(Debug, Clone, Copy)]
struct SpatialEntry {
    entry_id: LayoutEntryId,
    page_idx: usize,
    bounds: AABB<[f32; 2]>,
}

struct LayoutIndexBuilder<'a> {
    pages: &'a [LayoutPage],
    entries: Vec<LayoutEntry>,
    boxes_by_node_id: HashMap<NodeId, LayoutEntryId>,
    ancestors: Vec<NodeId>,
    path: Vec<usize>,
    spatial: Vec<SpatialEntry>,
}

impl LayoutIndex {
    pub(crate) fn new(tree: LayoutTree, pages: &[LayoutPage]) -> Self {
        let mut builder = LayoutIndexBuilder {
            pages,
            entries: Vec::new(),
            boxes_by_node_id: HashMap::new(),
            ancestors: Vec::new(),
            path: Vec::new(),
            spatial: Vec::new(),
        };
        builder.build_node(&tree.root);
        Self {
            tree,
            pages: pages.to_vec(),
            entries: builder.entries,
            boxes_by_node_id: builder.boxes_by_node_id,
            spatial: RTree::bulk_load(builder.spatial),
        }
    }

    pub(crate) fn tree(&self) -> &LayoutTree {
        &self.tree
    }

    pub(crate) fn pages(&self) -> &[LayoutPage] {
        &self.pages
    }

    pub(crate) fn page(&self, page_idx: usize) -> Option<&LayoutPage> {
        self.pages.get(page_idx)
    }

    pub(crate) fn point(&self, page_idx: usize, x: f32, page_y: f32) -> Option<LayoutPoint> {
        let page = self.pages.get(page_idx)?;
        let y = page_y + page.y_start;
        Some(LayoutPoint {
            page_idx,
            x,
            y,
            page_y_start: page.y_start,
        })
    }

    pub(crate) fn exact_entry(
        &self,
        point: LayoutPoint,
        include: impl Fn(&LayoutEntry, &LayoutNode) -> bool,
    ) -> Option<&LayoutEntry> {
        self.exact_entry_with(point, |entry, node| include(entry, node).then_some(()))
            .map(|(entry, ())| entry)
    }

    pub(crate) fn exact_entry_with<T>(
        &self,
        point: LayoutPoint,
        map: impl Fn(&LayoutEntry, &LayoutNode) -> Option<T>,
    ) -> Option<(&LayoutEntry, T)> {
        self.smallest_entry_at(point, map)
            .map(|(entry_id, value)| (&self.entries[entry_id], value))
    }

    pub(crate) fn closest_entry(
        &self,
        point: LayoutPoint,
        include: impl Fn(&LayoutEntry, &LayoutNode) -> bool,
    ) -> Option<&LayoutEntry> {
        self.closest_entry_with(point, |entry, node| include(entry, node).then_some(()))
            .map(|(entry, ())| entry)
    }

    pub(crate) fn closest_entry_with<T>(
        &self,
        point: LayoutPoint,
        map: impl Fn(&LayoutEntry, &LayoutNode) -> Option<T>,
    ) -> Option<(&LayoutEntry, T)> {
        self.entries_on_page(point.page_idx)
            .into_iter()
            .filter_map(|entry| {
                let node = entry.node(self)?;
                map(entry, node)
                    .map(|value| (distance_key(&entry.rect, point.x, point.y), entry, value))
            })
            .min_by(|a, b| compare_distance_key(a.0, b.0))
            .map(|(_, entry, value)| (entry, value))
    }

    pub(crate) fn entry_for_position(&self, pos: &Position) -> Option<&LayoutEntry> {
        let mut candidates = self.entries.iter().filter(|entry| {
            entry
                .node(self)
                .is_some_and(|node| position_matches_node(node, pos))
        });
        match pos.affinity {
            Affinity::Upstream => candidates.next(),
            Affinity::Downstream => candidates.next_back(),
        }
    }

    pub(crate) fn page_rect(&self, rect: Rect) -> Option<PageRect> {
        let page_idx = self
            .pages
            .iter()
            .position(|page| rect.y >= page.y_start && rect.y < page.y_end)?;
        let y_start = self.pages[page_idx].y_start;
        Some(PageRect::new(
            page_idx,
            Rect::from_xywh(rect.x, rect.y - y_start, rect.width, rect.height),
        ))
    }

    pub(crate) fn page_idx_for_y(&self, y: f32) -> Option<usize> {
        self.pages
            .iter()
            .position(|page| y >= page.y_start && y <= page.y_end)
    }

    pub(crate) fn page_y_start(&self, page_idx: usize) -> Option<f32> {
        self.pages.get(page_idx).map(|page| page.y_start)
    }

    pub(crate) fn box_entry(&self, node_id: NodeId) -> Option<&LayoutEntry> {
        self.boxes_by_node_id
            .get(&node_id)
            .map(|&entry_id| &self.entries[entry_id])
    }

    pub(crate) fn box_rect(&self, node_id: NodeId) -> Option<Rect> {
        Some(self.box_entry(node_id)?.rect)
    }

    pub(crate) fn box_page_rects(&self, ids: &[NodeId]) -> Vec<PageRect> {
        let mut rects = Vec::new();
        for &id in ids {
            let Some(entry) = self.box_entry(id) else {
                continue;
            };
            let node_top = entry.rect.y;
            let node_bottom = entry.rect.bottom();
            for (page_idx, page) in self.pages.iter().enumerate() {
                if node_bottom <= page.y_start || node_top >= page.y_end {
                    continue;
                }
                let top = node_top.max(page.y_start);
                let bottom = node_bottom.min(page.y_end);
                rects.push(PageRect::new(
                    page_idx,
                    Rect::from_xywh(
                        entry.rect.x,
                        top - page.y_start,
                        entry.rect.width,
                        bottom - top,
                    ),
                ));
            }
        }
        rects
    }

    pub(crate) fn nearest_box(&self, point: LayoutPoint, ids: &[NodeId]) -> Option<NodeId> {
        ids.iter()
            .filter_map(|&id| {
                self.box_entry(id)
                    .map(|entry| (distance_key(&entry.rect, point.x, point.y), id))
            })
            .min_by(|a, b| compare_distance_key(a.0, b.0))
            .map(|(_, id)| id)
    }

    pub(crate) fn box_contains(&self, point: LayoutPoint, node_id: NodeId) -> bool {
        self.box_entry(node_id)
            .is_some_and(|entry| entry.rect.contains(point.x, point.y))
    }

    pub(crate) fn entries_on_page(&self, page_idx: usize) -> Vec<&LayoutEntry> {
        let Some(page) = self.pages.get(page_idx) else {
            return Vec::new();
        };
        let envelope = AABB::from_corners([-f32::MAX, page.y_start], [f32::MAX, page.y_end]);
        let mut entry_ids: Vec<_> = self
            .spatial
            .locate_in_envelope_intersecting(envelope)
            .filter(|spatial| spatial.page_idx == page_idx)
            .map(|spatial| spatial.entry_id)
            .collect();
        entry_ids.sort_unstable();
        entry_ids.dedup();
        entry_ids
            .into_iter()
            .map(|entry_id| &self.entries[entry_id])
            .collect()
    }

    pub(crate) fn entries(&self) -> std::slice::Iter<'_, LayoutEntry> {
        self.entries.iter()
    }

    pub(crate) fn direct_child_entries(
        &self,
        node_id: NodeId,
    ) -> impl Iterator<Item = &LayoutEntry> + '_ {
        self.entries
            .iter()
            .filter(move |entry| entry.ancestors.last() == Some(&node_id))
    }

    pub(crate) fn direct_child_entries_in_y_range(
        &self,
        node_id: NodeId,
        y_start: f32,
        y_end: f32,
    ) -> impl Iterator<Item = &LayoutEntry> + '_ {
        self.direct_child_entries(node_id)
            .filter(move |entry| rect_overlaps_y_range(&entry.rect, y_start, y_end))
    }

    fn smallest_entry_at<T>(
        &self,
        point: LayoutPoint,
        map: impl Fn(&LayoutEntry, &LayoutNode) -> Option<T>,
    ) -> Option<(LayoutEntryId, T)> {
        let envelope = AABB::from_point([point.x, point.y]);
        self.spatial
            .locate_in_envelope_intersecting(envelope)
            .filter(|spatial| spatial.page_idx == point.page_idx)
            .map(|spatial| spatial.entry_id)
            .filter(|&entry_id| self.entry_exactly_contains(entry_id, point.x, point.y))
            .filter_map(|entry_id| {
                let entry = &self.entries[entry_id];
                let node = entry.node(self)?;
                map(entry, node).map(|value| (entry_id, value))
            })
            .min_by(|a, b| {
                rect_area(&self.entries[a.0].rect).total_cmp(&rect_area(&self.entries[b.0].rect))
            })
    }

    fn entry_exactly_contains(&self, entry_id: LayoutEntryId, x: f32, y: f32) -> bool {
        let entry = &self.entries[entry_id];
        match entry.node(self).map(|node| &node.content) {
            Some(LayoutContent::Line(_)) => y >= entry.rect.y && y < entry.rect.bottom(),
            Some(LayoutContent::Box(_)) | Some(LayoutContent::Atom(_)) => entry.rect.contains(x, y),
            Some(LayoutContent::Spacing(SpacingKind::Gap { .. })) => entry.rect.contains(x, y),
            Some(LayoutContent::Spacing(SpacingKind::Fill)) | None => false,
        }
    }

    pub(crate) fn entry_index(&self, entry: &LayoutEntry) -> Option<usize> {
        self.entries
            .iter()
            .position(|candidate| std::ptr::eq(candidate, entry))
    }
}

impl LayoutEntry {
    pub(crate) fn node<'a>(&self, layout_index: &'a LayoutIndex) -> Option<&'a LayoutNode> {
        node_at_path(&layout_index.tree.root, &self.path)
    }

    pub(crate) fn content<'a>(&self, layout_index: &'a LayoutIndex) -> Option<&'a LayoutContent> {
        Some(&self.node(layout_index)?.content)
    }

    pub(crate) fn is_node(&self, layout_index: &LayoutIndex, node: &LayoutNode) -> bool {
        self.node(layout_index)
            .is_some_and(|entry_node| std::ptr::eq(entry_node, node))
    }

    pub(crate) fn ancestors(&self) -> &[NodeId] {
        &self.ancestors
    }
}

impl LayoutIndexBuilder<'_> {
    fn build_node(&mut self, node: &LayoutNode) {
        match &node.content {
            LayoutContent::Box(b) => {
                self.add_entry(node.rect);
                self.boxes_by_node_id
                    .insert(b.node_id, self.entries.len() - 1);
                self.ancestors.push(b.node_id);
                for (idx, child) in b.children.iter().enumerate() {
                    self.path.push(idx);
                    self.build_node(child);
                    self.path.pop();
                }
                self.ancestors.pop();
            }
            LayoutContent::Spacing(SpacingKind::Gap { .. }) => {
                if !self.ancestors.is_empty() {
                    self.add_entry(node.rect);
                }
            }
            LayoutContent::Line(line) if line.is_phantom => {}
            LayoutContent::Spacing(_) => {}
            LayoutContent::Line(_) | LayoutContent::Atom(_) => {
                self.add_entry(node.rect);
            }
        }
    }

    fn add_entry(&mut self, rect: Rect) -> LayoutEntryId {
        let entry_id = self.entries.len();
        self.entries.push(LayoutEntry {
            rect,
            path: self.path.clone(),
            ancestors: self.ancestors.clone(),
        });
        if rect.width > 0.0 && rect.height > 0.0 {
            let bounds = AABB::from_corners([rect.x, rect.y], [rect.right(), rect.bottom()]);
            for (page_idx, page) in self.pages.iter().enumerate() {
                if rect_overlaps_y_range(&rect, page.y_start, page.y_end) {
                    self.spatial.push(SpatialEntry {
                        entry_id,
                        page_idx,
                        bounds,
                    });
                }
            }
        }
        entry_id
    }
}

impl RTreeObject for SpatialEntry {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.bounds
    }
}

fn node_at_path<'a>(mut node: &'a LayoutNode, path: &[usize]) -> Option<&'a LayoutNode> {
    for &idx in path {
        let LayoutContent::Box(b) = &node.content else {
            return None;
        };
        node = b.children.get(idx)?;
    }
    Some(node)
}

fn rect_overlaps_y_range(rect: &Rect, y_start: f32, y_end: f32) -> bool {
    rect.y < y_end && rect.bottom() > y_start
}

fn position_matches_node(node: &LayoutNode, pos: &Position) -> bool {
    match &node.content {
        LayoutContent::Box(b) => b.attachment.is_some_and(|attachment| {
            position_attaches_to_child(pos, attachment.parent_id, attachment.index)
        }),
        LayoutContent::Line(line) => position_matches_line(line, pos),
        LayoutContent::Atom(atom) => {
            position_attaches_to_child(pos, atom.attachment.parent_id, atom.attachment.index)
        }
        LayoutContent::Spacing(_) => false,
    }
}

fn position_matches_line(line: &LayoutLine, pos: &Position) -> bool {
    if let Some(range) = &line.child_range
        && line.node_id == pos.node_id
        && pos.offset >= range.start
        && pos.offset <= range.end
    {
        return true;
    }
    line.glyph_runs.iter().any(|run| {
        run.node_id == pos.node_id
            && pos.offset >= run.offset
            && pos.offset <= run.offset + super::grapheme::run_codepoint_count(run)
    })
}

fn position_attaches_to_child(pos: &Position, parent_id: NodeId, index: usize) -> bool {
    if pos.node_id != parent_id {
        return false;
    }
    match pos.affinity {
        Affinity::Downstream => pos.offset == index,
        Affinity::Upstream => index.checked_add(1) == Some(pos.offset),
    }
}

fn compare_distance_key(a: (f32, f32), b: (f32, f32)) -> std::cmp::Ordering {
    match a.0.total_cmp(&b.0) {
        std::cmp::Ordering::Equal => a.1.total_cmp(&b.1),
        ordering => ordering,
    }
}

fn distance_key(rect: &Rect, x: f32, y: f32) -> (f32, f32) {
    (
        axis_distance(rect.y, rect.bottom(), y),
        axis_distance(rect.x, rect.right(), x),
    )
}

fn axis_distance(start: f32, end: f32, value: f32) -> f32 {
    if value < start {
        start - value
    } else if value > end {
        value - end
    } else {
        0.0
    }
}

fn rect_area(rect: &Rect) -> f32 {
    rect.width * rect.height
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::glyph_run::{GlyphRun, GraphemeSpan};
    use crate::style::{Alignment, BorderMode, BoxStyle, Direction};
    use editor_common::{EdgeInsets, Size};

    fn page(y_start: f32, y_end: f32) -> LayoutPage {
        LayoutPage::new(y_start, y_end, Size::new(440.0, y_end - y_start))
    }

    fn line_node(id: NodeId, x: f32, y: f32, text: &str, char_w: f32) -> LayoutNode {
        let len = text.chars().count();
        LayoutNode {
            rect: Rect::from_xywh(x, y, len as f32 * char_w, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: id,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(
                    id,
                    0,
                    text,
                    0.0,
                    vec![
                        GraphemeSpan {
                            advance: char_w,
                            codepoints: 1
                        };
                        len
                    ],
                )],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        }
    }

    fn box_node(id: NodeId, rect: Rect, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            rect,
            content: LayoutContent::Box(LayoutBox {
                node_id: id,
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    decorations: vec![],
                    monolithic: false,
                },
                children,
                attachment: None,
            }),
        }
    }

    fn attached_box_node(
        id: NodeId,
        parent_id: NodeId,
        index: usize,
        rect: Rect,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        let mut node = box_node(id, rect, children);
        let LayoutContent::Box(b) = &mut node.content else {
            unreachable!("box_node creates a box");
        };
        b.attachment = Some(ChildAttachment { parent_id, index });
        node
    }

    fn line_id(layout_index: &LayoutIndex, entry: &LayoutEntry) -> NodeId {
        match entry.content(layout_index) {
            Some(LayoutContent::Line(line)) => line.node_id,
            other => panic!("expected line entry, got {other:?}"),
        }
    }

    #[test]
    fn exact_entry_returns_smallest_containing_node() {
        let line = NodeId::new();
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                Rect::from_xywh(0.0, 0.0, 200.0, 80.0),
                vec![line_node(line, 40.0, 20.0, "hello", 10.0)],
            ),
        };
        let pages = [page(0.0, 100.0)];
        let layout_index = LayoutIndex::new(tree, &pages);
        let point = layout_index.point(0, 45.0, 25.0).unwrap();

        let entry = layout_index.exact_entry(point, |_, _| true).unwrap();

        assert_eq!(line_id(&layout_index, entry), line);
    }

    #[test]
    fn exact_entry_can_return_box_background_when_box_is_requested() {
        let box_id = NodeId::new();
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                Rect::from_xywh(0.0, 0.0, 200.0, 100.0),
                vec![box_node(
                    box_id,
                    Rect::from_xywh(0.0, 20.0, 200.0, 60.0),
                    vec![line_node(NodeId::new(), 40.0, 50.0, "hello", 10.0)],
                )],
            ),
        };
        let pages = [page(0.0, 120.0)];
        let layout_index = LayoutIndex::new(tree, &pages);
        let point = layout_index.point(0, 20.0, 25.0).unwrap();

        let entry = layout_index
            .exact_entry(point, |_, node| {
                matches!(node.content, LayoutContent::Box(_))
            })
            .unwrap();

        assert!(matches!(
            entry.content(&layout_index),
            Some(LayoutContent::Box(b)) if b.node_id == box_id
        ));
    }

    #[test]
    fn entry_for_position_resolves_attached_non_monolithic_box_edges() {
        let box_id = NodeId::new();
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
                vec![attached_box_node(
                    box_id,
                    NodeId::ROOT,
                    0,
                    Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
                    vec![line_node(NodeId::new(), 0.0, 0.0, "hello", 10.0)],
                )],
            ),
        };
        let pages = [page(0.0, 100.0)];
        let layout_index = LayoutIndex::new(tree, &pages);

        for pos in [
            Position {
                node_id: NodeId::ROOT,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: NodeId::ROOT,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        ] {
            let entry = layout_index.entry_for_position(&pos).unwrap();
            assert!(matches!(
                entry.content(&layout_index),
                Some(LayoutContent::Box(b)) if b.node_id == box_id
            ));
        }
    }

    #[test]
    fn closest_entry_prefers_flow_y_before_x() {
        let nearer_y = NodeId::new();
        let nearer_x = NodeId::new();
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                Rect::from_xywh(0.0, 0.0, 600.0, 400.0),
                vec![
                    line_node(nearer_y, 500.0, 100.0, "yy", 10.0),
                    line_node(nearer_x, 0.0, 300.0, "xx", 10.0),
                ],
            ),
        };
        let pages = [page(0.0, 400.0)];
        let layout_index = LayoutIndex::new(tree, &pages);
        let point = layout_index.point(0, 5.0, 130.0).unwrap();

        let entry = layout_index
            .closest_entry(point, |_, node| {
                matches!(node.content, LayoutContent::Line(_))
            })
            .unwrap();

        assert_eq!(line_id(&layout_index, entry), nearer_y);
    }

    #[test]
    fn closest_entry_stays_on_requested_page() {
        let page_0_line = NodeId::new();
        let page_1_line = NodeId::new();
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                Rect::from_xywh(0.0, 0.0, 200.0, 200.0),
                vec![
                    line_node(page_0_line, 0.0, 0.0, "first", 10.0),
                    line_node(page_1_line, 0.0, 100.0, "second", 10.0),
                ],
            ),
        };
        let pages = [page(0.0, 100.0), page(100.0, 200.0)];
        let layout_index = LayoutIndex::new(tree, &pages);
        let point = layout_index.point(0, 5.0, 95.0).unwrap();

        let entry = layout_index
            .closest_entry(point, |_, node| {
                matches!(node.content, LayoutContent::Line(_))
            })
            .unwrap();

        assert_eq!(line_id(&layout_index, entry), page_0_line);
    }

    #[test]
    fn closest_entry_includes_entry_spanning_requested_page() {
        let spanning_box = NodeId::new();
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                Rect::from_xywh(0.0, 0.0, 200.0, 200.0),
                vec![box_node(
                    spanning_box,
                    Rect::from_xywh(20.0, 50.0, 100.0, 100.0),
                    vec![line_node(NodeId::new(), 30.0, 60.0, "inside", 10.0)],
                )],
            ),
        };
        let pages = [page(0.0, 100.0), page(100.0, 200.0)];
        let layout_index = LayoutIndex::new(tree, &pages);
        let point = layout_index.point(1, 150.0, 20.0).unwrap();

        let entry = layout_index
            .closest_entry(point, |_, node| {
                matches!(&node.content, LayoutContent::Box(b) if b.node_id == spanning_box)
            })
            .unwrap();

        assert!(matches!(
            entry.content(&layout_index),
            Some(LayoutContent::Box(b)) if b.node_id == spanning_box
        ));
    }

    #[test]
    fn vertical_navigation_candidates_choose_row_before_x() {
        let (r0c0, r0c1, r1c0, r1c1) = (NodeId::new(), NodeId::new(), NodeId::new(), NodeId::new());
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                Rect::from_xywh(0.0, 0.0, 200.0, 40.0),
                vec![
                    box_node(
                        NodeId::new(),
                        Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
                        vec![
                            line_node(r0c0, 0.0, 0.0, "test", 25.0),
                            line_node(r0c1, 100.0, 0.0, "test", 25.0),
                        ],
                    ),
                    box_node(
                        NodeId::new(),
                        Rect::from_xywh(0.0, 20.0, 200.0, 20.0),
                        vec![
                            line_node(r1c0, 0.0, 20.0, "test", 25.0),
                            line_node(r1c1, 100.0, 20.0, "test", 25.0),
                        ],
                    ),
                ],
            ),
        };
        let pages = [page(0.0, 100.0)];
        let layout_index = LayoutIndex::new(tree, &pages);

        assert_eq!(
            line_id(
                &layout_index,
                crate::query::navigation::navigable_below_at_x(&layout_index, 20.0, 150.0).unwrap()
            ),
            r1c1
        );
        assert_eq!(
            line_id(
                &layout_index,
                crate::query::navigation::navigable_above_at_x(&layout_index, 20.0, 150.0).unwrap()
            ),
            r0c1
        );
        assert_eq!(
            line_id(
                &layout_index,
                crate::query::navigation::navigable_below_at_x(&layout_index, 20.0, -50.0).unwrap()
            ),
            r1c0
        );
    }

    #[test]
    fn entries_on_page_returns_visible_entries_in_layout_order() {
        let visible_1 = NodeId::new();
        let visible_2 = NodeId::new();
        let page_1_line = NodeId::new();
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                Rect::from_xywh(0.0, 0.0, 200.0, 200.0),
                vec![
                    line_node(visible_1, 0.0, 10.0, "a", 10.0),
                    line_node(visible_2, 0.0, 80.0, "b", 10.0),
                    line_node(page_1_line, 0.0, 110.0, "c", 10.0),
                ],
            ),
        };
        let pages = [page(0.0, 100.0), page(100.0, 200.0)];
        let layout_index = LayoutIndex::new(tree, &pages);

        let line_ids: Vec<_> = layout_index
            .entries_on_page(0)
            .into_iter()
            .filter_map(|entry| match entry.content(&layout_index) {
                Some(LayoutContent::Line(line)) => Some(line.node_id),
                _ => None,
            })
            .collect();

        assert_eq!(line_ids, vec![visible_1, visible_2]);
    }
}
