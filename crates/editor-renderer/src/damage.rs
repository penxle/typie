use editor_common::Rect;

pub const MAX_DAMAGE_RECTS: usize = 8;
pub const DAMAGE_FULL_FRACTION: f64 = 0.6;
pub const MERGE_INPUT_CAP: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IRect {
    pub fn width(self) -> i32 {
        (self.x1 - self.x0).max(0)
    }

    pub fn height(self) -> i32 {
        (self.y1 - self.y0).max(0)
    }

    pub fn is_empty(self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    pub fn area(self) -> i64 {
        self.width() as i64 * self.height() as i64
    }

    pub fn intersect(self, o: IRect) -> Option<IRect> {
        let r = IRect {
            x0: self.x0.max(o.x0),
            y0: self.y0.max(o.y0),
            x1: self.x1.min(o.x1),
            y1: self.y1.min(o.y1),
        };
        if r.is_empty() { None } else { Some(r) }
    }

    pub fn union(self, o: IRect) -> IRect {
        IRect {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }

    pub fn from_rect_padded(r: Rect, pad: i32, bounds: IRect) -> Option<IRect> {
        let x0 = (r.x.floor() as i32) - pad;
        let y0 = (r.y.floor() as i32) - pad;
        let x1 = ((r.x + r.width).ceil() as i32) + pad;
        let y1 = ((r.y + r.height).ceil() as i32) + pad;
        IRect { x0, y0, x1, y1 }.intersect(bounds)
    }
}

pub fn merge_damage(rects: &[IRect], full: IRect) -> Vec<IRect> {
    let non_empty: Vec<IRect> = rects.iter().copied().filter(|r| !r.is_empty()).collect();
    if non_empty.len() > MERGE_INPUT_CAP {
        let Some(bbox) = non_empty.into_iter().reduce(IRect::union) else {
            return Vec::new();
        };
        let Some(bbox) = bbox.intersect(full) else {
            return Vec::new();
        };
        if bbox.area() as f64 >= full.area() as f64 * DAMAGE_FULL_FRACTION {
            return vec![full];
        }
        return vec![bbox];
    }

    let mut merged: Vec<IRect> = Vec::new();
    'outer: for &r in rects {
        if r.is_empty() {
            continue;
        }
        for m in merged.iter_mut() {
            if overlaps_or_adjacent(*m, r) {
                *m = m.union(r);
                continue 'outer;
            }
        }
        merged.push(r);
    }
    let mut changed = true;
    while changed {
        changed = false;
        'pair: for i in 0..merged.len() {
            for j in (i + 1)..merged.len() {
                if overlaps_or_adjacent(merged[i], merged[j]) {
                    merged[i] = merged[i].union(merged[j]);
                    merged.remove(j);
                    changed = true;
                    break 'pair;
                }
            }
        }
    }
    let total: i64 = merged.iter().map(|r| r.area()).sum();
    if merged.len() > MAX_DAMAGE_RECTS || total as f64 >= full.area() as f64 * DAMAGE_FULL_FRACTION
    {
        return vec![full];
    }
    merged
}

