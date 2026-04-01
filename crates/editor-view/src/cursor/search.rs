use editor_state::Position;

use crate::fragment::*;
use crate::page::Page;

pub fn find_navigable_at_y<'a>(fragment: &'a Fragment, y: f32) -> Option<&'a Fragment> {
    match fragment {
        Fragment::Container(c) => {
            for child in &c.children {
                let r = child.rect();
                if y >= r.y && y < r.bottom() {
                    return find_navigable_at_y(child, y);
                }
            }

            closest_navigable_by_y(&c.children, y)
        }
        Fragment::Line(_) | Fragment::Atom(_) => Some(fragment),
        Fragment::Placeholder(_) => None,
    }
}

pub fn closest_navigable_by_y<'a>(children: &'a [Fragment], y: f32) -> Option<&'a Fragment> {
    children
        .iter()
        .filter_map(|child| {
            find_first_navigable(child).map(|nav| {
                let dist = (nav.rect().y + nav.rect().height / 2.0 - y).abs();
                (dist, nav)
            })
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, nav)| nav)
}

pub fn closest_navigable_by_y_in<'a>(fragment: &'a Fragment, y: f32) -> Option<&'a Fragment> {
    match fragment {
        Fragment::Container(c) => closest_navigable_by_y(&c.children, y),
        Fragment::Line(_) | Fragment::Atom(_) => Some(fragment),
        Fragment::Placeholder(_) => None,
    }
}

pub fn find_first_navigable(fragment: &Fragment) -> Option<&Fragment> {
    match fragment {
        Fragment::Container(c) => c.children.iter().find_map(find_first_navigable),
        Fragment::Line(_) | Fragment::Atom(_) => Some(fragment),
        Fragment::Placeholder(_) => None,
    }
}

pub fn find_last_navigable(fragment: &Fragment) -> Option<&Fragment> {
    match fragment {
        Fragment::Container(c) => c.children.iter().rev().find_map(find_last_navigable),
        Fragment::Line(_) | Fragment::Atom(_) => Some(fragment),
        Fragment::Placeholder(_) => None,
    }
}

pub fn find_line_at<'a>(pages: &'a [Page], pos: &Position) -> Option<(usize, &'a LineFragment)> {
    for (page_idx, page) in pages.iter().enumerate() {
        for frag in &page.fragments {
            if let Some(line) = find_line_for_position(frag, pos) {
                return Some((page_idx, line));
            }
        }
    }

    None
}

fn find_line_for_position<'a>(fragment: &'a Fragment, pos: &Position) -> Option<&'a LineFragment> {
    match fragment {
        Fragment::Line(line) => {
            let contains = line.glyph_runs.iter().any(|run| {
                run.node_id == pos.node_id
                    && pos.offset >= run.offset
                    && pos.offset <= run.offset + run.char_advances.len()
            });
            if contains { Some(line) } else { None }
        }
        Fragment::Container(c) => c
            .children
            .iter()
            .find_map(|child| find_line_for_position(child, pos)),
        Fragment::Atom(_) | Fragment::Placeholder(_) => None,
    }
}

pub fn find_scope_container_at<'a>(
    pages: &'a [Page],
    pos: &Position,
) -> Option<&'a ContainerFragment> {
    for page in pages {
        for frag in &page.fragments {
            if let Some(container) = find_scope_containing(frag, pos) {
                return Some(container);
            }
        }
    }
    None
}

fn find_scope_containing<'a>(
    fragment: &'a Fragment,
    pos: &Position,
) -> Option<&'a ContainerFragment> {
    match fragment {
        Fragment::Container(c) => {
            for child in &c.children {
                if let Some(inner) = find_scope_containing(child, pos) {
                    return Some(inner);
                }
            }
            if c.scope && contains_position(fragment, pos) {
                return Some(c);
            }
            None
        }
        _ => None,
    }
}

