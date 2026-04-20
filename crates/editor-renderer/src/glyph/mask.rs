use zeno::{
    Command, Format, Mask, Origin, Placement, Scratch, Transform as ZTransform, Vector, Verb,
};

use super::outline::Outline;

/// Outline → alpha mask 래스터라이즈.
/// subpixel_offset_x 는 0.0, 0.25, 0.5, 0.75 중 하나이며 서브픽셀 포지셔닝을 결정한다.
/// transform 은 선택적 선형 변환(skew 등)으로 origin 좌표계 기준.
pub fn rasterize_outline_to_mask(
    outline: &Outline,
    scratch: &mut Scratch,
    subpixel_offset_x: f32,
    transform: Option<ZTransform>,
    out: &mut Vec<u8>,
) -> Placement {
    let commands = outline_to_commands(outline);
    let offset = Vector::new(subpixel_offset_x, 0.0);

    out.clear();
    Mask::with_scratch(&commands[..], scratch)
        .format(Format::Alpha)
        .origin(Origin::BottomLeft)
        .offset(offset)
        .render_offset(offset)
        .transform(transform)
        .inspect(|fmt, w, h| {
            out.resize(fmt.buffer_size(w, h), 0);
        })
        .render_into(out, None)
}

fn outline_to_commands(outline: &Outline) -> Vec<Command> {
    let points = outline.points();
    let verbs = outline.verbs();
    let mut out = Vec::with_capacity(verbs.len());
    let mut i = 0usize;
    for v in verbs {
        match v {
            Verb::MoveTo => {
                out.push(Command::MoveTo(points[i]));
                i += 1;
            }
            Verb::LineTo => {
                out.push(Command::LineTo(points[i]));
                i += 1;
            }
            Verb::QuadTo => {
                out.push(Command::QuadTo(points[i], points[i + 1]));
                i += 2;
            }
            Verb::CurveTo => {
                out.push(Command::CurveTo(points[i], points[i + 1], points[i + 2]));
                i += 3;
            }
            Verb::Close => {
                out.push(Command::Close);
            }
        }
    }
    out
}
