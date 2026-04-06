use zeno::{Point, Transform, Verb};

#[derive(Clone, Default)]
pub struct Outline {
    points: Vec<Point>,
    verbs: Vec<Verb>,
}

impl Outline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn points(&self) -> &[Point] {
        &self.points
    }

    pub fn verbs(&self) -> &[Verb] {
        &self.verbs
    }

    pub fn is_empty(&self) -> bool {
        self.verbs.is_empty()
    }

    pub fn clear(&mut self) {
        self.points.clear();
        self.verbs.clear();
    }

    pub fn move_to(&mut self, p: Point) {
        self.points.push(p);
        self.verbs.push(Verb::MoveTo);
    }

    pub fn line_to(&mut self, p: Point) {
        self.points.push(p);
        self.verbs.push(Verb::LineTo);
    }

    pub fn quad_to(&mut self, p0: Point, p1: Point) {
        self.points.push(p0);
        self.points.push(p1);
        self.verbs.push(Verb::QuadTo);
    }

    pub fn curve_to(&mut self, p0: Point, p1: Point, p2: Point) {
        self.points.push(p0);
        self.points.push(p1);
        self.points.push(p2);
        self.verbs.push(Verb::CurveTo);
    }

    pub fn close(&mut self) {
        self.verbs.push(Verb::Close);
    }

    pub fn embolden(&mut self, x_strength: f32, y_strength: f32) {
        let winding = compute_outline_winding(&self.points, &self.verbs);
        let mut point_start = 0;
        let mut pos = 0;

        for verb_idx in 0..self.verbs.len() {
            match self.verbs[verb_idx] {
                Verb::MoveTo | Verb::Close => {
                    if let Some(points) = self.points.get_mut(point_start..pos) {
                        if !points.is_empty() {
                            embolden(points, winding, x_strength, y_strength);
                        }

                        point_start = pos;

                        if self.verbs[verb_idx] == Verb::MoveTo {
                            pos += 1;
                        }
                    } else {
                        return;
                    }
                }
                Verb::LineTo => pos += 1,
                Verb::QuadTo => pos += 2,
                Verb::CurveTo => pos += 3,
            }
        }

        if pos > point_start
            && let Some(points) = self.points.get_mut(point_start..pos)
        {
            embolden(points, winding, x_strength, y_strength);
        }
    }

    pub fn transform(&mut self, transform: &Transform) {
        for p in &mut self.points {
            *p = transform.transform_point(*p);
        }
    }
}

fn embolden(points: &mut [Point], winding: u8, x_strength: f32, y_strength: f32) {
    if points.is_empty() {
        return;
    }

    let last = points.len() - 1;
    let mut i = last;
    let mut j = 0;
    let mut k = !0;
    let mut out_len;
    let mut in_len = 0.;
    let mut anchor_len = 0.;
    let mut anchor = Point::ZERO;
    let mut out;
    let mut in_ = Point::ZERO;
    while j != i && i != k {
        if j != k {
            out = points[j] - points[i];
            out_len = out.length();
            if out_len == 0. {
                j = if j < last { j + 1 } else { 0 };
                continue;
            } else {
                let s = 1. / out_len;
                out.x *= s;
                out.y *= s;
            }
        } else {
            out = anchor;
            out_len = anchor_len;
        }
        if in_len != 0. {
            if k == !0 {
                k = i;
                anchor = in_;
                anchor_len = in_len;
            }
            let mut d = (in_.x * out.x) + (in_.y * out.y);
            let shift = if d > -0.9396 {
                d += 1.;
                let mut sx = in_.y + out.y;
                let mut sy = in_.x + out.x;
                if winding == 0 {
                    sx = -sx;
                } else {
                    sy = -sy;
                }
                let mut q = (out.x * in_.y) - (out.y * in_.x);
                if winding == 0 {
                    q = -q;
                }
                let l = in_len.min(out_len);
                if x_strength * q <= l * d {
                    sx = sx * x_strength / d;
                } else {
                    sx = sx * l / q;
                }
                if y_strength * q <= l * d {
                    sy = sy * y_strength / d;
                } else {
                    sy = sy * l / q;
                }
                Point::new(sx, sy)
            } else {
                Point::ZERO
            };

            while i != j {
                points[i].x += x_strength + shift.x;
                points[i].y += y_strength + shift.y;
                i = if i < last { i + 1 } else { 0 };
            }
        } else {
            i = j;
        }
        in_ = out;
        in_len = out_len;
        j = if j < last { j + 1 } else { 0 };
    }
}

fn compute_outline_winding(points: &[Point], verbs: &[Verb]) -> u8 {
    let mut total_area = 0.0f32;
    let mut point_start = 0;
    let mut pos = 0;

    for verb in verbs {
        match verb {
            Verb::MoveTo | Verb::Close => {
                if let Some(contour) = points.get(point_start..pos)
                    && !contour.is_empty()
                {
                    total_area += contour_area(contour);
                }

                point_start = pos;

                if *verb == Verb::MoveTo {
                    pos += 1;
                }
            }
            Verb::LineTo => pos += 1,
            Verb::QuadTo => pos += 2,
            Verb::CurveTo => pos += 3,
        }
    }

    if let Some(contour) = points.get(point_start..pos)
        && !contour.is_empty()
    {
        total_area += contour_area(contour);
    }

    if total_area > 0. { 1 } else { 0 }
}

fn contour_area(points: &[Point]) -> f32 {
    if points.is_empty() {
        return 0.;
    }

    let mut area = 0.;
    let last = points.len() - 1;
    let mut prev = points[last];
    for cur in points[0..=last].iter() {
        area += (cur.y - prev.y) * (cur.x + prev.x);
        prev = *cur;
    }

    area
}
