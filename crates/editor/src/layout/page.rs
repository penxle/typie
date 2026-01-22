use crate::layout::cursor::NavigationContext;
use crate::layout::elements::external::ExternalElement;
use crate::layout::interactive::InteractionKind;
use crate::layout::{Element, PositionedNode};
use crate::model::NodeId;
use crate::state::Position;
use crate::types::{Point, PointerStyle, Size};
use rstar::{AABB, PointDistance, RTree, RTreeObject};

#[derive(Clone)]
pub struct ScopeEntry {
    pub pos: Point,
    pub size: Size,
    pub scope_id: NodeId,
    bounds: AABB<[f32; 2]>,
}

impl ScopeEntry {
    fn new(pos: Point, size: Size, scope_id: NodeId) -> Self {
        Self {
            pos,
            size,
            scope_id,
            bounds: AABB::from_corners([pos.x, pos.y], [pos.x + size.width, pos.y + size.height]),
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.pos.x
            && x < self.pos.x + self.size.width
            && y >= self.pos.y
            && y < self.pos.y + self.size.height
    }

    pub fn is_container(&self) -> bool {
        self.scope_id != NodeId::ROOT
    }
}

impl RTreeObject for ScopeEntry {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.bounds
    }
}

impl PointDistance for ScopeEntry {
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        let x = point[0];
        let y = point[1];
        let closest_x = x.clamp(self.pos.x, self.pos.x + self.size.width);
        let closest_y = y.clamp(self.pos.y, self.pos.y + self.size.height);
        (x - closest_x).powi(2) + (y - closest_y).powi(2)
    }
}

pub struct ElementEntry {
    pub pos: Point,
    pub size: Size,
    element: *const Element,
    bounds: AABB<[f32; 2]>,
    pub scope_id: NodeId,
}

impl ElementEntry {
    fn new(pos: Point, size: Size, element: *const Element, scope_id: NodeId) -> Self {
        Self {
            pos,
            size,
            element,
            bounds: AABB::from_corners([pos.x, pos.y], [pos.x + size.width, pos.y + size.height]),
            scope_id,
        }
    }

    pub fn element(&self) -> &Element {
        // SAFETY: Page가 root와 elements를 모두 소유하므로 포인터는 Page 수명 동안 유효함.
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
    elements: RTree<ElementEntry>,
    scopes: RTree<ScopeEntry>,
}

impl Page {
    pub fn from_root(root: PositionedNode) -> Self {
        let mut elements = Vec::new();
        let mut scopes = Vec::new();

        let doc_scope = NodeId::ROOT;

        Self::collect_elements_and_scopes(
            &root,
            Point::zero(),
            doc_scope,
            &mut elements,
            &mut scopes,
        );

        let elements_tree = RTree::bulk_load(elements);
        let scopes_tree = RTree::bulk_load(scopes);

        Self {
            root,
            elements: elements_tree,
            scopes: scopes_tree,
        }
    }

    pub fn scope_at(&self, x: f32, y: f32) -> Option<&ScopeEntry> {
        self.scopes.locate_all_at_point(&[x, y]).min_by(|a, b| {
            let a_area = a.size.width * a.size.height;
            let b_area = b.size.width * b.size.height;
            a_area.partial_cmp(&b_area).unwrap()
        })
    }
    pub fn scope_entry(&self, scope_id: NodeId) -> Option<&ScopeEntry> {
        self.scopes.iter().find(|s| s.scope_id == scope_id)
    }

    pub fn elements_in_scope(&self, scope_id: NodeId) -> impl Iterator<Item = &ElementEntry> {
        self.elements.iter().filter(move |e| e.scope_id == scope_id)
    }

    pub fn first_in_scope(&self, scope_id: NodeId) -> Option<&ElementEntry> {
        self.elements_in_scope(scope_id)
            .min_by(|a, b| a.pos.y.partial_cmp(&b.pos.y).unwrap())
    }

    pub fn last_in_scope(&self, scope_id: NodeId) -> Option<&ElementEntry> {
        self.elements_in_scope(scope_id).max_by(|a, b| {
            let a_bottom = a.pos.y + a.size.height;
            let b_bottom = b.pos.y + b.size.height;
            a_bottom.partial_cmp(&b_bottom).unwrap()
        })
    }

