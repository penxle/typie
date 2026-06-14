use zeno::{
    Cap, Command, Fill, Format, Join, Mask, Origin, Placement, Scratch, Stroke as ZStroke,
    Transform as ZTransform, Vector,
};

use editor_common::Color;

use crate::types::{Path, PathElement, Stroke, StrokeCap, StrokeJoin, Transform};

pub struct RasterScratch {
    zeno: Scratch,
    mask: Vec<u8>,
}

impl RasterScratch {
    pub fn new() -> Self {
        Self {
            zeno: Scratch::new(),
            mask: Vec::new(),
        }
    }
}

fn to_ztransform(t: Transform) -> ZTransform {
    let [a, b, c, d, e, f] = t.m;
    ZTransform {
        xx: a,
        xy: b,
        yx: c,
        yy: d,
        x: e,
        y: f,
    }
}

fn path_to_commands(path: &Path) -> Vec<Command> {
    let mut out = Vec::with_capacity(path.elements.len());
    for el in &path.elements {
        match *el {
            PathElement::MoveTo { x, y } => out.push(Command::MoveTo(Vector::new(x, y))),
            PathElement::LineTo { x, y } => out.push(Command::LineTo(Vector::new(x, y))),
            PathElement::QuadTo { x1, y1, x, y } => {
                out.push(Command::QuadTo(Vector::new(x1, y1), Vector::new(x, y)))
            }
            PathElement::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => out.push(Command::CurveTo(
                Vector::new(x1, y1),
                Vector::new(x2, y2),
                Vector::new(x, y),
            )),
            PathElement::Close => out.push(Command::Close),
        }
    }
    out
}

pub fn rasterize_fill_to_mask(
    scratch: &mut RasterScratch,
    path: &Path,
    transform: Transform,
) -> Placement {
    let commands = path_to_commands(path);
    scratch.mask.clear();
    Mask::with_scratch(&commands[..], &mut scratch.zeno)
        .format(Format::Alpha)
        .origin(Origin::TopLeft)
        .style(Fill::NonZero)
        .transform(Some(to_ztransform(transform)))
        .inspect(|fmt, w, h| scratch.mask.resize(fmt.buffer_size(w, h), 0))
        .render_into(&mut scratch.mask, None)
}

fn to_zcap(cap: StrokeCap) -> Cap {
    match cap {
        StrokeCap::Butt => Cap::Butt,
        StrokeCap::Round => Cap::Round,
        StrokeCap::Square => Cap::Square,
    }
}

fn to_zjoin(join: StrokeJoin) -> Join {
    match join {
        StrokeJoin::Miter => Join::Miter,
        StrokeJoin::Round => Join::Round,
        StrokeJoin::Bevel => Join::Bevel,
    }
}

pub fn rasterize_stroke_to_mask(
    scratch: &mut RasterScratch,
    path: &Path,
    stroke: &Stroke,
    transform: Transform,
) -> Placement {
    let commands = path_to_commands(path);
    let mut zstroke = ZStroke::new(stroke.width);
    zstroke.cap(to_zcap(stroke.cap));
    zstroke.join(to_zjoin(stroke.join));
    scratch.mask.clear();
    Mask::with_scratch(&commands[..], &mut scratch.zeno)
        .format(Format::Alpha)
        .origin(Origin::TopLeft)
        .style(zstroke)
        .transform(Some(to_ztransform(transform)))
        .inspect(|fmt, w, h| scratch.mask.resize(fmt.buffer_size(w, h), 0))
        .render_into(&mut scratch.mask, None)
}

pub fn mask(scratch: &RasterScratch) -> &[u8] {
    &scratch.mask
}

pub fn premul_pixel(m: u8, color: Color) -> [u8; 4] {
    let a = (m as u32 * color.a as u32) >> 8;
    let pr = (a * color.r as u32) >> 8;
    let pg = (a * color.g as u32) >> 8;
    let pb = (a * color.b as u32) >> 8;
    [pr as u8, pg as u8, pb as u8, a as u8]
}

#[cfg(test)]
mod tests {
    use super::premul_pixel;
    use editor_common::Color;

    #[test]
    fn premul_full_coverage_opaque() {
        let c = Color {
            r: 200,
            g: 100,
            b: 50,
            a: 255,
        };
        let p = premul_pixel(255, c);
        assert_eq!(p[3], 254);
        assert_eq!(p[0], (254u32 * 200 >> 8) as u8);
    }

    #[test]
    fn premul_zero_coverage_is_transparent() {
        let c = Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };
        assert_eq!(premul_pixel(0, c), [0, 0, 0, 0]);
    }
}
