use editor_renderer::damage::IRect;

pub const GL_THRESHOLD_PX: u64 = 262_144;
pub const GL_BACKOFF_PRESENTS: u32 = 30;

pub fn use_gl(r: IRect) -> bool {
    (r.width().max(0) as u64) * (r.height().max(0) as u64) >= GL_THRESHOLD_PX
}

pub fn split_strips(r: IRect, max_size: i32) -> Vec<IRect> {
    if max_size <= 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut y = r.y0;
    while y < r.y1 {
        let y1 = y.saturating_add(max_size).min(r.y1);
        let mut x = r.x0;
        while x < r.x1 {
            let x1 = x.saturating_add(max_size).min(r.x1);
            out.push(IRect {
                x0: x,
                y0: y,
                x1,
                y1,
            });
            x = x1;
        }
        y = y1;
    }
    out
}

pub struct Backoff {
    skip: u32,
}

impl Backoff {
    pub const fn new() -> Self {
        Self { skip: 0 }
    }

    pub fn allow(&mut self) -> bool {
        if self.skip > 0 {
            self.skip -= 1;
            false
        } else {
            true
        }
    }

    pub fn fail(&mut self) {
        self.skip = GL_BACKOFF_PRESENTS;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: i32, y0: i32, x1: i32, y1: i32) -> IRect {
        IRect { x0, y0, x1, y1 }
    }

    #[test]
    fn use_gl_threshold_boundary() {
        assert!(use_gl(rect(0, 0, 512, 512)));
        assert!(!use_gl(rect(0, 0, 512, 511)));
        assert!(!use_gl(rect(0, 0, 0, 0)));
    }

    #[test]
    fn split_strips_returns_rect_within_limit() {
        assert_eq!(
            split_strips(rect(3, 5, 100, 200), 4096),
            vec![rect(3, 5, 100, 200)]
        );
    }

    #[test]
    fn split_strips_splits_tall_rect_vertically() {
        let out = split_strips(rect(0, 0, 100, 10_000), 4096);
        assert_eq!(
            out,
            vec![
                rect(0, 0, 100, 4096),
                rect(0, 4096, 100, 8192),
                rect(0, 8192, 100, 10_000),
            ]
        );
    }

    #[test]
    fn split_strips_tiles_both_axes() {
        let out = split_strips(rect(0, 0, 5000, 5000), 4096);
        assert_eq!(
            out,
            vec![
                rect(0, 0, 4096, 4096),
                rect(4096, 0, 5000, 4096),
                rect(0, 4096, 4096, 5000),
                rect(4096, 4096, 5000, 5000),
            ]
        );
    }

    #[test]
    fn split_strips_empty_rect_yields_nothing() {
        assert!(split_strips(rect(10, 10, 10, 20), 4096).is_empty());
    }

    #[test]
    fn split_strips_nonpositive_max_size_yields_nothing() {
        assert!(split_strips(rect(0, 0, 100, 100), 0).is_empty());
        assert!(split_strips(rect(0, 0, 100, 100), -1).is_empty());
    }

    #[test]
    fn split_strips_survives_i32_max_bounds() {
        let r = rect(i32::MAX - 10, i32::MAX - 10, i32::MAX, i32::MAX);
        assert_eq!(split_strips(r, 4096), vec![r]);
    }

    #[test]
    fn backoff_blocks_exactly_n_presents_after_failure() {
        let mut b = Backoff::new();
        assert!(b.allow());
        b.fail();
        for _ in 0..GL_BACKOFF_PRESENTS {
            assert!(!b.allow());
        }
        assert!(b.allow());
    }
}