    pub fn find_above_in_scope(
        &self,
        x: f32,
        y: f32,
        scope_id: NodeId,
        exclude_element: Option<*const Element>,
    ) -> Option<&ElementEntry> {
        self.find_above_impl(x, y, Some(scope_id), None, exclude_element)
    }

    pub fn find_below_in_scope(
        &self,
        x: f32,
        y: f32,
        scope_id: NodeId,
        exclude_element: Option<*const Element>,
    ) -> Option<&ElementEntry> {
        self.find_below_impl(x, y, Some(scope_id), None, exclude_element)
    }

    pub fn find_above(
        &self,
        x: f32,
        y: f32,
        exclude_scope: Option<NodeId>,
    ) -> Option<&ElementEntry> {
        self.find_above_impl(x, y, None, exclude_scope, None)
    }

    pub fn find_below(
        &self,
        x: f32,
        y: f32,
        exclude_scope: Option<NodeId>,
    ) -> Option<&ElementEntry> {
        self.find_below_impl(x, y, None, exclude_scope, None)
    }

    pub fn find_target_above(
        &self,
        x: f32,
        y: f32,
        exclude_scope: Option<NodeId>,
    ) -> Option<&ElementEntry> {
        const EPSILON: f32 = 0.5;

        let candidate_scopes: Vec<_> = self
            .scopes
            .iter()
            .filter(|s| {
                let s_bottom = s.pos.y + s.size.height;
                let excluded = exclude_scope.is_some_and(|id| s.scope_id == id);
                !excluded && s_bottom <= y + EPSILON
            })
            .collect();

        let candidate_scope_ids: std::collections::HashSet<_> =
            candidate_scopes.iter().map(|s| s.scope_id).collect();

        let scopeless_elements: Vec<_> = self
            .elements
            .iter()
            .filter(|e| {
                let is_scopeless = e.scope_id == NodeId::ROOT;
                let e_bottom = e.pos.y + e.size.height;
                let excluded = exclude_scope.is_some_and(|id| e.scope_id == id);
                let in_candidate_scope = candidate_scope_ids.contains(&e.scope_id);
                is_scopeless && !excluded && !in_candidate_scope && e_bottom <= y + EPSILON
            })
            .collect();

        let score_scope = |s: &ScopeEntry| {
            let s_bottom = s.pos.y + s.size.height;
            let dy = (y - s_bottom).max(0.0);
            let x_left = s.pos.x;
            let x_right = s.pos.x + s.size.width;
            let dx = if x < x_left {
                x_left - x
            } else if x > x_right {
                x - x_right
            } else {
                0.0
            };
            dy * 10000.0 + dx
        };

        let score_element = |e: &ElementEntry| {
            let e_bottom = e.pos.y + e.size.height;
            let dy = (y - e_bottom).max(0.0);
            let x_left = e.pos.x;
            let x_right = e.pos.x + e.size.width;
            let dx = if x < x_left {
                x_left - x
            } else if x > x_right {
                x - x_right
            } else {
                0.0
            };
            dy * 10000.0 + dx
        };

        let best_scope = candidate_scopes
            .iter()
            .min_by(|a, b| score_scope(a).partial_cmp(&score_scope(b)).unwrap());
        let best_scopeless = scopeless_elements
            .iter()
            .min_by(|a, b| score_element(a).partial_cmp(&score_element(b)).unwrap());

        match (best_scope, best_scopeless) {
            (Some(scope), Some(element)) => {
                let scope_score = score_scope(scope);
                let element_score = score_element(element);
                if scope_score <= element_score {
                    self.find_above_in_scope(x, y, scope.scope_id, None)
                } else {
                    Some(*element)
                }
            }
            (Some(scope), None) => self.find_above_in_scope(x, y, scope.scope_id, None),
            (None, Some(element)) => Some(*element),
            (None, None) => None,
        }
    }