fn contains_position(fragment: &Fragment, pos: &Position) -> bool {
    match fragment {
        Fragment::Line(line) => line.glyph_runs.iter().any(|run| {
            run.node_id == pos.node_id
                && pos.offset >= run.offset
                && pos.offset <= run.offset + run.char_advances.len()
        }),
        Fragment::Container(c) => c.children.iter().any(|child| contains_position(child, pos)),
        Fragment::Atom(atom) => atom.parent_id == pos.node_id && pos.offset == atom.index,
        Fragment::Placeholder(_) => false,
    }
}

pub fn find_navigable_below<'a>(
    pages: &'a [Page],
    page_idx: usize,
    y: f32,
    preferred_x: f32,
) -> Option<(usize, &'a Fragment)> {
    if let Some(frag) = find_navigable_below_in_page(&pages[page_idx], y, preferred_x) {
        return Some((page_idx, frag));
    }

    if page_idx + 1 < pages.len() {
        let next = &pages[page_idx + 1];
        for frag in &next.fragments {
            if let Some(nav) = find_first_navigable(frag) {
                return Some((page_idx + 1, nav));
            }
        }
    }

    None
}

pub fn find_navigable_above<'a>(
    pages: &'a [Page],
    page_idx: usize,
    y: f32,
    preferred_x: f32,
) -> Option<(usize, &'a Fragment)> {
    if let Some(frag) = find_navigable_above_in_page(&pages[page_idx], y, preferred_x) {
        return Some((page_idx, frag));
    }

    if page_idx > 0 {
        let prev = &pages[page_idx - 1];
        for frag in prev.fragments.iter().rev() {
            if let Some(nav) = find_last_navigable(frag) {
                return Some((page_idx - 1, nav));
            }
        }
    }

    None
}

fn find_navigable_below_in_page<'a>(
    page: &'a Page,
    y: f32,
    _preferred_x: f32,
) -> Option<&'a Fragment> {
    for frag in &page.fragments {
        if let Some(nav) = find_navigable_below_in_fragment(frag, y) {
            return Some(nav);
        }
    }

    None
}

fn find_navigable_above_in_page<'a>(
    page: &'a Page,
    y: f32,
    _preferred_x: f32,
) -> Option<&'a Fragment> {
    for frag in page.fragments.iter().rev() {
        if let Some(nav) = find_navigable_above_in_fragment(frag, y) {
            return Some(nav);
        }
    }

    None
}

fn find_navigable_below_in_fragment<'a>(fragment: &'a Fragment, y: f32) -> Option<&'a Fragment> {
    match fragment {
        Fragment::Line(_) | Fragment::Atom(_) => {
            if fragment.rect().y >= y {
                Some(fragment)
            } else {
                None
            }
        }
        Fragment::Container(c) => {
            for child in &c.children {
                if let Some(nav) = find_navigable_below_in_fragment(child, y) {
                    return Some(nav);
                }
            }

            None
        }
        Fragment::Placeholder(_) => None,
    }
}

