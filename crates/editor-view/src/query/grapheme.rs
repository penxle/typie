use editor_state::Affinity;
use editor_state::Position;

use crate::paginate::types::LayoutLine;

type VisualEdge = Option<(f32, usize)>;

fn visual_bounds(line: &LayoutLine) -> (VisualEdge, VisualEdge) {
    let mut first: VisualEdge = None;
    let mut last: VisualEdge = None;

    for run in &line.glyph_runs {
        if first.is_none_or(|(x, _)| run.x < x) {
            first = Some((run.x, run.offset_range.start));
        }

        let end_x = run.x + run.width;
        if last.is_none_or(|(x, _)| end_x > x) {
            last = Some((end_x, run.offset_range.end));
        }
    }

    for gap in &line.tab_gaps {
        if first.is_none_or(|(x, _)| gap.x < x) {
            first = Some((gap.x, gap.offset_index));
        }

        let end_x = gap.x + gap.width;
        if last.is_none_or(|(x, _)| end_x > x) {
            last = Some((end_x, gap.offset_index + 1));
        }
    }

    (first, last)
}

pub(crate) fn last_position_in_line(line: &LayoutLine) -> Position {
    let (_, last) = visual_bounds(line);
    if let Some((_, offset)) = last {
        return Position {
            node: line.node,
            offset,
            affinity: Affinity::Upstream,
        };
    }
    if let Some(range) = &line.offset_range {
        let affinity = if range.start == range.end {
            Affinity::Downstream
        } else {
            Affinity::Upstream
        };
        return Position {
            node: line.node,
            offset: range.end,
            affinity,
        };
    }
    Position::new(line.node, 0)
}

pub(crate) fn first_position_in_line(line: &LayoutLine) -> Position {
    let (first, _) = visual_bounds(line);
    if let Some((_, offset)) = first {
        return Position::new(line.node, offset);
    }
    let offset = line.offset_range.as_ref().map(|r| r.start).unwrap_or(0);
    Position {
        node: line.node,
        offset,
        affinity: Affinity::Downstream,
    }
}

pub(crate) fn x_at_offset(line: &LayoutLine, pos: &Position) -> f32 {
    let raw = x_at_offset_raw(line, pos);
    match line.content_edge_x {
        Some(clamp) if raw > clamp => clamp,
        _ => raw,
    }
}