fn overlaps_or_adjacent(a: IRect, b: IRect) -> bool {
    a.x0 <= b.x1 && b.x0 <= a.x1 && a.y0 <= b.y1 && b.y0 <= a.y1
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::Rect;

    fn full() -> IRect {
        IRect {
            x0: 0,
            y0: 0,
            x1: 100,
            y1: 100,
        }
    }

    #[test]
    fn from_rect_padded_expands_and_clamps() {
        let r = Rect::from_xywh(10.2, 10.8, 5.0, 5.0);
        let ir = IRect::from_rect_padded(r, 1, full()).unwrap();
        assert_eq!((ir.x0, ir.y0, ir.x1, ir.y1), (9, 9, 17, 17));
    }

    #[test]
    fn from_rect_padded_clamps_to_bounds() {
        let r = Rect::from_xywh(-5.0, -5.0, 10.0, 10.0);
        let ir = IRect::from_rect_padded(r, 1, full()).unwrap();
        assert_eq!(ir.x0, 0);
        assert_eq!(ir.y0, 0);
    }

    #[test]
    fn from_rect_padded_empty_is_none() {
        let r = Rect::from_xywh(200.0, 200.0, 10.0, 10.0);
        assert!(IRect::from_rect_padded(r, 0, full()).is_none());
    }

    #[test]
    fn from_rect_padded_covers_fractional_right_bottom() {
        let r = Rect::from_xywh(2.0, 2.0, 3.4, 3.6);
        let ir = IRect::from_rect_padded(r, 0, full()).unwrap();
        assert!(
            ir.x1 >= 6 && ir.y1 >= 6,
            "ceil must include the partially-covered edge pixel"
        );
        assert_eq!((ir.x0, ir.y0), (2, 2));
    }

    #[test]
    fn negative_origin_clamped_but_keeps_visible_part() {
        let r = Rect::from_xywh(-3.0, 5.0, 10.0, 4.0);
        let ir = IRect::from_rect_padded(r, 0, full()).unwrap();
        assert_eq!(ir.x0, 0);
        assert_eq!(ir.x1, 7);
        assert_eq!((ir.y0, ir.y1), (5, 9));
    }

    #[test]
    fn intersect_and_union() {
        let a = IRect {
            x0: 0,
            y0: 0,
            x1: 10,
            y1: 10,
        };
        let b = IRect {
            x0: 5,
            y0: 5,
            x1: 20,
            y1: 20,
        };
        assert_eq!(
            a.intersect(b).unwrap(),
            IRect {
                x0: 5,
                y0: 5,
                x1: 10,
                y1: 10
            }
        );
        assert_eq!(
            a.union(b),
            IRect {
                x0: 0,
                y0: 0,
                x1: 20,
                y1: 20
            }
        );
        assert!(
            a.intersect(IRect {
                x0: 50,
                y0: 50,
                x1: 60,
                y1: 60
            })
            .is_none()
        );
    }

    #[test]
    fn merge_falls_back_to_full_when_too_many() {
        let rects: Vec<IRect> = (0..20)
            .map(|i| IRect {
                x0: i * 4,
                y0: 0,
                x1: i * 4 + 1,
                y1: 1,
            })
            .collect();
        assert_eq!(merge_damage(&rects, full()), vec![full()]);
    }

    #[test]
    fn merge_falls_back_to_full_when_large_area() {
        let big = IRect {
            x0: 0,
            y0: 0,
            x1: 100,
            y1: 70,
        };
        assert_eq!(merge_damage(&[big], full()), vec![full()]);
    }

    #[test]
    fn merge_keeps_disjoint_small_rects() {
        let a = IRect {
            x0: 0,
            y0: 0,
            x1: 5,
            y1: 5,
        };
        let b = IRect {
            x0: 80,
            y0: 80,
            x1: 85,
            y1: 85,
        };
        let out = merge_damage(&[a, b], full());
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn merge_large_clustered_input_returns_single_tight_bbox() {
        let rects: Vec<IRect> = (0..40)
            .map(|i| {
                let x = 10 + (i % 10);
                let y = 10 + (i / 10);
                IRect {
                    x0: x,
                    y0: y,
                    x1: x + 1,
                    y1: y + 1,
                }
            })
            .collect();
        assert!(rects.len() > MERGE_INPUT_CAP);
        let out = merge_damage(&rects, full());
        assert_eq!(out.len(), 1);
        assert_ne!(out[0], full());
        assert!(out[0].x0 <= 10 && out[0].y0 <= 10 && out[0].x1 >= 20 && out[0].y1 >= 14);
    }

    #[test]
    fn merge_large_input_covering_most_of_full_returns_full() {
        let mut rects: Vec<IRect> = (0..40)
            .map(|i| IRect {
                x0: i,
                y0: i,
                x1: i + 1,
                y1: i + 1,
            })
            .collect();
        rects.push(IRect {
            x0: 0,
            y0: 0,
            x1: 1,
            y1: 1,
        });
        rects.push(IRect {
            x0: 99,
            y0: 99,
            x1: 100,
            y1: 100,
        });
        assert!(rects.len() > MERGE_INPUT_CAP);
        assert_eq!(merge_damage(&rects, full()), vec![full()]);
    }
}
