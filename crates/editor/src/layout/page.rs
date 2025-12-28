use crate::layout::cursor::NavigationContext;
use crate::layout::elements::external::ExternalElement;
use crate::layout::interactive::InteractionKind;
use crate::layout::{Element, PositionedNode};
use crate::state::Position;
use crate::types::{Point, PointerStyle, Size};
use rstar::{AABB, PointDistance, RTree, RTreeObject};

pub struct ElementEntry {
    pub pos: Point,
    pub size: Size,
    element: *const Element,
    bounds: AABB<[f32; 2]>,
}

impl ElementEntry {
    fn new(pos: Point, size: Size, element: *const Element) -> Self {
        Self {
            pos,
            size,
            element,
            bounds: AABB::from_corners([pos.x, pos.y], [pos.x + size.width, pos.y + size.height]),
        }
    }

    pub fn element(&self) -> &Element {
        unsafe { &*self.element }
    }
}

impl RTreeObject for ElementEntry {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.bounds
    }
}

impl PointDistance for ElementEntry {
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        let x = point[0];
        let y = point[1];

        if y >= self.pos.y && y < self.pos.y + self.size.height {
            if x >= self.pos.x && x < self.pos.x + self.size.width {
                return 0.0;
            }

            return if x < self.pos.x {
                (self.pos.x - x).powi(2)
            } else {
                (x - (self.pos.x + self.size.width)).powi(2)
            };
        }

        let closest_x = x.clamp(self.pos.x, self.pos.x + self.size.width);
        let closest_y = y.clamp(self.pos.y, self.pos.y + self.size.height);

        (x - closest_x).powi(2) + (y - closest_y).powi(2)
    }
}

pub struct Page {
    pub root: PositionedNode,
    spatial_index: RTree<ElementEntry>,
}

impl Page {
    pub fn from_root(root: PositionedNode) -> Self {
        let mut elements = Vec::new();
        Self::collect_navigable_from_tree(&root, Point::zero(), &mut elements);

        let spatial_index = Self::build_spatial_index(elements);

        Self {
            root,
            spatial_index,
        }
    }

    fn build_spatial_index(elements: Vec<(Point, &Element)>) -> RTree<ElementEntry> {
        let entries: Vec<_> = elements
            .into_iter()
            .map(|(pos, elem)| ElementEntry::new(pos, elem.size(), elem as *const Element))
            .collect();
        RTree::bulk_load(entries)
    }

    pub fn spatial_index(&self) -> &RTree<ElementEntry> {
        &self.spatial_index
    }

    pub fn first_element(&self) -> Option<(Point, &Element)> {
        self.spatial_index
            .iter()
            .min_by(|a, b| a.pos.y.total_cmp(&b.pos.y))
            .map(|entry| (entry.pos, entry.element()))
    }

    pub fn last_element(&self) -> Option<(Point, &Element)> {
        self.spatial_index
            .iter()
            .max_by(|a, b| {
                let a_bottom = a.pos.y + a.size.height;
                let b_bottom = b.pos.y + b.size.height;
                a_bottom.total_cmp(&b_bottom)
            })
            .map(|entry| (entry.pos, entry.element()))
    }

    pub fn find_element_at_point(&self, point: Point) -> Option<(Point, &Element)> {
        self.spatial_index
            .locate_at_point(&[point.x, point.y])
            .map(|entry| (entry.pos, entry.element()))
    }