fn x_at_offset_raw(line: &LayoutLine, pos: &Position) -> f32 {
    for gap in &line.tab_gaps {
        if pos.node == line.node {
            if pos.offset == gap.offset_index {
                return gap.x;
            }
            if pos.offset == gap.offset_index + 1 {
                return gap.x + gap.width;
            }
        }
    }

    if pos.node == line.node {
        for run in &line.glyph_runs {
            let local_offset = pos.offset.saturating_sub(run.offset_range.start);
            let run_cp_count = run.offset_range.len();
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
    }

    if pos.node == line.node
        && let Some(range) = &line.offset_range
    {
        if pos.offset == range.start {
            return line_content_start_x(line);
        }
        if pos.offset == range.end {
            return line_content_end_x(line);
        }
    }

    line_content_end_x(line)
}

fn line_content_start_x(line: &LayoutLine) -> f32 {
    let mut x: Option<f32> = None;
    for run in &line.glyph_runs {
        x = Some(x.map_or(run.x, |current| current.min(run.x)));
    }
    for gap in &line.tab_gaps {
        x = Some(x.map_or(gap.x, |current| current.min(gap.x)));
    }
    x.unwrap_or(line.empty_caret_x)
}

fn line_content_end_x(line: &LayoutLine) -> f32 {
    let mut x: Option<f32> = None;
    for run in &line.glyph_runs {
        let end = run.x + run.width;
        x = Some(x.map_or(end, |current| current.max(end)));
    }
    for gap in &line.tab_gaps {
        let end = gap.x + gap.width;
        x = Some(x.map_or(end, |current| current.max(end)));
    }
    x.unwrap_or(line.empty_caret_x)
}

pub(crate) fn position_at_x(line: &LayoutLine, local_x: f32) -> Position {
    for gap in &line.tab_gaps {
        if local_x >= gap.x && local_x <= gap.x + gap.width {
            let before = local_x < gap.x + gap.width / 2.0;
            let offset = if before {
                gap.offset_index
            } else {
                gap.offset_index + 1
            };
            let last_position = last_position_in_line(line);
            if offset == last_position.offset {
                return last_position;
            }
            return Position {
                node: line.node,
                offset,
                affinity: Affinity::Downstream,
            };
        }
    }

    if line.glyph_runs.is_empty() && line.tab_gaps.is_empty() {
        let offset = line.offset_range.as_ref().map(|r| r.start).unwrap_or(0);
        return Position::new(line.node, offset);
    }

    if local_x <= line_content_start_x(line) {
        return first_position_in_line(line);
    }

    let last_position = last_position_in_line(line);

    if local_x >= line_content_end_x(line) {
        return last_position;
    }

    for run in &line.glyph_runs {
        if local_x < run.x || local_x > run.x + run.width {
            continue;
        }
        let mut acc = run.x;
        let mut cp_offset = 0usize;
        for g in &run.graphemes {
            if local_x < acc + g.advance / 2.0 {
                return Position::new(line.node, run.offset_range.start + cp_offset);
            }
            acc += g.advance;
            cp_offset += g.codepoints as usize;
        }
        let offset = run.offset_range.start + cp_offset;
        if offset == last_position.offset {
            return last_position;
        }
        return Position::new(line.node, offset);
    }

    last_position
}

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;
    use editor_state::Affinity;
    use editor_state::Position;

    use crate::glyph_run::GlyphRun;
    use crate::glyph_run::{GraphemeSpan, Synthesis, TextDecoration};
    use crate::measure::text::measure::TabGap;
    use crate::paginate::types::LayoutLine;

    use super::*;

    fn gs(advance: f32, codepoints: u8) -> GraphemeSpan {
        GraphemeSpan {
            advance,
            codepoints,
        }
    }

    fn run(offset_range: std::ops::Range<usize>, x: f32, graphemes: Vec<GraphemeSpan>) -> GlyphRun {
        let width = graphemes.iter().map(|g| g.advance).sum();
        GlyphRun {
            family_id: 0,
            weight: 400,
            font_size: 16.0,
            synthesis: Synthesis::default(),
            color: String::new(),
            background_color: None,
            glyphs: vec![],
            decoration: TextDecoration::default(),
            offset_range,
            link: None,
            text: String::new(),
            x,
            width,
            graphemes,
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        }
    }

    fn line(
        node: Dot,
        offset_range: Option<std::ops::Range<usize>>,
        runs: Vec<GlyphRun>,
        tab_gaps: Vec<TabGap>,
        empty_caret_x: f32,
        content_edge_x: Option<f32>,
    ) -> LayoutLine {
        LayoutLine {
            measured: std::sync::Arc::new(crate::measure::text::measure::MeasuredLine {
                height: 0.0,
                node,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: runs,
                ruby_annotations: vec![],
                empty_caret_x,
                offset_range,
                tab_gaps,
                is_phantom: false,
                content_edge_x,
            }),
        }
    }

    fn node() -> Dot {
        Dot::new(1, 1)
    }

    #[test]
    fn x_at_offset_start_middle_end() {
        let n = node();
        let l = line(
            n,
            None,
            vec![run(0..5, 0.0, vec![gs(10.0, 1); 5])],
            vec![],
            0.0,
            None,
        );
        assert_eq!(x_at_offset(&l, &Position::new(n, 0)), 0.0);
        assert_eq!(x_at_offset(&l, &Position::new(n, 3)), 30.0);
        assert_eq!(x_at_offset(&l, &Position::new(n, 5)), 50.0);
    }

    #[test]
    fn x_at_offset_with_run_x() {
        let n = node();
        let l = line(
            n,
            None,
            vec![run(1..6, 60.0, vec![gs(10.0, 1); 5])],
            vec![],
            0.0,
            None,
        );
        assert_eq!(x_at_offset(&l, &Position::new(n, 1)), 60.0);
    }

    #[test]
    fn x_at_offset_snaps_to_grapheme_boundary() {
        let n = node();
        let l = line(
            n,
            None,
            vec![run(0..5, 0.0, vec![gs(20.0, 3), gs(10.0, 1), gs(10.0, 1)])],
            vec![],
            0.0,
            None,
        );
        assert_eq!(x_at_offset(&l, &Position::new(n, 1)), 0.0);
        assert_eq!(x_at_offset(&l, &Position::new(n, 3)), 20.0);
        assert_eq!(x_at_offset(&l, &Position::new(n, 4)), 30.0);
    }

    #[test]
    fn position_at_x_round_trip() {
        let n = node();
        let l = line(
            n,
            None,
            vec![run(0..5, 0.0, vec![gs(10.0, 1); 5])],
            vec![],
            0.0,
            None,
        );

        for k in 0..=5usize {
            let x = x_at_offset(&l, &Position::new(n, k));
            let pos = position_at_x(&l, x);
            assert_eq!(pos.node, n, "round-trip node at k={k}");
            assert_eq!(pos.offset, k, "round-trip offset at k={k}");
        }

        let before = position_at_x(&l, -5.0);
        assert_eq!(before.node, n);
        assert_eq!(before.offset, 0);

        let after = position_at_x(&l, 100.0);
        assert_eq!(after.node, n);
        assert_eq!(after.offset, 5);
        assert_eq!(after.affinity, Affinity::Upstream);
    }

    #[test]
    fn position_at_x_handles_tab_only_line_edges() {
        let n = node();
        let l = line(
            n,
            Some(0..2),
            vec![],
            vec![
                TabGap {
                    offset_index: 0,
                    x: 0.0,
                    width: 40.0,
                    link: None,
                },
                TabGap {
                    offset_index: 1,
                    x: 40.0,
                    width: 40.0,
                    link: None,
                },
            ],
            0.0,
            None,
        );

        let before = position_at_x(&l, -10.0);
        assert_eq!(before.node, n);
        assert_eq!(before.offset, 0);

        let after = position_at_x(&l, 100.0);
        assert_eq!(after.node, n);
        assert_eq!(after.offset, 2);
        assert_eq!(after.affinity, Affinity::Upstream);

        let inside_last_gap_right_half = position_at_x(&l, 70.0);
        assert_eq!(inside_last_gap_right_half.node, n);
        assert_eq!(inside_last_gap_right_half.offset, 2);
        assert_eq!(inside_last_gap_right_half.affinity, Affinity::Upstream);
    }

    #[test]
    fn position_before_leading_tab_with_following_text_lands_at_line_start() {
        let n = node();
        let l = line(
            n,
            Some(0..3),
            vec![run(2..3, 100.0, vec![gs(10.0, 1)])],
            vec![
                TabGap {
                    offset_index: 0,
                    x: 20.0,
                    width: 40.0,
                    link: None,
                },
                TabGap {
                    offset_index: 1,
                    x: 60.0,
                    width: 40.0,
                    link: None,
                },
            ],
            20.0,
            None,
        );

        let before = position_at_x(&l, 10.0);
        assert_eq!(before.node, n);
        assert_eq!(before.offset, 0);
    }

    #[test]
    fn last_position_in_line_cases() {
        let n = node();

        let l_run = line(
            n,
            None,
            vec![run(0..2, 0.0, vec![gs(10.0, 1); 2])],
            vec![],
            0.0,
            None,
        );
        let pos = last_position_in_line(&l_run);
        assert_eq!(pos.node, n);
        assert_eq!(pos.offset, 2);
        assert_eq!(pos.affinity, Affinity::Upstream);

        let gap = TabGap {
            offset_index: 2,
            x: 20.0,
            width: 30.0,
            link: None,
        };
        let l_tab = line(
            n,
            None,
            vec![run(0..2, 0.0, vec![gs(10.0, 1); 2])],
            vec![gap],
            0.0,
            None,
        );
        let pos = last_position_in_line(&l_tab);
        assert_eq!(pos.node, n);
        assert_eq!(pos.offset, 3);
        assert_eq!(pos.affinity, Affinity::Upstream);

        let l_empty_degen = line(n, Some(3..3), vec![], vec![], 0.0, None);
        let pos = last_position_in_line(&l_empty_degen);
        assert_eq!(pos.node, n);
        assert_eq!(pos.offset, 3);
        assert_eq!(pos.affinity, Affinity::Downstream);

        let l_fully_empty = line(n, None, vec![], vec![], 0.0, None);
        let pos = last_position_in_line(&l_fully_empty);
        assert_eq!(pos.node, n);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn multi_run_selection() {
        let n = node();
        let l = line(
            n,
            None,
            vec![
                run(0..3, 0.0, vec![gs(10.0, 1); 3]),
                run(3..6, 30.0, vec![gs(10.0, 1); 3]),
            ],
            vec![],
            0.0,
            None,
        );

        let x = x_at_offset(&l, &Position::new(n, 4));
        assert_eq!(x, 40.0);

        let pos = position_at_x(&l, 45.0);
        assert_eq!(pos.node, n);
        assert_eq!(pos.offset, 5);
    }
}
