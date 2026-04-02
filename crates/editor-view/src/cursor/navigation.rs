use editor_common::{Axis, Direction, Movement};
use editor_resource::TextSegmenters;
use editor_state::{Position, Selection};

use crate::cursor::{search, segmentation};
use crate::fragment::*;
use crate::page::Page;
use crate::viewport::Viewport;

pub fn resolve_movement(
    pages: &[Page],
    pos: &Position,
    movement: &Movement,
    viewport: &Viewport,
    segmenters: Option<&TextSegmenters>,
) -> Option<Selection> {
    match movement {
        Movement::Grapheme(Direction::Forward) => move_grapheme_forward(pages, pos),
        Movement::Grapheme(Direction::Backward) => move_grapheme_backward(pages, pos),
        Movement::Word(Direction::Forward) => {
            segmenters.and_then(|s| segmentation::move_word_forward(pages, pos, s))
        }
        Movement::Word(Direction::Backward) => {
            segmenters.and_then(|s| segmentation::move_word_backward(pages, pos, s))
        }
        Movement::Sentence(Direction::Forward) => {
            segmenters.and_then(|s| segmentation::move_sentence_forward(pages, pos, s))
        }
        Movement::Sentence(Direction::Backward) => {
            segmenters.and_then(|s| segmentation::move_sentence_backward(pages, pos, s))
        }
        Movement::Line(Direction::Forward, Axis::Horizontal) => {
            move_line_horizontal_forward(pages, pos)
        }
        Movement::Line(Direction::Backward, Axis::Horizontal) => {
            move_line_horizontal_backward(pages, pos)
        }
        Movement::Line(Direction::Forward, Axis::Vertical) => {
            move_line_vertical_forward(pages, pos)
        }
        Movement::Line(Direction::Backward, Axis::Vertical) => {
            move_line_vertical_backward(pages, pos)
        }
        Movement::Block(Direction::Forward) => move_block_forward(pages, pos),
        Movement::Block(Direction::Backward) => move_block_backward(pages, pos),
        Movement::Page(Direction::Forward) => move_page_forward(pages, pos, viewport),
        Movement::Page(Direction::Backward) => move_page_backward(pages, pos, viewport),
        Movement::Document(Direction::Forward) => move_document_forward(pages),
        Movement::Document(Direction::Backward) => move_document_backward(pages),
    }
}

fn move_grapheme_forward(pages: &[Page], pos: &Position) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;

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

    let y = line.rect.bottom();
    let (_, next) = search::find_navigable_below(pages, page_idx, y, 0.0)?;
    Some(Selection::collapsed(first_position_in(next)))
}

fn move_grapheme_backward(pages: &[Page], pos: &Position) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;

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

    let y = line.rect.y;
    let (_, prev) = search::find_navigable_above(pages, page_idx, y, 0.0)?;
    Some(Selection::collapsed(last_position_in(prev)))
}

fn move_line_horizontal_forward(pages: &[Page], pos: &Position) -> Option<Selection> {
    let (_, line) = search::find_line_at(pages, pos)?;
    Some(Selection::collapsed(last_position_in_line(line)))
}

fn move_line_horizontal_backward(pages: &[Page], pos: &Position) -> Option<Selection> {
    let (_, line) = search::find_line_at(pages, pos)?;
    Some(Selection::collapsed(first_position_in_line(line)))
}

fn move_line_vertical_forward(pages: &[Page], pos: &Position) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let preferred_x = line.rect.x + super::x_at_offset(line, pos);
    let y = line.rect.bottom();
    let (_, target) = search::find_navigable_below(pages, page_idx, y, preferred_x)?;
    Some(navigate_to(target, preferred_x))
}

fn move_line_vertical_backward(pages: &[Page], pos: &Position) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let preferred_x = line.rect.x + super::x_at_offset(line, pos);
    let y = line.rect.y;
    let (_, target) = search::find_navigable_above(pages, page_idx, y, preferred_x)?;
    Some(navigate_to(target, preferred_x))
}

fn move_block_forward(pages: &[Page], pos: &Position) -> Option<Selection> {
    let container = search::find_scope_container_at(pages, pos)?;
    let nav = container
        .children
        .iter()
        .rev()
        .find_map(search::find_last_navigable)?;
    Some(Selection::collapsed(last_position_in(nav)))
}

fn move_block_backward(pages: &[Page], pos: &Position) -> Option<Selection> {
    let container = search::find_scope_container_at(pages, pos)?;
    let nav = container
        .children
        .iter()
        .find_map(search::find_first_navigable)?;
    Some(Selection::collapsed(first_position_in(nav)))
}

fn move_page_forward(pages: &[Page], pos: &Position, viewport: &Viewport) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let preferred_x = line.rect.x + super::x_at_offset(line, pos);
    let y = line.rect.y + viewport.height;
    let (_, target) = search::find_navigable_below(pages, page_idx, y, preferred_x)?;
    Some(navigate_to(target, preferred_x))
}