    pub fn find_element_at_position<'a>(
        &'a self,
        ctx: &NavigationContext,
        position: &Position,
    ) -> Option<(Point, &'a Element)> {
        Self::find_at_position(ctx, &self.root, Point::zero(), position)
    }

    fn find_at_position<'a>(
        ctx: &NavigationContext,
        positioned: &'a PositionedNode,
        offset: Point,
        position: &Position,
    ) -> Option<(Point, &'a Element)> {
        let pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        if let Some(children) = &positioned.node.children {
            for child in children {
                if let Some(result) = Self::find_at_position(ctx, child, pos, position) {
                    return Some(result);
                }
            }
        }

        if let Some(ref element) = positioned.node.element {
            if let Some(navigable) = element.as_cursor_navigable() {
                if navigable.cursor_bounds(ctx, position).is_some() {
                    return Some((pos, element));
                }
            }
        }

        None
    }

    #[allow(dead_code)]
    pub fn external_elements(&self) -> Vec<(Point, &ExternalElement)> {
        let mut result = Vec::new();
        Self::collect_external_from_tree(&self.root, Point::zero(), &mut result);
        result
    }

    fn collect_external_from_tree<'a>(
        positioned: &'a PositionedNode,
        offset: Point,
        result: &mut Vec<(Point, &'a ExternalElement)>,
    ) {
        let abs_pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        if let Some(ref element) = positioned.node.element {
            if let Element::External(ext) = element {
                result.push((abs_pos, ext));
            }
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::collect_external_from_tree(child, abs_pos, result);
            }
        }
    }

    fn collect_navigable_from_tree<'a>(
        positioned: &'a PositionedNode,
        offset: Point,
        result: &mut Vec<(Point, &'a Element)>,
    ) {
        let abs_pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        if let Some(ref element) = positioned.node.element {
            if element.as_cursor_navigable().is_some() {
                result.push((abs_pos, element));
            }
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::collect_navigable_from_tree(child, abs_pos, result);
            }
        }
    }

    pub fn get_pointer_style(&self, x: f32, y: f32) -> PointerStyle {
        if let Some(entry) = self.spatial_index.locate_at_point(&[x, y]) {
            return entry.element().cursor_visual();
        }

        if Self::traverse_for_interactive(&self.root, Point::zero(), x, y).is_some() {
            return PointerStyle::Pointer;
        }

        Self::traverse_for_pointer_style(&self.root, Point::zero(), x, y)
            .unwrap_or(PointerStyle::Text)
    }

    fn traverse_for_pointer_style(
        positioned: &PositionedNode,
        offset: Point,
        x: f32,
        y: f32,
    ) -> Option<PointerStyle> {
        let abs_x = offset.x + positioned.position.x;
        let abs_y = offset.y + positioned.position.y;
        let node = &positioned.node;

        if x >= abs_x && x <= abs_x + node.size.width && y >= abs_y && y <= abs_y + node.size.height
        {
            if let Some(ref element) = node.element {
                return Some(element.cursor_visual());
            }

            if let Some(children) = &node.children {
                for child in children {
                    if let Some(cursor) =
                        Self::traverse_for_pointer_style(child, Point::new(abs_x, abs_y), x, y)
                    {
                        return Some(cursor);
                    }
                }
            }
        }

        None
    }

    pub fn find_interactive_at(&self, x: f32, y: f32) -> Option<InteractionKind> {
        if self.is_over_cursor_navigable(x, y) {
            return None;
        }
        Self::traverse_for_interactive(&self.root, Point::zero(), x, y)
    }

    fn is_over_cursor_navigable(&self, x: f32, y: f32) -> bool {
        self.spatial_index.locate_at_point(&[x, y]).is_some()
    }

    fn traverse_for_interactive(
        positioned: &PositionedNode,
        offset: Point,
        x: f32,
        y: f32,
    ) -> Option<InteractionKind> {
        let abs_x = offset.x + positioned.position.x;
        let abs_y = offset.y + positioned.position.y;
        let node = &positioned.node;

        if x >= abs_x && x <= abs_x + node.size.width && y >= abs_y && y <= abs_y + node.size.height
        {
            if let Some(ref element) = node.element {
                if let Some(interactive) = element.as_interactive() {
                    return Some(interactive.interaction_kind());
                }
            }

            if let Some(children) = &node.children {
                for child in children {
                    if let Some(kind) =
                        Self::traverse_for_interactive(child, Point::new(abs_x, abs_y), x, y)
                    {
                        return Some(kind);
                    }
                }
            }
        }

        None
    }

    pub fn get_text_range_bounds(
        &self,
        block_id: crate::model::NodeId,
        start_offset: usize,
        end_offset: usize,
    ) -> Vec<crate::types::Rect> {
        let mut rects = Vec::new();

        Self::collect_text_range_bounds(
            &self.root,
            Point::zero(),
            block_id,
            start_offset,
            end_offset,
            &mut rects,
        );

        rects
    }

    fn collect_text_range_bounds(
        positioned: &PositionedNode,
        offset: Point,
        block_id: crate::model::NodeId,
        start_offset: usize,
        end_offset: usize,
        rects: &mut Vec<crate::types::Rect>,
    ) {
        let abs_pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        if let Some(ref element) = positioned.node.element {
            if let Element::Line(line) = element {
                if line.block_id == block_id {
                    let line_start = line.metric.start_offset;
                    let line_end = line.metric.end_offset;

                    if start_offset < line_end && end_offset > line_start {
                        let range_start = start_offset.max(line_start);
                        let range_end = end_offset.min(line_end);

                        let start_x = line.offset_to_x(range_start);
                        let end_x = line.offset_to_x(range_end);

                        let width = end_x - start_x;
                        if width > 0.0 {
                            rects.push(crate::types::Rect {
                                x: abs_pos.x + start_x,
                                y: abs_pos.y + line.metric.top,
                                width,
                                height: line.metric.height + line.metric.leading,
                            });
                        }
                    }
                }
            }
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::collect_text_range_bounds(
                    child,
                    abs_pos,
                    block_id,
                    start_offset,
                    end_offset,
                    rects,
                );
            }
        }
    }

    pub fn get_link_overlays(
        &self,
        link_ranges: &[crate::model::LinkRange],
    ) -> Vec<(String, Vec<crate::types::Rect>)> {
        let mut results = Vec::new();

        for range in link_ranges {
            let bounds =
                self.get_text_range_bounds(range.block_id, range.start_offset, range.end_offset);

            if !bounds.is_empty() {
                results.push((range.href.clone(), bounds));
            }
        }

        results
    }
}
