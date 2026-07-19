use crate::backend::cpu::CpuSink;
use crate::damage::{IRect, merge_damage};
use crate::display_list::DisplayList;
use crate::sink::RenderSink;

pub fn diff(prev: &DisplayList, new: &DisplayList, full: IRect) -> Vec<IRect> {
    use hashbrown::HashMap;
    let mut buckets: HashMap<u64, Vec<usize>> = HashMap::new();
    for (i, p) in prev.primitives.iter().enumerate() {
        buckets.entry(p.key).or_default().push(i);
    }
    let mut prev_used = vec![false; prev.primitives.len()];
    let mut raw: Vec<IRect> = Vec::new();
    let mut survivor_prev_idx: Vec<usize> = Vec::new();
    let mut survivor_bounds: Vec<IRect> = Vec::new();

    for np in &new.primitives {
        let mut matched: Option<usize> = None;
        if let Some(cands) = buckets.get(&np.key) {
            for &pi in cands {
                if !prev_used[pi] && prev.primitives[pi].same_content(np) {
                    prev_used[pi] = true;
                    matched = Some(pi);
                    break;
                }
            }
        }
        match matched {
            None => raw.push(np.bounds),
            Some(pi) => {
                survivor_prev_idx.push(pi);
                survivor_bounds.push(np.bounds);
            }
        }
    }
    for (i, used) in prev_used.iter().enumerate() {
        if !used {
            raw.push(prev.primitives[i].bounds);
        }
    }

    let mut max_pi = 0usize;
    let mut first = true;
    for (k, &pi) in survivor_prev_idx.iter().enumerate() {
        if first {
            max_pi = pi;
            first = false;
        } else if pi < max_pi {
            raw.push(survivor_bounds[k]);
        } else {
            max_pi = pi;
        }
    }

    if raw.is_empty() {
        return Vec::new();
    }
    merge_damage(&raw, full)
}

pub fn replay(dl: &DisplayList, clip: IRect, sink: &mut dyn RenderSink) {
    use crate::display_list::PrimPayload::*;
    for p in &dl.primitives {
        if p.bounds.intersect(clip).is_none() {
            continue;
        }
        match &p.payload {
            FillRect {
                rect,
                color,
                transform,
            } => sink.fill_rect(*rect, *color, *transform),
            FillPath {
                path,
                color,
                transform,
            } => sink.fill_path(path, *color, *transform),
            StrokePath {
                path,
                color,
                stroke,
                transform,
            } => sink.stroke_path(path, *color, stroke, *transform),
            Glyph {
                image,
                dst_x,
                dst_y,
                ..
            } => sink.draw_glyph(image, *dst_x, *dst_y),
        }
    }
}

pub fn render_incremental(
    prev: Option<&DisplayList>,
    new: &DisplayList,
    sink: &mut CpuSink,
    full: IRect,
) -> Vec<IRect> {
    let damage = match prev {
        None => vec![full],
        Some(prev) => diff(prev, new, full),
    };
    for &r in &damage {
        sink.clear_rect(r);
        sink.set_clip(Some(r));
        replay(new, r, sink);
    }
    sink.set_clip(None);
    damage
}

