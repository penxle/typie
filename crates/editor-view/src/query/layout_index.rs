use editor_common::Rect;
use editor_crdt::Dot;
use editor_state::Affinity;
use editor_state::Position;
use rstar::{AABB, RTree, RTreeObject};
use std::collections::HashMap;

use crate::page::{LayoutPage, PageRect};
use crate::paginate::types::{LayoutContent, LayoutLine, LayoutNode, LayoutTree, SpacingKind};

type LayoutEntryId = usize;

#[derive(Debug)]
pub(crate) struct LayoutIndex {
    tree: LayoutTree,
    pages: Vec<LayoutPage>,
    entries: Vec<LayoutEntry>,
    boxes_by_node_id: HashMap<Dot, LayoutEntryId>,
    entries_by_node: HashMap<Dot, Vec<LayoutEntryId>>,
    spatial_entries: Vec<SpatialEntry>,
    entries_by_page: Vec<Vec<LayoutEntryId>>,
    rtree: std::sync::OnceLock<RTree<SpatialEntry>>,
}

#[derive(Debug, Clone)]
pub(crate) struct LayoutEntry {
    pub(crate) rect: Rect,
    path: Vec<usize>,
    ancestors: Vec<Dot>,
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
    boxes_by_node_id: HashMap<Dot, LayoutEntryId>,
    entries_by_node: HashMap<Dot, Vec<LayoutEntryId>>,
    ancestors: Vec<Dot>,
    path: Vec<usize>,
    spatial: Vec<SpatialEntry>,
    entries_by_page: Vec<Vec<LayoutEntryId>>,
}

