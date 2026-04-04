use editor_common::{Axis, Direction, Movement};
use editor_resource::TextSegmenters;
use editor_state::{Affinity, Position, Selection};

use crate::paginate::*;
use crate::viewport::Viewport;

use super::cursor::x_at_offset;
use super::{search, segmentation};

pub fn resolve_movement(
    tree: &LayoutTree,
    pos: &Position,
    movement: &Movement,
    viewport: &Viewport,
    segmenters: Option<&TextSegmenters>,
) -> Option<Selection> {
    match movement {
        Movement::Grapheme {
            direction: Direction::Forward,
        } => move_grapheme_forward(tree, pos),
        Movement::Grapheme {
            direction: Direction::Backward,
        } => move_grapheme_backward(tree, pos),
        Movement::Word {
            direction: Direction::Forward,
        } => segmenters.and_then(|s| segmentation::move_word_forward(tree, pos, s)),
        Movement::Word {
            direction: Direction::Backward,
        } => segmenters.and_then(|s| segmentation::move_word_backward(tree, pos, s)),
        Movement::Sentence {
            direction: Direction::Forward,
        } => segmenters.and_then(|s| segmentation::move_sentence_forward(tree, pos, s)),
        Movement::Sentence {
            direction: Direction::Backward,
        } => segmenters.and_then(|s| segmentation::move_sentence_backward(tree, pos, s)),
        Movement::Line {
            direction: Direction::Forward,
            axis: Axis::Horizontal,
        } => move_line_horizontal_forward(tree, pos),
        Movement::Line {
            direction: Direction::Backward,
            axis: Axis::Horizontal,
        } => move_line_horizontal_backward(tree, pos),
        Movement::Line {
            direction: Direction::Forward,
            axis: Axis::Vertical,
        } => move_line_vertical_forward(tree, pos),
        Movement::Line {
            direction: Direction::Backward,
            axis: Axis::Vertical,
        } => move_line_vertical_backward(tree, pos),
        Movement::Block {
            direction: Direction::Forward,
        } => move_block_forward(tree, pos),
        Movement::Block {
            direction: Direction::Backward,
        } => move_block_backward(tree, pos),
        Movement::Page {
            direction: Direction::Forward,
        } => move_page_forward(tree, pos, viewport),
        Movement::Page {
            direction: Direction::Backward,
        } => move_page_backward(tree, pos, viewport),
        Movement::Document {
            direction: Direction::Forward,
        } => move_document_forward(tree),
        Movement::Document {
            direction: Direction::Backward,
        } => move_document_backward(tree),
    }
}

fn move_grapheme_forward(tree: &LayoutTree, pos: &Position) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;

    match &line_node.content {
        LayoutContent::Line(line) => {
            for run in &line.glyph_runs {
                if run.node_id != pos.node_id {
                    continue;
                }
                let local = pos.offset - run.offset;
                if local < run.char_advances.len() {
                    return Some(Selection::collapsed(Position::new(
                        run.node_id,
                        pos.offset + 1,
                    )));
                }
            }
            // At end of line: advance to the next navigable node
            let y = line_node.rect.bottom();
            let next = search::find_navigable_below(&tree.root, y)?;
            Some(Selection::collapsed(first_position_in(next)))
        }
        LayoutContent::Atom(a) => {
            let y = line_node.rect.bottom();
            if let Some(next) = search::find_navigable_below(&tree.root, y) {
                Some(Selection::collapsed(first_position_in(next)))
            } else {
                Some(Selection::collapsed(Position::new(
                    a.parent_id,
                    a.index + 1,
                )))
            }
        }
        _ => None,
    }
}

