use crate::vector::types::{
    VectorFillRule, VectorLineCap, VectorLineJoin, VectorOp, VectorPage, VectorPathCommand,
};

const MAGIC: u32 = 0x3156_4554;
const OP_FILL_PATH: u8 = 0;
const OP_STROKE_PATH: u8 = 1;
const OP_IMAGE: u8 = 2;
const FILL_RULE_WINDING: u8 = 0;
const FILL_RULE_EVEN_ODD: u8 = 1;
const LINE_CAP_BUTT: u8 = 0;
const LINE_CAP_ROUND: u8 = 1;
const LINE_CAP_SQUARE: u8 = 2;
const LINE_JOIN_MITER: u8 = 0;
const LINE_JOIN_ROUND: u8 = 1;
const LINE_JOIN_BEVEL: u8 = 2;
const CMD_MOVE_TO: u8 = 0;
const CMD_LINE_TO: u8 = 1;
const CMD_QUAD_TO: u8 = 2;
const CMD_CUBIC_TO: u8 = 3;
const CMD_CLOSE_PATH: u8 = 4;

pub fn encode_vector_page(page: &VectorPage) -> Vec<u8> {
    let mut out = Vec::with_capacity(page.ops.len() * 128 + 32);
    write_u32(&mut out, MAGIC);
    write_f32(&mut out, page.width);
    write_f32(&mut out, page.height);
    write_u32(&mut out, page.ops.len() as u32);

    for op in &page.ops {
        match op {
            VectorOp::FillPath {
                path,
                color,
                fill_rule,
            } => {
                write_u8(&mut out, OP_FILL_PATH);
                write_u32(&mut out, path.len() as u32);
                write_path_commands(&mut out, path);
                out.extend_from_slice(color);
                write_u8(
                    &mut out,
                    match fill_rule {
                        VectorFillRule::Winding => FILL_RULE_WINDING,
                        VectorFillRule::EvenOdd => FILL_RULE_EVEN_ODD,
                    },
                );
            }
            VectorOp::StrokePath {
                path,
                color,
                width,
                line_cap,
                line_join,
            } => {
                write_u8(&mut out, OP_STROKE_PATH);
                write_u32(&mut out, path.len() as u32);
                write_path_commands(&mut out, path);
                out.extend_from_slice(color);
                write_f32(&mut out, *width);
                write_u8(
                    &mut out,
                    match line_cap {
                        VectorLineCap::Butt => LINE_CAP_BUTT,
                        VectorLineCap::Round => LINE_CAP_ROUND,
                        VectorLineCap::Square => LINE_CAP_SQUARE,
                    },
                );
                write_u8(
                    &mut out,
                    match line_join {
                        VectorLineJoin::Miter => LINE_JOIN_MITER,
                        VectorLineJoin::Round => LINE_JOIN_ROUND,
                        VectorLineJoin::Bevel => LINE_JOIN_BEVEL,
                    },
                );
            }
            VectorOp::Image {
                data,
                width,
                height,
                x,
                y,
                render_width,
                render_height,
            } => {
                write_u8(&mut out, OP_IMAGE);
                write_u32(&mut out, *width);
                write_u32(&mut out, *height);
                write_f32(&mut out, *x);
                write_f32(&mut out, *y);
                write_f32(&mut out, *render_width);
                write_f32(&mut out, *render_height);
                write_u32(&mut out, data.len() as u32);
                out.extend_from_slice(data);
            }
        }
    }

    out
}

fn write_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}
fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn write_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_path_commands(out: &mut Vec<u8>, path: &[VectorPathCommand]) {
    for cmd in path {
        match cmd {
            VectorPathCommand::MoveTo { x, y } => {
                write_u8(out, CMD_MOVE_TO);
                write_f32(out, *x);
                write_f32(out, *y);
            }
            VectorPathCommand::LineTo { x, y } => {
                write_u8(out, CMD_LINE_TO);
                write_f32(out, *x);
                write_f32(out, *y);
            }
            VectorPathCommand::QuadTo { cx, cy, x, y } => {
                write_u8(out, CMD_QUAD_TO);
                write_f32(out, *cx);
                write_f32(out, *cy);
                write_f32(out, *x);
                write_f32(out, *y);
            }
            VectorPathCommand::CubicTo {
                c1x,
                c1y,
                c2x,
                c2y,
                x,
                y,
            } => {
                write_u8(out, CMD_CUBIC_TO);
                write_f32(out, *c1x);
                write_f32(out, *c1y);
                write_f32(out, *c2x);
                write_f32(out, *c2y);
                write_f32(out, *x);
                write_f32(out, *y);
            }
            VectorPathCommand::ClosePath => {
                write_u8(out, CMD_CLOSE_PATH);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::types::VectorPage;

    #[test]
    fn encode_starts_with_magic() {
        // 인코딩 결과가 TVE1 매직으로 시작해 외부 파서가 포맷을 식별할 수 있는지 확인한다.
        let page = VectorPage {
            width: 100.0,
            height: 200.0,
            ops: vec![],
        };
        let bytes = encode_vector_page(&page);
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert_eq!(magic, MAGIC);
    }

    #[test]
    fn encode_records_dimensions() {
        // 페이지 크기가 바이너리 헤더에 그대로 기록되는지 확인한다.
        let page = VectorPage {
            width: 123.0,
            height: 456.0,
            ops: vec![],
        };
        let bytes = encode_vector_page(&page);
        let w = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let h = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        assert!((w - 123.0).abs() < 0.001);
        assert!((h - 456.0).abs() < 0.001);
    }

    #[test]
    fn encode_vector_page_produces_converter_ready_binary() {
        // PDF/SVG 변환기가 바로 읽을 수 있도록
        // 인코딩 결과가 TVE1 헤더와 최소한의 op 정보를 포함하는지 확인한다.
        let page = VectorPage {
            width: 210.0,
            height: 297.0,
            ops: vec![VectorOp::FillPath {
                path: vec![
                    VectorPathCommand::MoveTo { x: 0.0, y: 0.0 },
                    VectorPathCommand::LineTo { x: 10.0, y: 0.0 },
                    VectorPathCommand::ClosePath,
                ],
                color: [255, 0, 0, 255],
                fill_rule: VectorFillRule::Winding,
            }],
        };

        let bytes = encode_vector_page(&page);

        assert_eq!(u32::from_le_bytes(bytes[0..4].try_into().unwrap()), MAGIC);
        assert!((f32::from_le_bytes(bytes[4..8].try_into().unwrap()) - 210.0).abs() < 0.001);
        assert!((f32::from_le_bytes(bytes[8..12].try_into().unwrap()) - 297.0).abs() < 0.001);
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 1);
        assert_eq!(bytes[16], OP_FILL_PATH);
    }
}