fn move_page_backward(pages: &[Page], pos: &Position, viewport: &Viewport) -> Option<Selection> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let preferred_x = line.rect.x + super::x_at_offset(line, pos);
    let y = line.rect.bottom() - viewport.height;
    let (_, target) = search::find_navigable_above(pages, page_idx, y, preferred_x)?;
    Some(navigate_to(target, preferred_x))
}

fn move_document_forward(pages: &[Page]) -> Option<Selection> {
    let page = pages.last()?;
    let nav = page
        .fragments
        .iter()
        .rev()
        .find_map(search::find_last_navigable)?;
    Some(Selection::collapsed(last_position_in(nav)))
}

fn move_document_backward(pages: &[Page]) -> Option<Selection> {
    let page = pages.first()?;
    let nav = page
        .fragments
        .iter()
        .find_map(search::find_first_navigable)?;
    Some(Selection::collapsed(first_position_in(nav)))
}

fn first_position_in_line(line: &LineFragment) -> Position {
    if let Some(run) = line.glyph_runs.first() {
        Position::new(run.node_id, run.offset)
    } else {
        Position::new(line.node_id, 0)
    }
}

fn last_position_in_line(line: &LineFragment) -> Position {
    if let Some(run) = line.glyph_runs.last() {
        Position::new(run.node_id, run.offset + run.char_advances.len())
    } else {
        Position::new(line.node_id, 0)
    }
}

pub(crate) fn first_position_in(fragment: &Fragment) -> Position {
    match fragment {
        Fragment::Line(line) => first_position_in_line(line),
        Fragment::Atom(atom) => Position::new(atom.parent_id, atom.index),
        Fragment::Container(_) => unreachable!(),
        Fragment::Placeholder(_) => unreachable!(),
    }
}