fn move_grapheme_backward(tree: &LayoutTree, pos: &Position) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;

    match &line_node.content {
        LayoutContent::Line(line) => {
            for run in &line.glyph_runs {
                if run.node_id != pos.node_id {
                    continue;
                }
                if pos.offset > run.offset {
                    return Some(Selection::collapsed(Position::new(
                        run.node_id,
                        pos.offset - 1,
                    )));
                }
            }
            // At start of line: retreat to the previous navigable node
            let y = line_node.rect.y;
            let prev = search::find_navigable_above(&tree.root, y)?;
            Some(Selection::collapsed(last_position_in(prev)))
        }
        LayoutContent::Atom(a) => {
            let y = line_node.rect.y;
            if let Some(prev) = search::find_navigable_above(&tree.root, y) {
                Some(Selection::collapsed(last_position_in(prev)))
            } else {
                Some(Selection::collapsed(Position::new(a.parent_id, a.index)))
            }
        }
        _ => None,
    }
}

fn move_line_horizontal_forward(tree: &LayoutTree, pos: &Position) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;
    match &line_node.content {
        LayoutContent::Line(line) => Some(Selection::collapsed(last_position_in_line(line))),
        _ => None,
    }
}

fn move_line_horizontal_backward(tree: &LayoutTree, pos: &Position) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;
    match &line_node.content {
        LayoutContent::Line(line) => Some(Selection::collapsed(first_position_in_line(line))),
        _ => None,
    }
}

fn move_line_vertical_forward(tree: &LayoutTree, pos: &Position) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;
    let preferred_x = compute_preferred_x(line_node, pos);
    let y = line_node.rect.bottom();
    let target = search::find_navigable_below(&tree.root, y)?;
    Some(navigate_to(target, preferred_x))
}

fn move_line_vertical_backward(tree: &LayoutTree, pos: &Position) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;
    let preferred_x = compute_preferred_x(line_node, pos);
    let y = line_node.rect.y;
    let target = search::find_navigable_above(&tree.root, y)?;
    Some(navigate_to(target, preferred_x))
}

fn move_block_forward(tree: &LayoutTree, pos: &Position) -> Option<Selection> {
    let container = search::find_scope_container_at(&tree.root, pos)?;
    let b = match &container.content {
        LayoutContent::Box(b) => b,
        _ => return None,
    };
    let nav = b
        .children
        .iter()
        .rev()
        .find_map(search::find_last_navigable)?;
    Some(Selection::collapsed(last_position_in(nav)))
}

fn move_block_backward(tree: &LayoutTree, pos: &Position) -> Option<Selection> {
    let container = search::find_scope_container_at(&tree.root, pos)?;
    let b = match &container.content {
        LayoutContent::Box(b) => b,
        _ => return None,
    };
    let nav = b.children.iter().find_map(search::find_first_navigable)?;
    Some(Selection::collapsed(first_position_in(nav)))
}

fn move_page_forward(tree: &LayoutTree, pos: &Position, viewport: &Viewport) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;
    let preferred_x = compute_preferred_x(line_node, pos);
    let y = line_node.rect.y + viewport.height;
    let target = search::find_navigable_below(&tree.root, y)?;
    Some(navigate_to(target, preferred_x))
}

fn move_page_backward(tree: &LayoutTree, pos: &Position, viewport: &Viewport) -> Option<Selection> {
    let line_node = search::find_line_at(tree, pos)?;
    let preferred_x = compute_preferred_x(line_node, pos);
    let y = line_node.rect.bottom() - viewport.height;
    let target = search::find_navigable_above(&tree.root, y)?;
    Some(navigate_to(target, preferred_x))
}

fn move_document_forward(tree: &LayoutTree) -> Option<Selection> {
    let nav = search::find_last_navigable(&tree.root)?;
    Some(Selection::collapsed(last_position_in(nav)))
}

fn move_document_backward(tree: &LayoutTree) -> Option<Selection> {
    let nav = search::find_first_navigable(&tree.root)?;
    Some(Selection::collapsed(first_position_in(nav)))
}