impl LayoutIndex {
    pub(crate) fn new(tree: LayoutTree, pages: &[LayoutPage]) -> Self {
        let mut builder = LayoutIndexBuilder {
            pages,
            entries: Vec::new(),
            boxes_by_node_id: HashMap::new(),
            entries_by_node: HashMap::new(),
            ancestors: Vec::new(),
            path: Vec::new(),
            spatial: Vec::new(),
            entries_by_page: vec![Vec::new(); pages.len()],
        };
        builder.build_node(&tree.root);
        Self {
            tree,
            pages: pages.to_vec(),
            entries: builder.entries,
            boxes_by_node_id: builder.boxes_by_node_id,
            entries_by_node: builder.entries_by_node,
            spatial_entries: builder.spatial,
            entries_by_page: builder.entries_by_page,
            rtree: std::sync::OnceLock::new(),
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
        let candidate_ids = self.entries_by_node.get(&pos.node)?;
        let mut candidates = candidate_ids
            .iter()
            .map(|&id| &self.entries[id])
            .filter(|entry| {
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

    pub(crate) fn box_entry(&self, node: &Dot) -> Option<&LayoutEntry> {
        self.boxes_by_node_id
            .get(node)
            .map(|&entry_id| &self.entries[entry_id])
    }

    pub(crate) fn box_rect(&self, node: &Dot) -> Option<Rect> {
        Some(self.box_entry(node)?.rect)
    }

    pub(crate) fn box_page_rects(&self, ids: &[Dot]) -> Vec<PageRect> {
        let mut rects = Vec::new();
        for id in ids {
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

    pub(crate) fn nearest_box(&self, point: LayoutPoint, ids: &[Dot]) -> Option<Dot> {
        ids.iter()
            .filter_map(|id| {
                self.box_entry(id)
                    .map(|entry| (distance_key(&entry.rect, point.x, point.y), id))
            })
            .min_by(|a, b| compare_distance_key(a.0, b.0))
            .map(|(_, id)| *id)
    }

    pub(crate) fn box_contains(&self, point: LayoutPoint, node: &Dot) -> bool {
        self.box_entry(node)
            .is_some_and(|entry| entry.rect.contains(point.x, point.y))
    }

    pub(crate) fn entries_on_page(&self, page_idx: usize) -> Vec<&LayoutEntry> {
        match self.entries_by_page.get(page_idx) {
            Some(ids) => ids.iter().map(|&id| &self.entries[id]).collect(),
            None => Vec::new(),
        }
    }

    fn rtree(&self) -> &RTree<SpatialEntry> {
        self.rtree
            .get_or_init(|| RTree::bulk_load(self.spatial_entries.clone()))
    }

    pub(crate) fn entries(&self) -> std::slice::Iter<'_, LayoutEntry> {
        self.entries.iter()
    }

    pub(crate) fn direct_child_entries<'a>(
        &'a self,
        node: &'a Dot,
    ) -> impl Iterator<Item = &'a LayoutEntry> + 'a {
        self.entries
            .iter()
            .filter(move |entry| entry.ancestors.last() == Some(node))
    }

    pub(crate) fn direct_child_entries_in_y_range<'a>(
        &'a self,
        node: &'a Dot,
        y_start: f32,
        y_end: f32,
    ) -> impl Iterator<Item = &'a LayoutEntry> + 'a {
        self.direct_child_entries(node)
            .filter(move |entry| rect_overlaps_y_range(&entry.rect, y_start, y_end))
    }

    fn smallest_entry_at<T>(
        &self,
        point: LayoutPoint,
        map: impl Fn(&LayoutEntry, &LayoutNode) -> Option<T>,
    ) -> Option<(LayoutEntryId, T)> {
        let envelope = AABB::from_point([point.x, point.y]);
        self.rtree()
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

    pub(crate) fn ancestors(&self) -> &[Dot] {
        &self.ancestors
    }

    pub(crate) fn overlaps_y_range(&self, y_start: f32, y_end: f32) -> bool {
        rect_overlaps_y_range(&self.rect, y_start, y_end)
    }
}

impl LayoutIndexBuilder<'_> {
    fn build_node(&mut self, node: &LayoutNode) {
        match &node.content {
            LayoutContent::Box(b) => {
                let id = self.add_entry(node.rect);
                self.boxes_by_node_id.insert(b.node, id);
                if let Some(attachment) = &b.attachment {
                    self.register_match_node(attachment.parent, id);
                }
                self.ancestors.push(b.node);
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
            LayoutContent::Line(line) => {
                let id = self.add_entry(node.rect);
                self.register_match_node(line.node, id);
            }
            LayoutContent::Atom(atom) => {
                let id = self.add_entry(node.rect);
                self.register_match_node(atom.attachment.parent, id);
            }
        }
    }

    fn register_match_node(&mut self, node: Dot, entry_id: LayoutEntryId) {
        let entries = self.entries_by_node.entry(node).or_default();
        if entries.last() != Some(&entry_id) {
            entries.push(entry_id);
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
            let first = self.pages.partition_point(|p| p.y_end <= rect.y);
            for page_idx in first..self.pages.len() {
                let page = &self.pages[page_idx];
                if page.y_start >= rect.bottom() {
                    break;
                }
                if rect_overlaps_y_range(&rect, page.y_start, page.y_end) {
                    self.spatial.push(SpatialEntry {
                        entry_id,
                        page_idx,
                        bounds,
                    });
                    self.entries_by_page[page_idx].push(entry_id);
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
        LayoutContent::Box(b) => b
            .attachment
            .as_ref()
            .is_some_and(|a| position_attaches_to_child(pos, &a.parent, a.index)),
        LayoutContent::Line(line) => position_matches_line(line, pos),
        LayoutContent::Atom(atom) => {
            position_attaches_to_child(pos, &atom.attachment.parent, atom.attachment.index)
        }
        LayoutContent::Spacing(_) => false,
    }
}

fn position_matches_line(line: &LayoutLine, pos: &Position) -> bool {
    if line.node != pos.node {
        return false;
    }
    if let Some(range) = &line.offset_range
        && pos.offset >= range.start
        && pos.offset <= range.end
    {
        return true;
    }
    line.glyph_runs
        .iter()
        .any(|run| pos.offset >= run.offset_range.start && pos.offset <= run.offset_range.end)
}

fn position_attaches_to_child(pos: &Position, parent: &Dot, index: usize) -> bool {
    if pos.node != *parent {
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
    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AtomLeaf, DocLogs, DocView, HorizontalRuleVariant, ModifierAttrLog, NodeAttrLog,
        NodeMarkerLog, NodeStyleLog, NodeType, SeqItem, SpanLog, StyleLog, project_document,
    };
    use editor_resource::Resource;
    use editor_state::Affinity;

    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;
    use crate::paginate::paginator::Paginator;

    use super::*;

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_styles: NodeStyleLog::new(),
            node_markers: NodeMarkerLog::new(),
            styles: StyleLog::new(),
        }
    }

    fn build_index(doc: &DocLogs, width: f32) -> (Dot, LayoutIndex) {
        let pd = project_document(doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let root_id = root_node.id();
        let mut res = Resource::new_test();
        let measured = measure_node(&root_node, width, &MeasureContext::default(), &mut res);
        let layout = Paginator::continuous(width, 100_000.0, EdgeInsets::all(0.0))
            .paginate(MeasuredTree { root: measured });
        let index = LayoutIndex::new(layout.tree, &layout.pages);
        (root_id, index)
    }

    fn para_doc(text: &str, width: f32) -> (DocLogs, Dot, Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
            },
        )];
        for (i, ch) in text.chars().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), SeqItem::Char(ch)));
        }
        let doc = logs(&items);
        let para_id = para;
        let (root_id, index) = build_index(&doc, width);
        (doc, root_id, para_id, index)
    }

    fn all_lines_under_para<'a>(index: &'a LayoutIndex, para_id: &Dot) -> Vec<&'a LayoutNode> {
        let LayoutContent::Box(ref root_box) = index.tree().root.content else {
            panic!("root is not a box");
        };
        let para_box_node = root_box
            .children
            .iter()
            .find(|n| matches!(&n.content, LayoutContent::Box(b) if &b.node == para_id))
            .expect("para box not found");
        let LayoutContent::Box(ref para_box) = para_box_node.content else {
            panic!("para is not a box");
        };
        para_box
            .children
            .iter()
            .filter(|n| matches!(n.content, LayoutContent::Line(_)))
            .collect()
    }

    #[test]
    fn box_lookup() {
        let (_, root_id, para_id, index) = para_doc("Hi", 400.0);
        let rect = index.box_rect(&para_id);
        assert!(rect.is_some());
        let rect = rect.unwrap();
        assert!(rect.width > 0.0);
        assert!(rect.height > 0.0);
        assert!(index.box_entry(&root_id).is_some());
    }

    #[test]
    fn entry_for_position_line() {
        let (_, _root_id, para_id, index) = para_doc("Hi", 400.0);
        let pos = Position {
            node: para_id,
            offset: 1,
            affinity: Affinity::default(),
        };
        let entry = index.entry_for_position(&pos).expect("entry must exist");
        let node = entry.node(&index).expect("entry node must exist");
        assert!(matches!(&node.content, LayoutContent::Line(l) if l.node == para_id));
    }

    #[test]
    fn wrapped_interior_via_run_fallback() {
        let text = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let (_, _root_id, para_id, index) = para_doc(text, 40.0);

        let lines = all_lines_under_para(&index, &para_id);
        assert!(
            lines.len() >= 3,
            "must wrap into ≥3 lines, got {}",
            lines.len()
        );

        let interior_lines: Vec<_> = lines[1..lines.len() - 1].to_vec();
        for l in &interior_lines {
            let LayoutContent::Line(ref ll) = l.content else {
                panic!()
            };
            assert!(
                ll.offset_range.is_none(),
                "interior line must have offset_range == None"
            );
        }

        let interior_line = interior_lines[0];
        let LayoutContent::Line(ref interior_ll) = interior_line.content else {
            panic!()
        };
        let run = interior_ll
            .glyph_runs
            .first()
            .expect("interior line must have a glyph run");
        let interior_offset =
            run.offset_range.start + (run.offset_range.end - run.offset_range.start) / 2;

        let pos_interior = Position {
            node: para_id,
            offset: interior_offset,
            affinity: Affinity::default(),
        };
        let entry = index
            .entry_for_position(&pos_interior)
            .expect("interior position must resolve via run fallback");
        let node = entry.node(&index).expect("entry node must exist");
        assert!(matches!(&node.content, LayoutContent::Line(l) if l.node == para_id));
        assert!(
            std::ptr::eq(node as *const _, interior_line as *const _),
            "must resolve to the interior line node"
        );

        let first_line = lines[0];
        let LayoutContent::Line(ref first_ll) = first_line.content else {
            panic!()
        };
        let first_range = first_ll
            .offset_range
            .as_ref()
            .expect("first line must have offset_range");
        let pos_boundary = Position {
            node: para_id,
            offset: first_range.start,
            affinity: Affinity::Upstream,
        };
        let boundary_entry = index
            .entry_for_position(&pos_boundary)
            .expect("boundary position must resolve");
        let boundary_node = boundary_entry
            .node(&index)
            .expect("boundary entry node must exist");
        assert!(matches!(&boundary_node.content, LayoutContent::Line(l) if l.node == para_id));
        assert!(
            std::ptr::eq(boundary_node as *const _, first_line as *const _),
            "boundary position (offset == first_range.start, Upstream) must resolve to the FIRST line node \
             via the line-level offset_range branch, not to any other line"
        );
    }

    #[test]
    fn point_hit_test() {
        let (_, _root_id, para_id, index) = para_doc("Hi", 400.0);
        let para_rect = index.box_rect(&para_id).unwrap();
        let mid_x = para_rect.x + para_rect.width / 2.0;
        let mid_y = para_rect.y + para_rect.height / 2.0;

        let pt = index
            .point(0, mid_x, mid_y)
            .expect("point must be on page 0");

        let closest = index
            .closest_entry(pt, |_, node| matches!(node.content, LayoutContent::Line(_)))
            .expect("closest entry must exist");
        assert!(matches!(
            closest.node(&index).map(|n| &n.content),
            Some(LayoutContent::Line(l)) if l.node == para_id
        ));

        let exact = index
            .exact_entry(pt, |_, node| matches!(node.content, LayoutContent::Line(_)))
            .expect("exact entry must exist");
        assert!(matches!(
            exact.node(&index).map(|n| &n.content),
            Some(LayoutContent::Line(l)) if l.node == para_id
        ));
    }

    #[test]
    fn atom_attachment_position() {
        let root = Dot::ROOT;
        let hr = Dot::new(1, 1);
        let p = Dot::new(1, 2);
        let items = vec![
            (
                hr,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HorizontalRule {
                        variant: HorizontalRuleVariant::default(),
                    },
                    parents: vec![root],
                },
            ),
            (
                p,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let doc = logs(&items);
        let (root_id, index) = build_index(&doc, 400.0);

        let pos = Position {
            node: root_id,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let entry = index
            .entry_for_position(&pos)
            .expect("atom attachment position must resolve");
        let node = entry.node(&index).expect("atom entry node must exist");
        assert!(
            matches!(&node.content, LayoutContent::Atom(a) if a.attachment.parent == root_id && a.attachment.index == 0),
            "entry must be the HR atom with parent root_id at index 0"
        );
    }

    fn two_para_doc(text1: &str, text2: &str, width: f32) -> (Dot, Dot, Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let para1 = Dot::new(1, 1);
        let para2 = Dot::new(1, 2);
        let mut items = vec![(
            para1,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
            },
        )];
        let base = 3u64;
        for (i, ch) in text1.chars().enumerate() {
            items.push((Dot::new(1, base + i as u64), SeqItem::Char(ch)));
        }
        let base2 = base + text1.len() as u64;
        items.push((
            para2,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
            },
        ));
        for (i, ch) in text2.chars().enumerate() {
            items.push((Dot::new(1, base2 + 1 + i as u64), SeqItem::Char(ch)));
        }
        let doc = logs(&items);
        let para1_id = para1;
        let para2_id = para2;
        let (root_id, index) = build_index(&doc, width);
        (root_id, para1_id, para2_id, index)
    }

    #[test]
    fn direct_children_and_nearest() {
        let (root_id, para1_id, para2_id, index) = two_para_doc("near", "far", 400.0);

        let direct: Vec<_> = index.direct_child_entries(&root_id).collect();
        assert!(!direct.is_empty(), "root must have direct child entries");
        for entry in &direct {
            assert_eq!(
                entry.ancestors().last(),
                Some(&root_id),
                "direct child entry must have root_id as last ancestor"
            );
        }

        assert!(
            direct
                .iter()
                .all(|entry| entry.ancestors().last() != Some(&para1_id)),
            "direct_child_entries must EXCLUDE entries whose last ancestor is para1_id (i.e. no deeper line entries should leak through)"
        );

        let para1_rect = index.box_rect(&para1_id).unwrap();
        let para2_rect = index.box_rect(&para2_id).unwrap();
        let mid_x = para1_rect.x + para1_rect.width / 2.0;
        let mid_y = para1_rect.y + para1_rect.height / 2.0;
        let pt = index.point(0, mid_x, mid_y).unwrap();

        let nearest = index.nearest_box(pt, &[para1_id, para2_id]);
        assert_eq!(
            nearest,
            Some(para1_id),
            "nearest_box must return para1_id (nearer block) not para2_id (farther block at y={})",
            para2_rect.y
        );

        let page_rects = index.box_page_rects(&[para1_id]);
        assert!(!page_rects.is_empty(), "box_page_rects must be non-empty");
    }
}