    pub fn find_target_below(
        &self,
        x: f32,
        y: f32,
        exclude_scope: Option<NodeId>,
    ) -> Option<&ElementEntry> {
        const EPSILON: f32 = 0.5;

        let candidate_scopes: Vec<_> = self
            .scopes
            .iter()
            .filter(|s| {
                let excluded = exclude_scope.is_some_and(|id| s.scope_id == id);
                !excluded && s.pos.y >= y - EPSILON
            })
            .collect();

        let candidate_scope_ids: std::collections::HashSet<_> =
            candidate_scopes.iter().map(|s| s.scope_id).collect();

        let scopeless_elements: Vec<_> = self
            .elements
            .iter()
            .filter(|e| {
                let is_scopeless = e.scope_id == NodeId::ROOT;
                let excluded = exclude_scope.is_some_and(|id| e.scope_id == id);
                let in_candidate_scope = candidate_scope_ids.contains(&e.scope_id);
                is_scopeless && !excluded && !in_candidate_scope && e.pos.y >= y - EPSILON
            })
            .collect();

        let score_scope = |s: &ScopeEntry| {
            let dy = (s.pos.y - y).max(0.0);
            let x_left = s.pos.x;
            let x_right = s.pos.x + s.size.width;
            let dx = if x < x_left {
                x_left - x
            } else if x > x_right {
                x - x_right
            } else {
                0.0
            };
            dy * 10000.0 + dx
        };

        let score_element = |e: &ElementEntry| {
            let dy = (e.pos.y - y).max(0.0);
            let x_left = e.pos.x;
            let x_right = e.pos.x + e.size.width;
            let dx = if x < x_left {
                x_left - x
            } else if x > x_right {
                x - x_right
            } else {
                0.0
            };
            dy * 10000.0 + dx
        };

        let best_scope = candidate_scopes
            .iter()
            .min_by(|a, b| score_scope(a).partial_cmp(&score_scope(b)).unwrap());
        let best_scopeless = scopeless_elements
            .iter()
            .min_by(|a, b| score_element(a).partial_cmp(&score_element(b)).unwrap());

        match (best_scope, best_scopeless) {
            (Some(scope), Some(element)) => {
                let scope_score = score_scope(scope);
                let element_score = score_element(element);
                if scope_score <= element_score {
                    self.find_below_in_scope(x, y, scope.scope_id, None)
                } else {
                    Some(*element)
                }
            }
            (Some(scope), None) => self.find_below_in_scope(x, y, scope.scope_id, None),
            (None, Some(element)) => Some(*element),
            (None, None) => None,
        }
    }

    pub fn find_target_left(
        &self,
        x: f32,
        y: f32,
        exclude_scope: Option<NodeId>,
    ) -> Option<&ElementEntry> {
        const EPSILON: f32 = 0.5;

        let candidate_scopes: Vec<_> = self
            .scopes
            .iter()
            .filter(|s| {
                let s_right = s.pos.x + s.size.width;
                let excluded = exclude_scope.is_some_and(|id| s.scope_id == id);
                !excluded && s_right <= x + EPSILON
            })
            .collect();

        let candidate_scope_ids: std::collections::HashSet<_> =
            candidate_scopes.iter().map(|s| s.scope_id).collect();

        let scopeless_elements: Vec<_> = self
            .elements
            .iter()
            .filter(|e| {
                let e_right = e.pos.x + e.size.width;
                let in_candidate_scope = candidate_scope_ids.contains(&e.scope_id);
                let excluded = exclude_scope.is_some_and(|id| e.scope_id == id);
                !excluded && !in_candidate_scope && e_right <= x + EPSILON
            })
            .collect();

        let score_scope = |s: &ScopeEntry| {
            let s_right = s.pos.x + s.size.width;
            let dx = (x - s_right).max(0.0);
            let y_top = s.pos.y;
            let y_bottom = s.pos.y + s.size.height;
            let dy = if y < y_top {
                y_top - y
            } else if y > y_bottom {
                y - y_bottom
            } else {
                0.0
            };
            dx * 10000.0 + dy
        };

        let score_element = |e: &ElementEntry| {
            let e_right = e.pos.x + e.size.width;
            let dx = (x - e_right).max(0.0);
            let y_top = e.pos.y;
            let y_bottom = e.pos.y + e.size.height;
            let dy = if y < y_top {
                y_top - y
            } else if y > y_bottom {
                y - y_bottom
            } else {
                0.0
            };
            dx * 10000.0 + dy
        };

        let best_scope = candidate_scopes
            .iter()
            .min_by(|a, b| score_scope(a).partial_cmp(&score_scope(b)).unwrap());
        let best_scopeless = scopeless_elements
            .iter()
            .min_by(|a, b| score_element(a).partial_cmp(&score_element(b)).unwrap());

        match (best_scope, best_scopeless) {
            (Some(scope), Some(element)) => {
                let scope_score = score_scope(scope);
                let element_score = score_element(element);
                if scope_score <= element_score {
                    self.last_in_scope(scope.scope_id)
                } else {
                    Some(*element)
                }
            }
            (Some(scope), None) => self.last_in_scope(scope.scope_id),
            (None, Some(element)) => Some(*element),
            (None, None) => None,
        }
    }

