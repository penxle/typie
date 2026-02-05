use skrifa::outline::OutlinePen;
use tiny_skia::PathBuilder;

enum PenCmd {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    CurveTo(f32, f32, f32, f32, f32, f32),
    Close,
}

pub(crate) struct TinySkiaPen {
    commands: Vec<PenCmd>,
}

impl TinySkiaPen {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn finish(self) -> Option<tiny_skia::Path> {
        build_path(&self.commands, None)
    }

    pub fn finish_emboldened(self, x_strength: f32, y_strength: f32) -> Option<tiny_skia::Path> {
        if x_strength <= 0.0 && y_strength <= 0.0 {
            return self.finish();
        }

        let mut points = extract_points(&self.commands);
        let contours = extract_contours(&self.commands, &points);

        embolden_outline(&mut points, &contours, x_strength, y_strength);

        build_path(&self.commands, Some(&points))
    }

    pub fn measure_width_at_mid_y(&self) -> Option<f32> {
        let segments = flatten_commands(&self.commands);
        let (min_y, max_y) = segments.iter().fold((f32::MAX, f32::MIN), |(mn, mx), s| {
            (mn.min(s.0.1).min(s.1.1), mx.max(s.0.1).max(s.1.1))
        });
        if max_y <= min_y {
            return None;
        }
        let scan_y = (min_y + max_y) * 0.5;
        let mut crossings = Vec::new();
        for &((x0, y0), (x1, y1)) in &segments {
            if (y0 <= scan_y && scan_y < y1) || (y1 <= scan_y && scan_y < y0) {
                let t = (scan_y - y0) / (y1 - y0);
                crossings.push(x0 + t * (x1 - x0));
            }
        }
        crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        min_adjacent_span(&crossings)
    }

    pub fn measure_height_at_mid_x(&self) -> Option<f32> {
        let segments = flatten_commands(&self.commands);
        let (min_x, max_x) = segments.iter().fold((f32::MAX, f32::MIN), |(mn, mx), s| {
            (mn.min(s.0.0).min(s.1.0), mx.max(s.0.0).max(s.1.0))
        });
        if max_x <= min_x {
            return None;
        }
        let scan_x = (min_x + max_x) * 0.5;
        let mut crossings = Vec::new();
        for &((x0, y0), (x1, y1)) in &segments {
            if (x0 <= scan_x && scan_x < x1) || (x1 <= scan_x && scan_x < x0) {
                let t = (scan_x - x0) / (x1 - x0);
                crossings.push(y0 + t * (y1 - y0));
            }
        }
        crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        min_adjacent_span(&crossings)
    }
}