fn first_position_in_line(line: &LayoutLine) -> Position {
    if let Some(run) = line.glyph_runs.first() {
        Position::new(run.node_id, run.offset)
    } else {
        Position::new(line.node_id, 0)
    }
}

fn last_position_in_line(line: &LayoutLine) -> Position {
    if let Some(run) = line.glyph_runs.last() {
        Position::new(run.node_id, run.offset + run.char_advances.len())
    } else {
        Position::new(line.node_id, 0)
    }
}

pub(crate) fn first_position_in(node: &LayoutNode) -> Position {
    match &node.content {
        LayoutContent::Line(line) => first_position_in_line(line),
        LayoutContent::Atom(atom) => Position::new(atom.parent_id, atom.index),
        LayoutContent::Box(_) | LayoutContent::Spacing(_) => unreachable!(),
    }
}

pub(crate) fn last_position_in(node: &LayoutNode) -> Position {
    match &node.content {
        LayoutContent::Line(line) => last_position_in_line(line),
        LayoutContent::Atom(atom) => Position::new(atom.parent_id, atom.index),
        LayoutContent::Box(_) | LayoutContent::Spacing(_) => unreachable!(),
    }
}

fn navigate_to(node: &LayoutNode, preferred_x: f32) -> Selection {
    match &node.content {
        LayoutContent::Line(line) => {
            Selection::collapsed(position_in_line(line, &node.rect, preferred_x))
        }
        LayoutContent::Atom(atom) => Selection::new(
            Position {
                node_id: atom.parent_id,
                offset: atom.index,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: atom.parent_id,
                offset: atom.index + 1,
                affinity: Affinity::Upstream,
            },
        ),
        _ => unreachable!("navigate_to called on non-navigable"),
    }
}

fn position_in_line(line: &LayoutLine, rect: &editor_common::Rect, x: f32) -> Position {
    let local_x = x - rect.x;
    for run in &line.glyph_runs {
        if local_x > run.x + run.width {
            continue;
        }
        if local_x < run.x {
            return Position::new(run.node_id, run.offset);
        }
        let mut acc = run.x;
        for (i, &adv) in run.char_advances.iter().enumerate() {
            if local_x < acc + adv / 2.0 {
                return Position::new(run.node_id, run.offset + i);
            }
            acc += adv;
        }
        return Position::new(run.node_id, run.offset + run.char_advances.len());
    }
    // Fallback: position at the end of the last run
    if let Some(last) = line.glyph_runs.last() {
        Position::new(last.node_id, last.offset + last.char_advances.len())
    } else {
        Position::new(line.node_id, 0)
    }
}