    pub fn find_target_right(
        &self,
        x: f32,
        y: f32,
        exclude_scope: Option<NodeId>,
    ) -> Option<&ElementEntry> {
        const EPSILON: f32 = 0.5;

        let candidate_scopes: Vec<_> = self
            .scopes
            .iter()
            .filter(|s| {
                let excluded = exclude_scope.is_some_and(|id| s.scope_id == id);
                !excluded && s.pos.x >= x - EPSILON
            })
            .collect();

        let candidate_scope_ids: std::collections::HashSet<_> =
            candidate_scopes.iter().map(|s| s.scope_id).collect();

        let scopeless_elements: Vec<_> = self
            .elements
            .iter()
            .filter(|e| {
                let in_candidate_scope = candidate_scope_ids.contains(&e.scope_id);
                let excluded = exclude_scope.is_some_and(|id| e.scope_id == id);
                !excluded && !in_candidate_scope && e.pos.x >= x - EPSILON
            })
            .collect();

        let score_scope = |s: &ScopeEntry| {
            let dx = (s.pos.x - x).max(0.0);
            let y_top = s.pos.y;
            let y_bottom = s.pos.y + s.size.height;
            let dy = if y < y_top {
                y_top - y
            } else if y > y_bottom {
                y - y_bottom
            } else {
                0.0
            };
            dx * 10000.0 + dy
        };

        let score_element = |e: &ElementEntry| {
            let dx = (e.pos.x - x).max(0.0);
            let y_top = e.pos.y;
            let y_bottom = e.pos.y + e.size.height;
            let dy = if y < y_top {
                y_top - y
            } else if y > y_bottom {
                y - y_bottom
            } else {
                0.0
            };
            dx * 10000.0 + dy
        };

        let best_scope = candidate_scopes
            .iter()
            .min_by(|a, b| score_scope(a).partial_cmp(&score_scope(b)).unwrap());
        let best_scopeless = scopeless_elements
            .iter()
            .min_by(|a, b| score_element(a).partial_cmp(&score_element(b)).unwrap());

        match (best_scope, best_scopeless) {
            (Some(scope), Some(element)) => {
                let scope_score = score_scope(scope);
                let element_score = score_element(element);
                if scope_score <= element_score {
                    self.first_in_scope(scope.scope_id)
                } else {
                    Some(*element)
                }
            }
            (Some(scope), None) => self.first_in_scope(scope.scope_id),
            (None, Some(element)) => Some(*element),
            (None, None) => None,
        }
    }

    fn find_above_impl(
        &self,
        x: f32,
        y: f32,
        scope_filter: Option<NodeId>,
        exclude_scope: Option<NodeId>,
        exclude_element: Option<*const Element>,
    ) -> Option<&ElementEntry> {
        const EPSILON: f32 = 0.5;

        let score_fn = |e: &ElementEntry| {
            let e_bottom = e.pos.y + e.size.height;
            let dy = (y - e_bottom).max(0.0);
            let x_left = e.pos.x;
            let x_right = e.pos.x + e.size.width;
            let dx = if x < x_left {
                x_left - x
            } else if x > x_right {
                x - x_right
            } else {
                0.0
            };
            dy * 10000.0 + dx
        };

        self.elements
            .iter()
            .filter(|e| {
                let scope_match = scope_filter.is_none_or(|id| e.scope_id == id);
                let exclude_match = exclude_scope.is_none_or(|id| e.scope_id != id);
                let node_match = exclude_element.is_none_or(|ptr| e.element != ptr);
                let e_bottom = e.pos.y + e.size.height;
                scope_match && exclude_match && node_match && e_bottom <= y + EPSILON
            })
            .min_by(|a, b| score_fn(a).partial_cmp(&score_fn(b)).unwrap())
    }