impl OutlinePen for TinySkiaPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.commands.push(PenCmd::MoveTo(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.commands.push(PenCmd::LineTo(x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.commands.push(PenCmd::QuadTo(x1, y1, x, y));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.commands.push(PenCmd::CurveTo(x1, y1, x2, y2, x, y));
    }

    fn close(&mut self) {
        self.commands.push(PenCmd::Close);
    }
}

fn extract_points(commands: &[PenCmd]) -> Vec<(f32, f32)> {
    let mut points = Vec::new();
    for cmd in commands {
        match *cmd {
            PenCmd::MoveTo(x, y) | PenCmd::LineTo(x, y) => {
                points.push((x, y));
            }
            PenCmd::QuadTo(x1, y1, x, y) => {
                points.push((x1, y1));
                points.push((x, y));
            }
            PenCmd::CurveTo(x1, y1, x2, y2, x, y) => {
                points.push((x1, y1));
                points.push((x2, y2));
                points.push((x, y));
            }
            PenCmd::Close => {}
        }
    }
    points
}

fn extract_contours(commands: &[PenCmd], points: &[(f32, f32)]) -> Vec<(usize, usize)> {
    let mut contours = Vec::new();
    let mut contour_start: usize = 0;
    let mut pt_idx: usize = 0;

    for cmd in commands {
        match *cmd {
            PenCmd::MoveTo(_, _) => {
                contour_start = pt_idx;
                pt_idx += 1;
            }
            PenCmd::LineTo(_, _) => pt_idx += 1,
            PenCmd::QuadTo(_, _, _, _) => pt_idx += 2,
            PenCmd::CurveTo(_, _, _, _, _, _) => pt_idx += 3,
            PenCmd::Close => {
                if contour_start < pt_idx && pt_idx <= points.len() {
                    contours.push((contour_start, pt_idx - 1));
                }
            }
        }
    }
    contours
}

fn build_path(commands: &[PenCmd], points: Option<&[(f32, f32)]>) -> Option<tiny_skia::Path> {
    let mut builder = PathBuilder::new();
    let mut pt_idx = 0;

    for cmd in commands {
        match *cmd {
            PenCmd::MoveTo(x, y) => {
                let (x, y) = points.map_or((x, y), |p| p[pt_idx]);
                builder.move_to(x, y);
                pt_idx += 1;
            }
            PenCmd::LineTo(x, y) => {
                let (x, y) = points.map_or((x, y), |p| p[pt_idx]);
                builder.line_to(x, y);
                pt_idx += 1;
            }
            PenCmd::QuadTo(x1, y1, x, y) => {
                let (x1, y1) = points.map_or((x1, y1), |p| p[pt_idx]);
                let (x, y) = points.map_or((x, y), |p| p[pt_idx + 1]);
                builder.quad_to(x1, y1, x, y);
                pt_idx += 2;
            }
            PenCmd::CurveTo(x1, y1, x2, y2, x, y) => {
                let (x1, y1) = points.map_or((x1, y1), |p| p[pt_idx]);
                let (x2, y2) = points.map_or((x2, y2), |p| p[pt_idx + 1]);
                let (x, y) = points.map_or((x, y), |p| p[pt_idx + 2]);
                builder.cubic_to(x1, y1, x2, y2, x, y);
                pt_idx += 3;
            }
            PenCmd::Close => builder.close(),
        }
    }
    builder.finish()
}

fn outline_orientation(points: &[(f32, f32)], contours: &[(usize, usize)]) -> i32 {
    let mut area: f64 = 0.0;

    for &(first, last) in contours {
        let (mut prev_x, mut prev_y) = points[last];
        for n in first..=last {
            let (cur_x, cur_y) = points[n];
            area += (cur_y - prev_y) as f64 * (cur_x + prev_x) as f64;
            prev_x = cur_x;
            prev_y = cur_y;
        }
    }

    if area > 0.0 {
        -1
    } else if area < 0.0 {
        1
    } else {
        0
    }
}

fn vec_normalize(x: &mut f32, y: &mut f32) -> f32 {
    let len = (*x * *x + *y * *y).sqrt();
    if len > 0.0 {
        *x /= len;
        *y /= len;
    }
    len
}

fn flatten_commands(commands: &[PenCmd]) -> Vec<((f32, f32), (f32, f32))> {
    const SUBDIVISIONS: usize = 32;
    let mut segments = Vec::new();
    let mut cx = 0.0f32;
    let mut cy = 0.0f32;

    for cmd in commands {
        match *cmd {
            PenCmd::MoveTo(x, y) => {
                cx = x;
                cy = y;
            }
            PenCmd::LineTo(x, y) => {
                segments.push(((cx, cy), (x, y)));
                cx = x;
                cy = y;
            }
            PenCmd::QuadTo(x1, y1, x2, y2) => {
                let (px, py) = (cx, cy);
                for i in 1..=SUBDIVISIONS {
                    let t = i as f32 / SUBDIVISIONS as f32;
                    let inv = 1.0 - t;
                    let nx = inv * inv * px + 2.0 * inv * t * x1 + t * t * x2;
                    let ny = inv * inv * py + 2.0 * inv * t * y1 + t * t * y2;
                    segments.push(((cx, cy), (nx, ny)));
                    cx = nx;
                    cy = ny;
                }
            }
            PenCmd::CurveTo(x1, y1, x2, y2, x3, y3) => {
                let (px, py) = (cx, cy);
                for i in 1..=SUBDIVISIONS {
                    let t = i as f32 / SUBDIVISIONS as f32;
                    let inv = 1.0 - t;
                    let nx = inv * inv * inv * px
                        + 3.0 * inv * inv * t * x1
                        + 3.0 * inv * t * t * x2
                        + t * t * t * x3;
                    let ny = inv * inv * inv * py
                        + 3.0 * inv * inv * t * y1
                        + 3.0 * inv * t * t * y2
                        + t * t * t * y3;
                    segments.push(((cx, cy), (nx, ny)));
                    cx = nx;
                    cy = ny;
                }
            }
            PenCmd::Close => {}
        }
    }

    segments
}

fn min_adjacent_span(sorted: &[f32]) -> Option<f32> {
    if sorted.len() < 2 {
        return None;
    }
    let mut min_span = f32::MAX;
    for pair in sorted.chunks_exact(2) {
        let span = (pair[1] - pair[0]).abs();
        if span > 0.0 && span < min_span {
            min_span = span;
        }
    }
    if min_span < f32::MAX {
        Some(min_span)
    } else {
        None
    }
}

fn embolden_outline(
    points: &mut [(f32, f32)],
    contours: &[(usize, usize)],
    x_strength: f32,
    y_strength: f32,
) {
    let x_strength = x_strength / 2.0;
    let y_strength = y_strength / 2.0;

    if x_strength == 0.0 && y_strength == 0.0 {
        return;
    }

    let orientation = outline_orientation(points, contours);
    if orientation == 0 {
        return;
    }

    for &(first, last) in contours {
        if first > last {
            continue;
        }

        let mut l_in: f32 = 0.0;
        let mut in_x: f32 = 0.0;
        let mut in_y: f32 = 0.0;
        let mut anchor_x: f32 = 0.0;
        let mut anchor_y: f32 = 0.0;
        let mut l_anchor: f32 = 0.0;

        let mut i = last;
        let mut j = first;
        let mut k: i32 = -1;

        let wrap_idx = |idx: usize| -> usize { if idx < last { idx + 1 } else { first } };

        while j != i && (k < 0 || i != k as usize) {
            let out_x: f32;
            let out_y: f32;
            let l_out: f32;

            if k < 0 || j != k as usize {
                let mut ox = points[j].0 - points[i].0;
                let mut oy = points[j].1 - points[i].1;
                let lo = vec_normalize(&mut ox, &mut oy);

                if lo == 0.0 {
                    j = wrap_idx(j);
                    if j == i {
                        break;
                    }
                    continue;
                }
                out_x = ox;
                out_y = oy;
                l_out = lo;
            } else {
                out_x = anchor_x;
                out_y = anchor_y;
                l_out = l_anchor;
            }

            if l_in != 0.0 {
                if k < 0 {
                    k = i as i32;
                    anchor_x = in_x;
                    anchor_y = in_y;
                    l_anchor = l_in;
                }

                let d_cos = in_x * out_x + in_y * out_y;

                let shift_x: f32;
                let shift_y: f32;
                if d_cos > -0.9375 {
                    let d = d_cos + 1.0;

                    let mut sx = in_y + out_y;
                    let mut sy = in_x + out_x;

                    if orientation > 0 {
                        sx = -sx;
                    } else {
                        sy = -sy;
                    }

                    let mut q = out_x * in_y - out_y * in_x;
                    if orientation > 0 {
                        q = -q;
                    }

                    let l = l_in.min(l_out);

                    if d != 0.0 && q != 0.0 {
                        shift_x = if x_strength * q <= l * d {
                            sx * x_strength / d
                        } else {
                            sx * l / q
                        };
                        shift_y = if y_strength * q <= l * d {
                            sy * y_strength / d
                        } else {
                            sy * l / q
                        };
                    } else {
                        shift_x = 0.0;
                        shift_y = 0.0;
                    }
                } else {
                    shift_x = 0.0;
                    shift_y = 0.0;
                }

                while i != j {
                    points[i].0 += x_strength + shift_x;
                    points[i].1 += y_strength + shift_y;
                    i = wrap_idx(i);
                }
            } else {
                i = j;
            }

            in_x = out_x;
            in_y = out_y;
            l_in = l_out;

            j = wrap_idx(j);
        }
    }
}