/// Rasters the display-list content of device rect `r` into `scratch` at origin.
/// `scratch` must be at least `r.width() x r.height()`; offsets are integral so
/// the result is byte-identical to the same subregion of a full-page raster.
pub fn raster_rect(dl: &DisplayList, r: IRect, scratch: &mut CpuSink) {
    let local = IRect {
        x0: 0,
        y0: 0,
        x1: r.width(),
        y1: r.height(),
    };
    scratch.set_clip(None);
    scratch.clear_rect(local);
    scratch.set_clip(Some(local));
    let mut sink = crate::translate::TranslatedSink::new(scratch, -(r.x0 as f32), -(r.y0 as f32));
    replay(dl, r, &mut sink);
    scratch.set_clip(None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::IRect;
    use crate::display_list::DisplayListRecorder;
    use crate::sink::RenderSink;
    use crate::types::{Color, Transform};

    fn full() -> IRect {
        IRect {
            x0: 0,
            y0: 0,
            x1: 200,
            y1: 200,
        }
    }

    fn dl_with(rects: &[(f32, f32, u8)]) -> crate::display_list::DisplayList {
        let mut rec = DisplayListRecorder::new(full());
        for &(x, y, c) in rects {
            rec.fill_rect(
                editor_common::Rect::from_xywh(x, y, 10.0, 10.0),
                Color::new(c, 0, 0, 255),
                Transform::IDENTITY,
            );
        }
        rec.into_list()
    }

    #[test]
    fn no_change_empty_damage() {
        let a = dl_with(&[(10.0, 10.0, 1), (50.0, 50.0, 2)]);
        let b = dl_with(&[(10.0, 10.0, 1), (50.0, 50.0, 2)]);
        assert!(diff(&a, &b, full()).is_empty());
    }

    #[test]
    fn changed_color_damages_that_rect() {
        let a = dl_with(&[(10.0, 10.0, 1)]);
        let b = dl_with(&[(10.0, 10.0, 9)]);
        let d = diff(&a, &b, full());
        assert_eq!(d.len(), 1);
        assert!(d[0].x0 <= 10 && d[0].x1 >= 20);
    }

    #[test]
    fn moved_rect_damages_old_and_new() {
        let a = dl_with(&[(10.0, 10.0, 1)]);
        let b = dl_with(&[(100.0, 100.0, 1)]);
        let d = diff(&a, &b, full());
        let covers = |rs: &[IRect], px: i32, py: i32| {
            rs.iter()
                .any(|r| r.x0 <= px && px < r.x1 && r.y0 <= py && py < r.y1)
        };
        assert!(covers(&d, 12, 12));
        assert!(covers(&d, 102, 102));
    }

    #[test]
    fn zorder_swap_of_overlapping_damages() {
        let a = dl_with(&[(10.0, 10.0, 1), (12.0, 12.0, 2)]);
        let b = {
            let mut rec = DisplayListRecorder::new(full());
            rec.fill_rect(
                editor_common::Rect::from_xywh(12.0, 12.0, 10.0, 10.0),
                Color::new(2, 0, 0, 255),
                Transform::IDENTITY,
            );
            rec.fill_rect(
                editor_common::Rect::from_xywh(10.0, 10.0, 10.0, 10.0),
                Color::new(1, 0, 0, 255),
                Transform::IDENTITY,
            );
            rec.into_list()
        };
        let d = diff(&a, &b, full());
        let covers = |rs: &[IRect], px: i32, py: i32| {
            rs.iter()
                .any(|r| r.x0 <= px && px < r.x1 && r.y0 <= py && py < r.y1)
        };
        assert!(
            covers(&d, 15, 15),
            "damage must cover the overlap region of the swapped primitives"
        );
        assert!(
            !covers(&d, 90, 90),
            "z-order damage must stay local, not full-page"
        );
    }

    #[test]
    fn raster_rect_matches_full_subregion_fixture() {
        let dl = dl_with(&[(10.0, 10.0, 1), (50.0, 50.0, 2)]);
        let mut full_sink = CpuSink::new(200, 200);
        render_incremental(None, &dl, &mut full_sink, full());

        let r = IRect {
            x0: 8,
            y0: 8,
            x1: 64,
            y1: 64,
        };
        let mut scratch = CpuSink::new(56, 56);
        raster_rect(&dl, r, &mut scratch);

        let mut expect = vec![0u8; 56 * 56 * 4];
        full_sink.read_back_rect(&mut expect, 56 * 4, r);
        let mut got = vec![0u8; 56 * 56 * 4];
        scratch.read_back_rect(
            &mut got,
            56 * 4,
            IRect {
                x0: 0,
                y0: 0,
                x1: 56,
                y1: 56,
            },
        );
        assert_eq!(got, expect);
    }

    #[test]
    fn diff_resolves_forced_key_collision() {
        use crate::display_list::{DisplayList, PrimPayload, Primitive};
        let mk = |rx: f32| Primitive {
            bounds: IRect {
                x0: rx as i32,
                y0: 0,
                x1: rx as i32 + 10,
                y1: 10,
            },
            key: 99,
            payload: PrimPayload::FillRect {
                rect: editor_common::Rect::from_xywh(rx, 0.0, 10.0, 10.0),
                color: Color::new(1, 2, 3, 255),
                transform: Transform::IDENTITY,
            },
        };
        let prev = DisplayList {
            primitives: vec![mk(10.0)],
        };
        let new = DisplayList {
            primitives: vec![mk(100.0)],
        };
        let d = diff(&prev, &new, full());
        let covers = |rs: &[IRect], px: i32, py: i32| {
            rs.iter()
                .any(|r| r.x0 <= px && px < r.x1 && r.y0 <= py && py < r.y1)
        };
        assert!(covers(&d, 12, 2), "collided-old content must be damaged");
        assert!(covers(&d, 102, 2), "collided-new content must be damaged");
    }

    /// A parallelogram whose long edges run diagonally (slope ~1) from
    /// `(x0, y0)` to `(x1, y1)`, thickened by `t` along y. Diagonal edges are
    /// never axis-aligned, so they yield partial-coverage (AA) pixels across
    /// every row they cross -- unlike an axis-aligned rect, which is only
    /// antialiased at its own fractional edge columns.
    fn diag_path(x0: f32, y0: f32, x1: f32, y1: f32, t: f32) -> crate::types::Path {
        use crate::types::PathElement::*;
        crate::types::Path {
            elements: vec![
                MoveTo { x: x0, y: y0 - t },
                LineTo { x: x1, y: y1 - t },
                LineTo { x: x1, y: y1 + t },
                LineTo { x: x0, y: y0 + t },
                Close,
            ],
        }
    }

    #[test]
    fn raster_rect_matches_full_subregion_at_high_coordinate_boundaries() {
        let page = IRect {
            x0: 0,
            y0: 0,
            x1: 4096,
            y1: 4096,
        };
        let mut rec = DisplayListRecorder::new(page);
        rec.fill_rect(
            editor_common::Rect::from_xywh(2046.5, 1022.25, 5.0, 5.0),
            Color::new(9, 0, 0, 255),
            Transform::IDENTITY,
        );
        rec.fill_path(
            &diag_path(970.15, 970.65, 1130.35, 1130.85, 10.0),
            Color::new(1, 0, 0, 255),
            Transform::IDENTITY,
        );
        rec.fill_path(
            &diag_path(1994.15, 1994.65, 2154.35, 2154.85, 10.0),
            Color::new(2, 0, 0, 255),
            Transform::IDENTITY,
        );
        rec.fill_path(
            &diag_path(3986.15, 3986.65, 4095.6, 4095.95, 10.0),
            Color::new(3, 0, 0, 255),
            Transform::IDENTITY,
        );
        let dl = rec.into_list();

        let mut full_sink = CpuSink::new(4096, 4096);
        render_incremental(None, &dl, &mut full_sink, page);

        let rects = [
            IRect {
                x0: 1016,
                y0: 1016,
                x1: 1088,
                y1: 1088,
            },
            IRect {
                x0: 2040,
                y0: 2040,
                x1: 2112,
                y1: 2112,
            },
            IRect {
                x0: 4032,
                y0: 4032,
                x1: 4096,
                y1: 4096,
            },
        ];
        for r in rects {
            let (rw, rh) = (r.width() as usize, r.height() as usize);
            let mut expect = vec![0u8; rw * rh * 4];
            full_sink.read_back_rect(&mut expect, rw * 4, r);

            assert!(
                expect.chunks_exact(4).any(|px| px[3] != 0),
                "expected non-zero pixels near boundary rect {r:?}, oracle would vacuously pass"
            );
            assert!(
                expect.chunks_exact(4).any(|px| px[3] != 0 && px[3] != 255),
                "expected AA (0<alpha<255) pixels near boundary rect {r:?}, oracle would vacuously pass"
            );

            let mut scratch = CpuSink::new(r.width() as u16, r.height() as u16);
            raster_rect(&dl, r, &mut scratch);
            let mut got = vec![0u8; rw * rh * 4];
            scratch.read_back_rect(
                &mut got,
                rw * 4,
                IRect {
                    x0: 0,
                    y0: 0,
                    x1: r.width(),
                    y1: r.height(),
                },
            );
            assert_eq!(got, expect, "high-coordinate subregion mismatch for {r:?}");
        }
    }
}
