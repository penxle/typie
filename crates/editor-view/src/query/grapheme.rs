use editor_state::Position;

use crate::glyph_run::GlyphRun;
use crate::paginate::LayoutLine;

pub fn run_codepoint_count(run: &GlyphRun) -> usize {
    run.graphemes.iter().map(|g| g.codepoints as usize).sum()
}

pub fn last_position_in_line(line: &LayoutLine) -> Position {
    if let Some(run) = line.glyph_runs.last() {
        Position::new(run.node_id, run.offset + run_codepoint_count(run))
    } else {
        Position::new(line.node_id, 0)
    }
}

pub fn x_at_offset(line: &LayoutLine, pos: &Position) -> f32 {
    for run in &line.glyph_runs {
        if run.node_id != pos.node_id {
            continue;
        }

        let local_offset = pos.offset.saturating_sub(run.offset);
        let run_cp_count = run_codepoint_count(run);
        if local_offset > run_cp_count {
            continue;
        }

        let mut acc = 0usize;
        let mut x = run.x;
        for g in &run.graphemes {
            let cp = g.codepoints as usize;
            if acc + cp > local_offset {
                break;
            }
            acc += cp;
            x += g.advance;
        }
        return x;
    }

    line.glyph_runs
        .last()
        .map(|r| r.x + r.width)
        .unwrap_or(line.text_indent)
}

