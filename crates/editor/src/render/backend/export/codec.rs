use super::sink::{
    ExportFillRule, ExportLineCap, ExportLineJoin, ExportOp, ExportPage, ExportPathCommand,
};

const MAGIC: u32 = 0x3156_4554; // TVE1

const OP_FILL_PATH: u8 = 0;
const OP_STROKE_PATH: u8 = 1;
const OP_DRAW_IMAGE: u8 = 2;

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

pub fn encode_export_page(page: &ExportPage) -> Vec<u8> {
    let mut out = Vec::with_capacity(page.ops.len() * 128 + 32);
    write_u32(&mut out, MAGIC);
    write_f32(&mut out, page.width);
    write_f32(&mut out, page.height);
    write_u32(&mut out, page.ops.len() as u32);

    for op in &page.ops {
        match op {
            ExportOp::FillPath {
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
                        ExportFillRule::Winding => FILL_RULE_WINDING,
                        ExportFillRule::EvenOdd => FILL_RULE_EVEN_ODD,
                    },
                );
            }
            ExportOp::StrokePath {
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
                        ExportLineCap::Butt => LINE_CAP_BUTT,
                        ExportLineCap::Round => LINE_CAP_ROUND,
                        ExportLineCap::Square => LINE_CAP_SQUARE,
                    },
                );
                write_u8(
                    &mut out,
                    match line_join {
                        ExportLineJoin::Miter => LINE_JOIN_MITER,
                        ExportLineJoin::Round => LINE_JOIN_ROUND,
                        ExportLineJoin::Bevel => LINE_JOIN_BEVEL,
                    },
                );
            }
            ExportOp::DrawImage {
                data,
                width,
                height,
                x,
                y,
            } => {
                write_u8(&mut out, OP_DRAW_IMAGE);
                write_u32(&mut out, *width);
                write_u32(&mut out, *height);
                write_f32(&mut out, *x);
                write_f32(&mut out, *y);
                write_u32(&mut out, data.len() as u32);
                out.extend_from_slice(data);
            }
        }
    }

    write_u32(&mut out, page.text_ops.len() as u32);
    for text_op in &page.text_ops {
        write_f32(&mut out, text_op.x);
        write_f32(&mut out, text_op.y);
        write_f32(&mut out, text_op.size);
        write_str(&mut out, &text_op.text);
    }

    out
}

fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_str(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    write_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

fn write_path_commands(out: &mut Vec<u8>, path: &[ExportPathCommand]) {
    for command in path {
        match command {
            ExportPathCommand::MoveTo { x, y } => {
                write_u8(out, CMD_MOVE_TO);
                write_f32(out, *x);
                write_f32(out, *y);
            }
            ExportPathCommand::LineTo { x, y } => {
                write_u8(out, CMD_LINE_TO);
                write_f32(out, *x);
                write_f32(out, *y);
            }
            ExportPathCommand::QuadTo { cx, cy, x, y } => {
                write_u8(out, CMD_QUAD_TO);
                write_f32(out, *cx);
                write_f32(out, *cy);
                write_f32(out, *x);
                write_f32(out, *y);
            }
            ExportPathCommand::CubicTo {
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
            ExportPathCommand::ClosePath => {
                write_u8(out, CMD_CLOSE_PATH);
            }
        }
    }
}