fn compute_preferred_x(line_node: &LayoutNode, pos: &Position) -> f32 {
    match &line_node.content {
        LayoutContent::Line(line) => line_node.rect.x + x_at_offset(line, pos),
        _ => line_node.rect.x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph_run::GlyphRun;
    use crate::style::{BorderMode, BoxStyle, Direction as LayoutDirection};
    use editor_common::{Alignment, Direction, EdgeInsets, Rect};
    use editor_model::NodeId;

    fn make_line_node(id: NodeId, y: f32, text: &str) -> LayoutNode {
        let n = text.chars().count();
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: id,
                baseline: 16.0,
                glyph_runs: vec![GlyphRun::make_test_run(id, 0, text, 0.0, vec![10.0; n])],
            }),
        }
    }

    fn make_atom_node(parent_id: NodeId, node_id: NodeId, y: f32, index: usize) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, 20.0),
            content: LayoutContent::Atom(LayoutAtom {
                node_id,
                parent_id,
                index,
            }),
        }
    }

    fn make_box_node(y: f32, h: f32, scope: bool, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, h),
            content: LayoutContent::Box(LayoutBox {
                node_id: NodeId::new(),
                style: BoxStyle {
                    direction: LayoutDirection::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    scope,
                    decorations: vec![],
                },
                children,
            }),
        }
    }

    fn make_spacing(y: f32, h: f32) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 0.0, h),
            content: LayoutContent::Spacing(SpacingKind::Gap),
        }
    }

    /// Shared fixture:
    ///
    /// ```text
    /// Block1 (scope, y=0..40):
    ///   Line0: "hello world"  (y=0,  h=20)
    ///   Line1: "foo bar"      (y=20, h=20)
    /// Block2 (scope, y=40..120):
    ///   Atom                  (y=40, h=20, parent=atom_parent, index=0)
    ///   Line2: "baz"          (y=60, h=20)
    ///   Line3: "qux quux"     (y=80, h=20)
    ///   Line4: "end"          (y=100, h=20)
    /// ```
    struct Fixture {
        tree: LayoutTree,
        lines: [NodeId; 5],
        atom_parent: NodeId,
    }

    fn fixture() -> Fixture {
        let lines: [NodeId; 5] = std::array::from_fn(|_| NodeId::new());
        let atom_parent = NodeId::new();
        let atom_id = NodeId::new();

        let tree = LayoutTree {
            root: make_box_node(
                0.0,
                120.0,
                false,
                vec![
                    make_box_node(
                        0.0,
                        40.0,
                        true,
                        vec![
                            make_line_node(lines[0], 0.0, "hello world"),
                            make_line_node(lines[1], 20.0, "foo bar"),
                        ],
                    ),
                    make_box_node(
                        40.0,
                        80.0,
                        true,
                        vec![
                            make_atom_node(atom_parent, atom_id, 40.0, 0),
                            make_line_node(lines[2], 60.0, "baz"),
                            make_line_node(lines[3], 80.0, "qux quux"),
                            make_line_node(lines[4], 100.0, "end"),
                        ],
                    ),
                ],
            ),
        };

        Fixture {
            tree,
            lines,
            atom_parent,
        }
    }

    const VP: Viewport = Viewport {
        width: 200.0,
        height: 800.0,
        scale_factor: 1.0,
    };

    fn mov(tree: &LayoutTree, pos: Position, movement: Movement) -> Option<Selection> {
        resolve_movement(tree, &pos, &movement, &VP, None)
    }

    #[test]
    fn grapheme_forward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 2),
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.offset, 3);
    }

    #[test]
    fn grapheme_backward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 3),
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.offset, 2);
    }

    #[test]
    fn grapheme_forward_at_line_end_wraps() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 11),
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[1]);
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn grapheme_backward_at_line_start_wraps() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[1], 0),
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[0]);
        assert_eq!(sel.head.offset, 11);
    }

    #[test]
    fn word_movement_without_segmenters_returns_none() {
        let f = fixture();
        assert!(
            mov(
                &f.tree,
                Position::new(f.lines[0], 0),
                Movement::Word {
                    direction: Direction::Forward
                },
            )
            .is_none()
        );
    }

    #[test]
    fn line_horizontal_forward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 2),
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Horizontal,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[0]);
        assert_eq!(sel.head.offset, 11);
    }

    #[test]
    fn line_horizontal_backward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 5),
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Horizontal,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[0]);
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn line_vertical_forward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 2),
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[1]);
    }

    #[test]
    fn line_vertical_backward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[1], 2),
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[0]);
    }

    #[test]
    fn line_vertical_forward_at_last_returns_none() {
        let f = fixture();
        assert!(
            mov(
                &f.tree,
                Position::new(f.lines[4], 0),
                Movement::Line {
                    direction: Direction::Forward,
                    axis: Axis::Vertical
                },
            )
            .is_none()
        );
    }

    #[test]
    fn move_line_down_skips_spacing() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let tree = LayoutTree {
            root: make_box_node(
                0.0,
                56.0,
                false,
                vec![
                    make_line_node(id1, 0.0, "hello"),
                    make_spacing(20.0, 16.0),
                    make_line_node(id2, 36.0, "world"),
                ],
            ),
        };
        let sel = mov(
            &tree,
            Position::new(id1, 2),
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        )
        .unwrap();
        assert_eq!(sel.head.node_id, id2);
    }

    #[test]
    fn block_forward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 2),
            Movement::Block {
                direction: Direction::Forward,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[1]);
        assert_eq!(sel.head.offset, 7);
    }

    #[test]
    fn block_backward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[1], 3),
            Movement::Block {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[0]);
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn block_forward_from_atom() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.atom_parent, 0),
            Movement::Block {
                direction: Direction::Forward,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[4]);
        assert_eq!(sel.head.offset, 3);
    }

    #[test]
    fn block_backward_to_atom() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[2], 1),
            Movement::Block {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.atom_parent);
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn page_forward_skips_lines() {
        let f = fixture();
        let vp = Viewport {
            width: 200.0,
            height: 50.0,
            scale_factor: 1.0,
        };
        let sel = resolve_movement(
            &f.tree,
            &Position::new(f.lines[0], 0),
            &Movement::Page {
                direction: Direction::Forward,
            },
            &vp,
            None,
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[2]);
    }

    #[test]
    fn page_backward_skips_lines() {
        let f = fixture();
        let vp = Viewport {
            width: 200.0,
            height: 50.0,
            scale_factor: 1.0,
        };
        let sel = resolve_movement(
            &f.tree,
            &Position::new(f.lines[4], 0),
            &Movement::Page {
                direction: Direction::Backward,
            },
            &vp,
            None,
        )
        .unwrap();
        assert_eq!(sel.anchor.node_id, f.atom_parent);
        assert_eq!(sel.anchor.offset, 0);
        assert_eq!(sel.head.node_id, f.atom_parent);
        assert_eq!(sel.head.offset, 1);
    }

    #[test]
    fn move_document_start_end() {
        let f = fixture();
        let sel_start = mov(
            &f.tree,
            Position::new(f.lines[4], 2),
            Movement::Document {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        assert_eq!(sel_start.head, sel_start.anchor);
        assert_eq!(sel_start.head.node_id, f.lines[0]);
        assert_eq!(sel_start.head.offset, 0);

        let sel_end = mov(
            &f.tree,
            Position::new(f.lines[0], 0),
            Movement::Document {
                direction: Direction::Forward,
            },
        )
        .unwrap();
        assert_eq!(sel_end.head, sel_end.anchor);
        assert_eq!(sel_end.head.node_id, f.lines[4]);
        assert_eq!(sel_end.head.offset, 3);
    }

    #[test]
    fn move_line_start_end() {
        let f = fixture();
        let sel_end = mov(
            &f.tree,
            Position::new(f.lines[0], 2),
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Horizontal,
            },
        )
        .unwrap();
        assert_eq!(sel_end.head.node_id, f.lines[0]);
        assert_eq!(sel_end.head.offset, 11);

        let sel_start = mov(
            &f.tree,
            Position::new(f.lines[0], 5),
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Horizontal,
            },
        )
        .unwrap();
        assert_eq!(sel_start.head.node_id, f.lines[0]);
        assert_eq!(sel_start.head.offset, 0);
    }
}

#[cfg(test)]
mod integration_tests {
    use editor_common::{Axis, Direction, Movement};
    use editor_macros::state;
    use editor_state::Position;

    use crate::view::View;

    #[test]
    fn line_vertical_forward_from_start_two_paragraphs() {
        let (state, t1, t2) = state! {
            doc {
                root [paragraph_indent(1), block_gap(1)] {
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("world") }
                }
            }
            selection: (t1, 0)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(t1, 0),
                &Movement::Line {
                    direction: Direction::Forward,
                    axis: Axis::Vertical,
                },
                &state.doc,
                None,
            )
            .unwrap();

        assert_eq!(sel.head.node_id, t2);
        assert_eq!(sel.head.offset, 0);
    }
}
