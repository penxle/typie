use editor_state::Selection;

use crate::cursor::search;
use crate::fragment::*;
use crate::page::Page;

pub fn hit_test(page: &Page, x: f32, y: f32) -> Option<Selection> {
    for frag in &page.fragments {
        if let Some(sel) = hit_test_fragment(frag, x, y) {
            return Some(sel);
        }
    }

    page.fragments
        .iter()
        .find_map(|frag| search::closest_navigable_by_y_in(frag, y).map(|nav| navigate_to(nav, x)))
}

fn hit_test_fragment(fragment: &Fragment, x: f32, y: f32) -> Option<Selection> {
    match fragment {
        Fragment::Container(c) => {
            if !c.rect.contains(x, y) {
                return None;
            }

            for child in &c.children {
                if let Some(sel) = hit_test_fragment(child, x, y) {
                    return Some(sel);
                }
            }

            search::closest_navigable_by_y_in(fragment, y).map(|nav| navigate_to(nav, x))
        }
        Fragment::Line(_) => {
            let r = fragment.rect();
            if y >= r.y && y < r.bottom() {
                Some(navigate_to(fragment, x))
            } else {
                None
            }
        }
        Fragment::Atom(_) => {
            if fragment.rect().contains(x, y) {
                Some(navigate_to(fragment, x))
            } else {
                None
            }
        }
        Fragment::Placeholder(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use editor_common::{EdgeInsets, Rect};
    use editor_model::NodeId;

    use super::*;

    fn make_run(
        node_id: NodeId,
        offset: usize,
        text: &str,
        x: f32,
        advances: Vec<f32>,
    ) -> GlyphRun {
        GlyphRun::make_test_run(node_id, offset, text, x, advances)
    }

    #[test]
    fn hit_test_line() {
        let id = NodeId::new();
        let page = Page::new(
            vec![Fragment::Container(ContainerFragment {
                node_id: NodeId::new(),
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 200.0,
                    height: 20.0,
                },
                children: vec![Fragment::Line(LineFragment {
                    node_id: id,
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 200.0,
                        height: 20.0,
                    },
                    baseline: 16.0,
                    glyph_runs: vec![make_run(id, 0, "hello", 0.0, vec![10.0; 5])],
                })],
                scope: false,
                breaks: Breaks::default(),
                border: EdgeInsets::default(),
            })],
            800.0,
        );

        let sel = hit_test(&page, 25.0, 5.0).unwrap();

        assert!(sel.is_collapsed());
        assert_eq!(sel.head.node_id, id);
    }

    #[test]
    fn hit_test_second_line() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let page = Page::new(
            vec![Fragment::Container(ContainerFragment {
                node_id: NodeId::new(),
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 200.0,
                    height: 40.0,
                },
                children: vec![
                    Fragment::Line(LineFragment {
                        node_id: id1,
                        rect: Rect {
                            x: 0.0,
                            y: 0.0,
                            width: 200.0,
                            height: 20.0,
                        },
                        baseline: 16.0,
                        glyph_runs: vec![make_run(id1, 0, "hello", 0.0, vec![10.0; 5])],
                    }),
                    Fragment::Line(LineFragment {
                        node_id: id2,
                        rect: Rect {
                            x: 0.0,
                            y: 20.0,
                            width: 200.0,
                            height: 20.0,
                        },
                        baseline: 16.0,
                        glyph_runs: vec![make_run(id2, 0, "world", 0.0, vec![10.0; 5])],
                    }),
                ],
                scope: false,
                breaks: Breaks::default(),
                border: EdgeInsets::default(),
            })],
            800.0,
        );

        let sel = hit_test(&page, 5.0, 25.0).unwrap();

        assert_eq!(sel.head.node_id, id2);
    }

    #[test]
    fn hit_test_atom_returns_range() {
        let parent_id = NodeId::new();
        let page = Page::new(
            vec![Fragment::Atom(AtomFragment {
                node_id: NodeId::new(),
                parent_id,
                index: 0,
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 200.0,
                    height: 100.0,
                },
            })],
            800.0,
        );

        let sel = hit_test(&page, 100.0, 50.0).unwrap();

        assert!(!sel.is_collapsed());
        assert_eq!(sel.anchor.node_id, parent_id);
    }

    #[test]
    fn hit_test_fallback_to_closest() {
        let id = NodeId::new();
        let page = Page::new(
            vec![Fragment::Container(ContainerFragment {
                node_id: NodeId::new(),
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 200.0,
                    height: 20.0,
                },
                children: vec![Fragment::Line(LineFragment {
                    node_id: id,
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 200.0,
                        height: 20.0,
                    },
                    baseline: 16.0,
                    glyph_runs: vec![make_run(id, 0, "hello", 0.0, vec![10.0; 5])],
                })],
                scope: false,
                breaks: Breaks::default(),
                border: EdgeInsets::default(),
            })],
            800.0,
        );

        let sel = hit_test(&page, 25.0, 100.0).unwrap();

        assert_eq!(sel.head.node_id, id);
    }
}