    fn find_below_impl(
        &self,
        x: f32,
        y: f32,
        scope_filter: Option<NodeId>,
        exclude_scope: Option<NodeId>,
        exclude_element: Option<*const Element>,
    ) -> Option<&ElementEntry> {
        const EPSILON: f32 = 0.5;

        let score_fn = |e: &ElementEntry| {
            let dy = (e.pos.y - y).max(0.0);
            let x_left = e.pos.x;
            let x_right = e.pos.x + e.size.width;
            let dx = if x < x_left {
                x_left - x
            } else if x > x_right {
                x - x_right
            } else {
                0.0
            };
            dy * 10000.0 + dx
        };

        self.elements
            .iter()
            .filter(|e| {
                let scope_match = scope_filter.is_none_or(|id| e.scope_id == id);
                let exclude_match = exclude_scope.is_none_or(|id| e.scope_id != id);
                let node_match = exclude_element.is_none_or(|ptr| e.element != ptr);
                scope_match && exclude_match && node_match && e.pos.y >= y - EPSILON
            })
            .min_by(|a, b| score_fn(a).partial_cmp(&score_fn(b)).unwrap())
    }

    fn collect_elements_and_scopes<'a>(
        positioned: &'a PositionedNode,
        offset: Point,
        current_scope: NodeId,
        elements: &mut Vec<ElementEntry>,
        scopes: &mut Vec<ScopeEntry>,
    ) {
        let abs_pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        let is_navigable = positioned
            .node
            .element
            .as_ref()
            .map(|e| e.as_cursor_navigable().is_some())
            .unwrap_or(false);

        let new_scope = if let Some(scope_id) = positioned.node.scope_id {
            if !is_navigable {
                scopes.push(ScopeEntry::new(abs_pos, positioned.node.size, scope_id));
            }
            scope_id
        } else {
            current_scope
        };

        if let Some(ref element) = positioned.node.element {
            if element.as_cursor_navigable().is_some() {
                elements.push(ElementEntry::new(
                    abs_pos,
                    element.size(),
                    element as *const Element,
                    new_scope,
                ));
            }
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::collect_elements_and_scopes(child, abs_pos, new_scope, elements, scopes);
            }
        }
    }

    pub fn spatial_index(&self) -> &RTree<ElementEntry> {
        &self.elements
    }

    pub fn first_element(&self) -> Option<(Point, &Element)> {
        self.spatial_index()
            .iter()
            .min_by(|a, b| a.pos.y.total_cmp(&b.pos.y))
            .map(|entry| (entry.pos, entry.element()))
    }

    pub fn last_element(&self) -> Option<(Point, &Element)> {
        self.spatial_index()
            .iter()
            .max_by(|a, b| {
                let a_bottom = a.pos.y + a.size.height;
                let b_bottom = b.pos.y + b.size.height;
                a_bottom.total_cmp(&b_bottom)
            })
            .map(|entry| (entry.pos, entry.element()))
    }

    pub fn find_element_at_point(&self, point: Point) -> Option<(Point, &Element)> {
        self.spatial_index()
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

    pub fn get_pointer_style(&self, x: f32, y: f32) -> PointerStyle {
        if let Some(entry) = self.spatial_index().locate_at_point(&[x, y]) {
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
        self.spatial_index().locate_at_point(&[x, y]).is_some()
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
    ) -> Vec<crate::types::TextBound> {
        let mut bounds = Vec::new();

        Self::collect_text_range_bounds(
            &self.root,
            Point::zero(),
            block_id,
            start_offset,
            end_offset,
            &mut bounds,
        );

        bounds
    }

    fn collect_text_range_bounds(
        positioned: &PositionedNode,
        offset: Point,
        block_id: crate::model::NodeId,
        start_offset: usize,
        end_offset: usize,
        bounds: &mut Vec<crate::types::TextBound>,
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
                            bounds.push(crate::types::TextBound {
                                x: abs_pos.x + start_x,
                                y: abs_pos.y + line.metric.top,
                                width,
                                height: line.metric.height,
                                ascent: line.metric.ascent,
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
                    bounds,
                );
            }
        }
    }

    pub fn get_link_overlays(
        &self,
        link_ranges: &[crate::model::LinkRange],
    ) -> Vec<(String, Vec<crate::types::TextBound>)> {
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
