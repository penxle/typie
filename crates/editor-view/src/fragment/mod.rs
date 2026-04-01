mod atom;
mod container;
mod glyph_run;
mod line;
mod placeholder;

pub use atom::*;
pub use container::*;
pub use glyph_run::*;
pub use line::*;
pub use placeholder::*;

use editor_common::Rect;
use editor_model::NodeId;
use editor_state::{Affinity, Position, Selection};

#[derive(Debug, Clone)]
pub enum Fragment {
    Container(ContainerFragment),
    Line(LineFragment),
    Atom(AtomFragment),
    Placeholder(PlaceholderFragment),
}

impl Fragment {
    pub fn rect(&self) -> &Rect {
        match self {
            Fragment::Container(f) => &f.rect,
            Fragment::Line(f) => &f.rect,
            Fragment::Atom(f) => &f.rect,
            Fragment::Placeholder(f) => &f.rect,
        }
    }

    pub fn node_id(&self) -> Option<NodeId> {
        match self {
            Fragment::Container(f) => Some(f.node_id),
            Fragment::Line(f) => Some(f.node_id),
            Fragment::Atom(f) => Some(f.node_id),
            Fragment::Placeholder(_) => None,
        }
    }
}

pub fn navigate_to(fragment: &Fragment, preferred_x: f32) -> Selection {
    match fragment {
        Fragment::Line(line) => Selection::collapsed(position_in_line(line, preferred_x)),
        Fragment::Atom(atom) => Selection::new(
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
        Fragment::Container(_) => panic!("navigate_to called on Container"),
        Fragment::Placeholder(_) => panic!("navigate_to called on Placeholder"),
    }
}

pub fn position_in_line(line: &LineFragment, x: f32) -> Position {
    let local_x = x - line.rect.x;
    for run in &line.glyph_runs {
        if local_x < run.x || local_x > run.x + run.width {
            continue;
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
    if let Some(last) = line.glyph_runs.last() {
        Position::new(last.node_id, last.offset + last.char_advances.len())
    } else {
        Position::new(line.node_id, 0)
    }
}

#[cfg(test)]
mod tests {
    use editor_common::StrExt;
    use editor_model::NodeId;

    use super::*;

    fn make_line(id: NodeId, y: f32, text: &str, char_w: f32) -> LineFragment {
        let n = text.char_count();
        LineFragment {
            node_id: id,
            rect: Rect {
                x: 0.0,
                y,
                width: n as f32 * char_w,
                height: 20.0,
            },
            baseline: 16.0,
            glyph_runs: vec![GlyphRun {
                font_id: 0,
                font_weight: 400,
                font_size: 14.0,
                synthesis: Synthesis::default(),
                color: String::new(),
                background_color: None,
                glyphs: vec![],
                node_id: id,
                offset: 0,
                text: text.into(),
                x: 0.0,
                width: n as f32 * char_w,
                char_advances: vec![char_w; n],
            }],
        }
    }

    #[test]
    fn navigate_to_line_start() {
        let id = NodeId::new();
        let sel = navigate_to(&Fragment::Line(make_line(id, 0.0, "hello", 10.0)), 3.0);

        assert!(sel.is_collapsed());
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn navigate_to_line_middle() {
        let id = NodeId::new();
        let sel = navigate_to(&Fragment::Line(make_line(id, 0.0, "hello", 10.0)), 25.0);

        assert_eq!(sel.head.offset, 3);
    }

    #[test]
    fn navigate_to_atom_selects_range() {
        let parent_id = NodeId::new();
        let atom = Fragment::Atom(AtomFragment {
            node_id: NodeId::new(),
            parent_id,
            index: 2,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
        });
        let sel = navigate_to(&atom, 50.0);

        assert!(!sel.is_collapsed());
        assert_eq!(sel.anchor.offset, 2);
        assert_eq!(sel.head.offset, 3);
    }

    #[test]
    fn position_in_line_at_end() {
        let id = NodeId::new();
        let line = make_line(id, 0.0, "hello", 10.0);
        let pos = position_in_line(&line, 50.0); // past end

        assert_eq!(pos.offset, 5);
    }
}
