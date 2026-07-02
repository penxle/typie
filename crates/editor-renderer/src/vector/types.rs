#[derive(Debug, Clone)]
pub enum VectorPathCommand {
    MoveTo {
        x: f32,
        y: f32,
    },
    LineTo {
        x: f32,
        y: f32,
    },
    QuadTo {
        cx: f32,
        cy: f32,
        x: f32,
        y: f32,
    },
    CubicTo {
        c1x: f32,
        c1y: f32,
        c2x: f32,
        c2y: f32,
        x: f32,
        y: f32,
    },
    ClosePath,
}

#[derive(Debug, Clone, Copy)]
pub enum VectorFillRule {
    Winding,
    EvenOdd,
}

#[derive(Debug, Clone, Copy)]
pub enum VectorLineCap {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy)]
pub enum VectorLineJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone)]
pub enum VectorOp {
    FillPath {
        path: Vec<VectorPathCommand>,
        color: [u8; 4],
        fill_rule: VectorFillRule,
    },
    StrokePath {
        path: Vec<VectorPathCommand>,
        color: [u8; 4],
        width: f32,
        line_cap: VectorLineCap,
        line_join: VectorLineJoin,
    },
    Image {
        data: std::sync::Arc<[u8]>,
        width: u32,
        height: u32,
        x: f32,
        y: f32,
        render_width: f32,
        render_height: f32,
    },
}

#[derive(Debug, Clone)]
pub struct TextOp {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
}

#[derive(Debug, Clone)]
pub struct VectorPage {
    pub width: f32,
    pub height: f32,
    pub ops: Vec<VectorOp>,
    pub text_ops: Vec<TextOp>,
}