pub(crate) fn last_position_in(fragment: &Fragment) -> Position {
    match fragment {
        Fragment::Line(line) => last_position_in_line(line),
        Fragment::Atom(atom) => Position::new(atom.parent_id, atom.index),
        Fragment::Container(_) => unreachable!(),
        Fragment::Placeholder(_) => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use editor_common::{EdgeInsets, Rect};
    use editor_model::NodeId;

    use super::*;
    use crate::viewport::Viewport;

    /// Shared test fixture:
    ///
    /// ```text
    /// Block1 (scope, y=0..40):
    ///   Line0: "hello world"  (y=0,  h=20, 11 chars × 10px)
    ///   Line1: "foo bar"      (y=20, h=20,  7 chars × 10px)
    /// Block2 (scope, y=40..120):
    ///   Atom                  (y=40, h=20, parent=atom_parent, index=0)
    ///   Line2: "baz"          (y=60, h=20,  3 chars × 10px)
    ///   Line3: "qux quux"     (y=80, h=20,  8 chars × 10px)
    ///   Line4: "end"          (y=100, h=20, 3 chars × 10px)
    /// ```
    struct Fixture {
        page: Page,
        lines: [NodeId; 5],
        atom_parent: NodeId,
    }

    fn fixture() -> Fixture {
        let lines: [NodeId; 5] = std::array::from_fn(|_| NodeId::new());
        let atom_parent = NodeId::new();
        let atom_id = NodeId::new();

        let make_line = |id: NodeId, y: f32, text: &str| {
            let n = text.chars().count();
            Fragment::Line(LineFragment {
                node_id: id,
                rect: Rect {
                    x: 0.0,
                    y,
                    width: 200.0,
                    height: 20.0,
                },
                baseline: 16.0,
                glyph_runs: vec![GlyphRun {
                    font_id: 0,
                    font_weight: 400,
                    font_size: 16.0,
                    synthesis: Synthesis::default(),
                    color: String::new(),
                    background_color: None,
                    glyphs: vec![],
                    node_id: id,
                    offset: 0,
                    text: text.into(),
                    x: 0.0,
                    width: 10.0 * n as f32,
                    char_advances: vec![10.0; n],
                }],
            })
        };

        let page = Page::new(
            vec![
                Fragment::Container(ContainerFragment {
                    node_id: NodeId::new(),
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 200.0,
                        height: 40.0,
                    },
                    children: vec![
                        make_line(lines[0], 0.0, "hello world"),
                        make_line(lines[1], 20.0, "foo bar"),
                    ],
                    scope: true,
                    breaks: Breaks::default(),
                    border: EdgeInsets::default(),
                }),
                Fragment::Container(ContainerFragment {
                    node_id: NodeId::new(),
                    rect: Rect {
                        x: 0.0,
                        y: 40.0,
                        width: 200.0,
                        height: 80.0,
                    },
                    children: vec![
                        Fragment::Atom(AtomFragment {
                            node_id: atom_id,
                            parent_id: atom_parent,
                            index: 0,
                            rect: Rect {
                                x: 0.0,
                                y: 40.0,
                                width: 200.0,
                                height: 20.0,
                            },
                        }),
                        make_line(lines[2], 60.0, "baz"),
                        make_line(lines[3], 80.0, "qux quux"),
                        make_line(lines[4], 100.0, "end"),
                    ],
                    scope: true,
                    breaks: Breaks::default(),
                    border: EdgeInsets::default(),
                }),
            ],
            800.0,
        );

        Fixture {
            page,
            lines,
            atom_parent,
        }
    }

    const VP: Viewport = Viewport {
        width: 200.0,
        height: 800.0,
        scale_factor: 1.0,
    };

    fn mov(
        pages: &[Page],
        pos: Position,
        movement: Movement,
        viewport: &Viewport,
    ) -> Option<Selection> {
        resolve_movement(pages, &pos, &movement, viewport, None)
    }

    #[test]
    fn grapheme_forward() {
        let f = fixture();
        let sel = mov(
            &[f.page],
            Position::new(f.lines[0], 2),
            Movement::Grapheme(Direction::Forward),
            &VP,
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.offset, 3);
    }

    #[test]
    fn grapheme_backward() {
        let f = fixture();
        let sel = mov(
            &[f.page],
            Position::new(f.lines[0], 3),
            Movement::Grapheme(Direction::Backward),
            &VP,
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.offset, 2);
    }

    #[test]
    fn grapheme_forward_at_line_end_wraps() {
        let f = fixture();
        let sel = mov(
            &[f.page],
            Position::new(f.lines[0], 11),
            Movement::Grapheme(Direction::Forward),
            &VP,
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
            &[f.page],
            Position::new(f.lines[1], 0),
            Movement::Grapheme(Direction::Backward),
            &VP,
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
                &[f.page],
                Position::new(f.lines[0], 0),
                Movement::Word(Direction::Forward),
                &VP
            )
            .is_none()
        );
    }

    #[test]
    fn line_horizontal_forward() {
        let f = fixture();
        let sel = mov(
            &[f.page],
            Position::new(f.lines[0], 2),
            Movement::Line(Direction::Forward, Axis::Horizontal),
            &VP,
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
            &[f.page],
            Position::new(f.lines[0], 5),
            Movement::Line(Direction::Backward, Axis::Horizontal),
            &VP,
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
            &[f.page],
            Position::new(f.lines[0], 2),
            Movement::Line(Direction::Forward, Axis::Vertical),
            &VP,
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[1]);
    }

    #[test]
    fn line_vertical_backward() {
        let f = fixture();
        let sel = mov(
            &[f.page],
            Position::new(f.lines[1], 2),
            Movement::Line(Direction::Backward, Axis::Vertical),
            &VP,
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
                &[f.page],
                Position::new(f.lines[4], 0),
                Movement::Line(Direction::Forward, Axis::Vertical),
                &VP
            )
            .is_none()
        );
    }

    #[test]
    fn block_forward() {
        let f = fixture();
        let sel = mov(
            &[f.page],
            Position::new(f.lines[0], 2),
            Movement::Block(Direction::Forward),
            &VP,
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
            &[f.page],
            Position::new(f.lines[1], 3),
            Movement::Block(Direction::Backward),
            &VP,
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
            &[f.page],
            Position::new(f.atom_parent, 0),
            Movement::Block(Direction::Forward),
            &VP,
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
            &[f.page],
            Position::new(f.lines[2], 1),
            Movement::Block(Direction::Backward),
            &VP,
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
        let sel = mov(
            &[f.page],
            Position::new(f.lines[0], 0),
            Movement::Page(Direction::Forward),
            &vp,
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
        let sel = mov(
            &[f.page],
            Position::new(f.lines[4], 0),
            Movement::Page(Direction::Backward),
            &vp,
        )
        .unwrap();
        assert_eq!(sel.anchor.node_id, f.atom_parent);
        assert_eq!(sel.anchor.offset, 0);
        assert_eq!(sel.head.node_id, f.atom_parent);
        assert_eq!(sel.head.offset, 1);
    }

    #[test]
    fn document_forward() {
        let f = fixture();
        let sel = mov(
            &[f.page],
            Position::new(f.lines[0], 0),
            Movement::Document(Direction::Forward),
            &VP,
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[4]);
        assert_eq!(sel.head.offset, 3);
    }

    #[test]
    fn document_backward() {
        let f = fixture();
        let sel = mov(
            &[f.page],
            Position::new(f.lines[4], 2),
            Movement::Document(Direction::Backward),
            &VP,
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[0]);
        assert_eq!(sel.head.offset, 0);
    }
}