pub fn position_at_x(line: &LayoutLine, local_x: f32) -> Position {
    if line.glyph_runs.is_empty() {
        return Position::new(line.node_id, 0);
    }

    let first = &line.glyph_runs[0];
    let last = &line.glyph_runs[line.glyph_runs.len() - 1];

    if local_x <= first.x {
        return Position::new(first.node_id, first.offset);
    }

    if local_x >= last.x + last.width {
        return Position::new(last.node_id, last.offset + run_codepoint_count(last));
    }

    for run in &line.glyph_runs {
        if local_x < run.x || local_x > run.x + run.width {
            continue;
        }
        let mut acc = run.x;
        let mut cp_offset = 0usize;
        for g in &run.graphemes {
            if local_x < acc + g.advance / 2.0 {
                return Position::new(run.node_id, run.offset + cp_offset);
            }
            acc += g.advance;
            cp_offset += g.codepoints as usize;
        }
        return Position::new(run.node_id, run.offset + cp_offset);
    }

    Position::new(last.node_id, last.offset + run_codepoint_count(last))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph_run::GraphemeSpan;
    use editor_model::NodeId;

    fn gs(advance: f32, codepoints: u8) -> GraphemeSpan {
        GraphemeSpan {
            advance,
            codepoints,
        }
    }

    fn ascii_spans(count: usize, advance: f32) -> Vec<GraphemeSpan> {
        vec![gs(advance, 1); count]
    }

    #[test]
    fn run_codepoint_count_ascii() {
        let id = NodeId::new();
        let run = GlyphRun::make_test_run(id, 0, "hello", 0.0, ascii_spans(5, 10.0));
        assert_eq!(run_codepoint_count(&run), 5);
    }

    #[test]
    fn run_codepoint_count_multi_cp_graphemes() {
        let id = NodeId::new();
        let run = GlyphRun::make_test_run(
            id,
            0,
            "\u{1F468}\u{200D}\u{1F469}ab",
            0.0,
            vec![gs(20.0, 3), gs(10.0, 1), gs(10.0, 1)],
        );
        assert_eq!(run_codepoint_count(&run), 5);
    }

    #[test]
    fn x_at_offset_start() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![GlyphRun::make_test_run(
                id,
                0,
                "hello",
                0.0,
                ascii_spans(5, 10.0),
            )],
            text_indent: 0.0,
        };
        assert_eq!(x_at_offset(&line, &Position::new(id, 0)), 0.0);
    }

    #[test]
    fn x_at_offset_middle() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![GlyphRun::make_test_run(
                id,
                0,
                "hello",
                0.0,
                ascii_spans(5, 10.0),
            )],
            text_indent: 0.0,
        };
        assert_eq!(x_at_offset(&line, &Position::new(id, 3)), 30.0);
    }

    #[test]
    fn x_at_offset_end() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![GlyphRun::make_test_run(
                id,
                0,
                "hello",
                0.0,
                ascii_spans(5, 10.0),
            )],
            text_indent: 0.0,
        };
        assert_eq!(x_at_offset(&line, &Position::new(id, 5)), 50.0);
    }

    #[test]
    fn x_at_offset_snaps_to_grapheme_boundary() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![GlyphRun::make_test_run(
                id,
                0,
                "\u{1F468}\u{200D}\u{1F469}ab",
                0.0,
                vec![gs(20.0, 3), gs(10.0, 1), gs(10.0, 1)],
            )],
            text_indent: 0.0,
        };
        // offset 1 is inside the first grapheme (3 codepoints) => snaps to start
        assert_eq!(x_at_offset(&line, &Position::new(id, 1)), 0.0);
        // offset 3 = after the 3-codepoint grapheme
        assert_eq!(x_at_offset(&line, &Position::new(id, 3)), 20.0);
        // offset 4 = after 'a'
        assert_eq!(x_at_offset(&line, &Position::new(id, 4)), 30.0);
    }

    #[test]
    fn x_at_offset_with_run_x() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![GlyphRun::make_test_run(
                id,
                0,
                "hi",
                50.0,
                ascii_spans(2, 10.0),
            )],
            text_indent: 0.0,
        };
        assert_eq!(x_at_offset(&line, &Position::new(id, 1)), 60.0);
    }

    #[test]
    fn position_at_x_start() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![GlyphRun::make_test_run(
                id,
                0,
                "hello",
                0.0,
                ascii_spans(5, 10.0),
            )],
            text_indent: 0.0,
        };
        let pos = position_at_x(&line, -5.0);
        assert_eq!(pos.node_id, id);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn position_at_x_end() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![GlyphRun::make_test_run(
                id,
                0,
                "hello",
                0.0,
                ascii_spans(5, 10.0),
            )],
            text_indent: 0.0,
        };
        let pos = position_at_x(&line, 100.0);
        assert_eq!(pos.node_id, id);
        assert_eq!(pos.offset, 5);
    }

    #[test]
    fn position_at_x_midpoint_snaps() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![GlyphRun::make_test_run(
                id,
                0,
                "hello",
                0.0,
                ascii_spans(5, 10.0),
            )],
            text_indent: 0.0,
        };
        // x=4 is < 5.0 (half of first advance), so snaps to offset 0
        let pos = position_at_x(&line, 4.0);
        assert_eq!(pos.offset, 0);
        // x=6 is >= 5.0, so snaps to offset 1
        let pos = position_at_x(&line, 6.0);
        assert_eq!(pos.offset, 1);
    }

    #[test]
    fn position_at_x_multi_cp_grapheme() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![GlyphRun::make_test_run(
                id,
                0,
                "\u{1F468}\u{200D}\u{1F469}ab",
                0.0,
                vec![gs(20.0, 3), gs(10.0, 1), gs(10.0, 1)],
            )],
            text_indent: 0.0,
        };
        // x=9 is < 10.0 (half of 20.0 advance) => offset 0
        let pos = position_at_x(&line, 9.0);
        assert_eq!(pos.offset, 0);
        // x=11 is >= 10.0 => offset 3 (past the 3-codepoint grapheme)
        let pos = position_at_x(&line, 11.0);
        assert_eq!(pos.offset, 3);
        // x=25 is >= 25.0 (20 + 5) => offset 4
        let pos = position_at_x(&line, 25.0);
        assert_eq!(pos.offset, 4);
    }

    #[test]
    fn position_at_x_empty_line() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![],
            text_indent: 0.0,
        };
        let pos = position_at_x(&line, 50.0);
        assert_eq!(pos.node_id, id);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn last_position_in_line_with_runs() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![GlyphRun::make_test_run(
                id,
                0,
                "hello",
                0.0,
                ascii_spans(5, 10.0),
            )],
            text_indent: 0.0,
        };
        let pos = last_position_in_line(&line);
        assert_eq!(pos.node_id, id);
        assert_eq!(pos.offset, 5);
    }

    #[test]
    fn last_position_in_line_empty() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![],
            text_indent: 0.0,
        };
        let pos = last_position_in_line(&line);
        assert_eq!(pos.node_id, id);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn multi_run_x_at_offset() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let line = LayoutLine {
            node_id: id1,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![
                GlyphRun::make_test_run(id1, 0, "ab", 0.0, ascii_spans(2, 10.0)),
                GlyphRun::make_test_run(id2, 0, "cd", 20.0, ascii_spans(2, 10.0)),
            ],
            text_indent: 0.0,
        };
        assert_eq!(x_at_offset(&line, &Position::new(id2, 1)), 30.0);
    }

    #[test]
    fn multi_run_position_at_x() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let line = LayoutLine {
            node_id: id1,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![
                GlyphRun::make_test_run(id1, 0, "ab", 0.0, ascii_spans(2, 10.0)),
                GlyphRun::make_test_run(id2, 0, "cd", 20.0, ascii_spans(2, 10.0)),
            ],
            text_indent: 0.0,
        };
        let pos = position_at_x(&line, 25.0);
        assert_eq!(pos.node_id, id2);
        assert_eq!(pos.offset, 1);
    }

    #[test]
    fn x_at_offset_empty_line_with_text_indent() {
        let id = NodeId::new();
        let line = LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![],
            text_indent: 32.0,
        };
        assert_eq!(x_at_offset(&line, &Position::new(id, 0)), 32.0);
    }
}