fn find_navigable_above_in_fragment<'a>(fragment: &'a Fragment, y: f32) -> Option<&'a Fragment> {
    match fragment {
        Fragment::Line(_) | Fragment::Atom(_) => {
            if fragment.rect().bottom() <= y {
                Some(fragment)
            } else {
                None
            }
        }
        Fragment::Container(c) => {
            for child in c.children.iter().rev() {
                if let Some(nav) = find_navigable_above_in_fragment(child, y) {
                    return Some(nav);
                }
            }

            None
        }
        Fragment::Placeholder(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use editor_common::{EdgeInsets, Rect};
    use editor_model::NodeId;

    use super::*;

    fn line_frag(id: NodeId, y: f32) -> Fragment {
        Fragment::Line(LineFragment {
            node_id: id,
            rect: Rect {
                x: 0.0,
                y,
                width: 200.0,
                height: 20.0,
            },
            baseline: 16.0,
            glyph_runs: vec![GlyphRun::make_test_run(id, 0, "test", 0.0, vec![10.0; 4])],
        })
    }

    fn container_frag(id: NodeId, y: f32, h: f32, children: Vec<Fragment>) -> Fragment {
        Fragment::Container(ContainerFragment {
            node_id: id,
            rect: Rect {
                x: 0.0,
                y,
                width: 200.0,
                height: h,
            },
            children,
            scope: false,
            breaks: Breaks::default(),
            border: EdgeInsets::default(),
        })
    }

    #[test]
    fn find_navigable_at_y_finds_line() {
        let id = NodeId::new();
        let tree = container_frag(
            NodeId::new(),
            0.0,
            40.0,
            vec![line_frag(id, 0.0), line_frag(NodeId::new(), 20.0)],
        );
        let result = find_navigable_at_y(&tree, 5.0);
        assert_eq!(result.unwrap().node_id().unwrap(), id);
    }

    #[test]
    fn find_navigable_at_y_closest_when_no_exact_match() {
        let id = NodeId::new();
        let tree = container_frag(NodeId::new(), 0.0, 20.0, vec![line_frag(id, 0.0)]);
        let result = find_navigable_at_y(&tree, 50.0);
        assert_eq!(result.unwrap().node_id().unwrap(), id);
    }

    #[test]
    fn find_first_navigable_skips_containers() {
        let id = NodeId::new();
        let tree = container_frag(
            NodeId::new(),
            0.0,
            40.0,
            vec![container_frag(
                NodeId::new(),
                0.0,
                20.0,
                vec![line_frag(id, 0.0)],
            )],
        );
        assert_eq!(find_first_navigable(&tree).unwrap().node_id().unwrap(), id);
    }

    #[test]
    fn find_last_navigable_returns_bottom() {
        let id = NodeId::new();
        let tree = container_frag(
            NodeId::new(),
            0.0,
            40.0,
            vec![line_frag(NodeId::new(), 0.0), line_frag(id, 20.0)],
        );
        assert_eq!(find_last_navigable(&tree).unwrap().node_id().unwrap(), id);
    }

    #[test]
    fn find_line_at_locates_position() {
        let id = NodeId::new();
        let page = Page::new(
            vec![container_frag(
                NodeId::new(),
                0.0,
                40.0,
                vec![line_frag(id, 0.0), line_frag(NodeId::new(), 20.0)],
            )],
            800.0,
        );
        let pos = Position::new(id, 2);
        let pages = [page];
        let (page_idx, line) = find_line_at(&pages, &pos).unwrap();
        assert_eq!(page_idx, 0);
        assert_eq!(line.node_id, id);
    }

    #[test]
    fn find_navigable_below_finds_in_same_page() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let page = Page::new(vec![line_frag(id1, 0.0), line_frag(id2, 20.0)], 800.0);
        let pages = [page];
        let (_, nav) = find_navigable_below(&pages, 0, 10.0, 0.0).unwrap();
        assert_eq!(nav.node_id().unwrap(), id2);
    }

    #[test]
    fn find_navigable_below_crosses_page() {
        let id = NodeId::new();
        let page1 = Page::new(vec![line_frag(NodeId::new(), 0.0)], 40.0);
        let page2 = Page::new(vec![line_frag(id, 0.0)], 40.0);
        let pages = [page1, page2];
        let (page_idx, nav) = find_navigable_below(&pages, 0, 30.0, 0.0).unwrap();
        assert_eq!(page_idx, 1);
        assert_eq!(nav.node_id().unwrap(), id);
    }

    #[test]
    fn find_navigable_above_finds_in_same_page() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let page = Page::new(vec![line_frag(id1, 0.0), line_frag(id2, 20.0)], 800.0);
        let pages = [page];
        let (_, nav) = find_navigable_above(&pages, 0, 20.0, 0.0).unwrap();
        assert_eq!(nav.node_id().unwrap(), id1);
    }

    #[test]
    fn find_navigable_returns_none_at_boundary() {
        let page = Page::new(vec![line_frag(NodeId::new(), 0.0)], 40.0);
        let pages = [page];
        assert!(find_navigable_above(&pages, 0, 0.0, 0.0).is_none());
    }
}
