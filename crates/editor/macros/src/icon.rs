use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitStr, Token};

pub struct SvgIconArgs {
    pub path: LitStr,
    pub size: Expr,
    pub cx: Expr,
    pub cy: Expr,
}

impl Parse for SvgIconArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let size: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let cx: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let cy: Expr = input.parse()?;
        Ok(SvgIconArgs { path, size, cx, cy })
    }
}

#[derive(Deserialize)]
struct IconifyIcons {
    icons: HashMap<String, IconData>,
}

#[derive(Deserialize)]
struct IconData {
    body: String,
}

fn get_macros_dir() -> Option<PathBuf> {
    std::env::var("CARGO_MANIFEST_DIR").ok().map(PathBuf::from)
}

fn find_lucide_icons_json() -> Option<PathBuf> {
    let manifest_path = get_macros_dir()?;

    let path = manifest_path
        .join("..")
        .join("..")
        .join("apps")
        .join("website")
        .join("node_modules")
        .join("@iconify-json")
        .join("lucide")
        .join("icons.json");

    if path.exists() {
        path.canonicalize().ok()
    } else {
        None
    }
}

fn load_lucide_icons() -> Result<IconifyIcons, String> {
    let path = find_lucide_icons_json().ok_or_else(|| {
        "Could not find @iconify-json/lucide/icons.json. Make sure the package is installed."
            .to_string()
    })?;

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

fn get_svg_body(icon_path: &str) -> Result<String, String> {
    if let Some(icon_name) = icon_path.strip_prefix("lucide/") {
        let icons = load_lucide_icons()?;
        icons
            .icons
            .get(icon_name)
            .map(|d| d.body.clone())
            .ok_or_else(|| format!("Icon '{}' not found in @iconify-json/lucide", icon_name))
    } else if let Some(icon_name) = icon_path.strip_prefix("typie/") {
        if icon_name.contains("..") || icon_name.contains('/') || icon_name.contains('\\') {
            return Err(format!(
                "Invalid icon name '{}': must not contain path separators or '..'",
                icon_name
            ));
        }

        let manifest_path =
            get_macros_dir().ok_or_else(|| "Could not get CARGO_MANIFEST_DIR".to_string())?;

        let svg_path = manifest_path
            .join("..")
            .join("assets")
            .join("icons")
            .join(format!("{}.svg", icon_name));

        if !svg_path.exists() {
            return Err(format!(
                "Icon '{}' not found at {}",
                icon_name,
                svg_path.display()
            ));
        }

        std::fs::read_to_string(&svg_path)
            .map_err(|e| format!("Failed to read {}: {}", svg_path.display(), e))
    } else {
        Err(format!(
            "Unknown icon source: '{}'. Use 'lucide/icon_name' or 'typie/icon_name'",
            icon_path
        ))
    }
}

fn parse_svg_body(body: &str) -> Vec<ParsedCommand> {
    let mut commands = Vec::new();

    for cap in regex_circle(body) {
        commands.push(ParsedCommand::Circle(cap.0, cap.1, cap.2));
    }

    for d_attr in regex_path_d(body) {
        commands.extend(parse_path_d(&d_attr));
    }

    commands
}

fn regex_circle(svg: &str) -> Vec<(f32, f32, f32)> {
    let mut circles = Vec::new();
    let mut remaining = svg;

    while let Some(start) = remaining.find("<circle") {
        let rest = &remaining[start..];
        if let Some(end) = rest.find("/>") {
            let element = &rest[..end + 2];

            let cx = extract_attr(element, "cx").unwrap_or(0.0);
            let cy = extract_attr(element, "cy").unwrap_or(0.0);
            let r = extract_attr(element, "r").unwrap_or(0.0);

            circles.push((cx, cy, r));
            remaining = &rest[end + 2..];
        } else {
            break;
        }
    }

    circles
}

fn regex_path_d(svg: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut remaining = svg;

    while let Some(start) = remaining.find("<path") {
        let rest = &remaining[start..];
        if let Some(end) = rest.find("/>") {
            let element = &rest[..end + 2];

            if let Some(d) = extract_string_attr(element, "d") {
                paths.push(d);
            }
            remaining = &rest[end + 2..];
        } else {
            break;
        }
    }

    paths
}

fn extract_attr(element: &str, name: &str) -> Option<f32> {
    let pattern = format!("{}=\"", name);
    let start = element.find(&pattern)? + pattern.len();
    let rest = &element[start..];
    let end = rest.find('"')?;
    rest[..end].parse().ok()
}

fn extract_string_attr(element: &str, name: &str) -> Option<String> {
    let pattern = format!("{}=\"", name);
    let start = element.find(&pattern)? + pattern.len();
    let rest = &element[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[derive(Debug, Clone)]
enum ParsedCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    HLineTo(f32),
    VLineTo(f32),
    CurveTo(f32, f32, f32, f32, f32, f32),
    QuadTo(f32, f32, f32, f32),
    Circle(f32, f32, f32),
    Close,
}

fn arc_to_beziers(
    x1: f32,
    y1: f32,
    mut rx: f32,
    mut ry: f32,
    x_rotation: f32,
    large_arc: bool,
    sweep: bool,
    x2: f32,
    y2: f32,
) -> Vec<(f32, f32, f32, f32, f32, f32)> {
    let mut result = Vec::new();

    if (x1 - x2).abs() < 1e-6 && (y1 - y2).abs() < 1e-6 {
        return result;
    }
    if rx.abs() < 1e-6 || ry.abs() < 1e-6 {
        return result;
    }

    rx = rx.abs();
    ry = ry.abs();

    let phi = x_rotation.to_radians();
    let cos_phi = phi.cos();
    let sin_phi = phi.sin();

    let dx = (x1 - x2) / 2.0;
    let dy = (y1 - y2) / 2.0;
    let x1p = cos_phi * dx + sin_phi * dy;
    let y1p = -sin_phi * dx + cos_phi * dy;

    let x1p_sq = x1p * x1p;
    let y1p_sq = y1p * y1p;
    let rx_sq = rx * rx;
    let ry_sq = ry * ry;

    let lambda = x1p_sq / rx_sq + y1p_sq / ry_sq;
    if lambda > 1.0 {
        let lambda_sqrt = lambda.sqrt();
        rx *= lambda_sqrt;
        ry *= lambda_sqrt;
    }

    let rx_sq = rx * rx;
    let ry_sq = ry * ry;

    let sq = ((rx_sq * ry_sq - rx_sq * y1p_sq - ry_sq * x1p_sq)
        / (rx_sq * y1p_sq + ry_sq * x1p_sq))
        .max(0.0)
        .sqrt();

    let sq = if large_arc == sweep { -sq } else { sq };

    let cxp = sq * rx * y1p / ry;
    let cyp = -sq * ry * x1p / rx;

    let cx = cos_phi * cxp - sin_phi * cyp + (x1 + x2) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (y1 + y2) / 2.0;

    let ux = (x1p - cxp) / rx;
    let uy = (y1p - cyp) / ry;
    let vx = (-x1p - cxp) / rx;
    let vy = (-y1p - cyp) / ry;

    let n = (ux * ux + uy * uy).sqrt();
    let theta1 = (ux / n).acos() * if uy < 0.0 { -1.0 } else { 1.0 };

    let n = ((ux * ux + uy * uy) * (vx * vx + vy * vy)).sqrt();
    let mut dtheta = ((ux * vx + uy * vy) / n).clamp(-1.0, 1.0).acos();
    if ux * vy - uy * vx < 0.0 {
        dtheta = -dtheta;
    }

    if sweep && dtheta < 0.0 {
        dtheta += 2.0 * std::f32::consts::PI;
    } else if !sweep && dtheta > 0.0 {
        dtheta -= 2.0 * std::f32::consts::PI;
    }

    let num_segments = (dtheta.abs() / (std::f32::consts::PI / 2.0)).ceil() as usize;
    let num_segments = num_segments.max(1);
    let delta = dtheta / num_segments as f32;

    let t = (delta / 4.0).tan();
    let alpha = (delta.sin()) * ((4.0 + 3.0 * t * t).sqrt() - 1.0) / 3.0;

    let mut theta = theta1;
    let mut prev_x = x1;
    let mut prev_y = y1;

    for _ in 0..num_segments {
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();
        let cos_theta2 = (theta + delta).cos();
        let sin_theta2 = (theta + delta).sin();

        let ex = cx + rx * cos_phi * cos_theta2 - ry * sin_phi * sin_theta2;
        let ey = cy + rx * sin_phi * cos_theta2 + ry * cos_phi * sin_theta2;

        let dx1 = -rx * cos_phi * sin_theta - ry * sin_phi * cos_theta;
        let dy1 = -rx * sin_phi * sin_theta + ry * cos_phi * cos_theta;
        let cp1x = prev_x + alpha * dx1;
        let cp1y = prev_y + alpha * dy1;

        let dx2 = -rx * cos_phi * sin_theta2 - ry * sin_phi * cos_theta2;
        let dy2 = -rx * sin_phi * sin_theta2 + ry * cos_phi * cos_theta2;
        let cp2x = ex - alpha * dx2;
        let cp2y = ey - alpha * dy2;

        result.push((cp1x, cp1y, cp2x, cp2y, ex, ey));

        theta += delta;
        prev_x = ex;
        prev_y = ey;
    }

    result
}

fn parse_path_d(d: &str) -> Vec<ParsedCommand> {
    let mut commands = Vec::new();
    let mut chars = d.chars().peekable();
    let mut current_cmd = ' ';
    let mut cx = 0.0_f32;
    let mut cy = 0.0_f32;

    while chars.peek().is_some() {
        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            chars.next();
        }

        if let Some(&c) = chars.peek() {
            if c.is_ascii_alphabetic() {
                current_cmd = c;
                chars.next();
            }
        }

        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            chars.next();
        }

        match current_cmd {
            'M' => {
                let x = parse_number(&mut chars);
                let y = parse_number(&mut chars);
                cx = x;
                cy = y;
                commands.push(ParsedCommand::MoveTo(x, y));
                current_cmd = 'L';
            }
            'm' => {
                let dx = parse_number(&mut chars);
                let dy = parse_number(&mut chars);
                cx += dx;
                cy += dy;
                commands.push(ParsedCommand::MoveTo(cx, cy));
                current_cmd = 'l';
            }
            'L' => {
                let x = parse_number(&mut chars);
                let y = parse_number(&mut chars);
                cx = x;
                cy = y;
                commands.push(ParsedCommand::LineTo(x, y));
            }
            'l' => {
                let dx = parse_number(&mut chars);
                let dy = parse_number(&mut chars);
                cx += dx;
                cy += dy;
                commands.push(ParsedCommand::LineTo(cx, cy));
            }
            'H' => {
                let x = parse_number(&mut chars);
                cx = x;
                commands.push(ParsedCommand::HLineTo(x));
            }
            'h' => {
                let dx = parse_number(&mut chars);
                cx += dx;
                commands.push(ParsedCommand::HLineTo(cx));
            }
            'V' => {
                let y = parse_number(&mut chars);
                cy = y;
                commands.push(ParsedCommand::VLineTo(y));
            }
            'v' => {
                let dy = parse_number(&mut chars);
                cy += dy;
                commands.push(ParsedCommand::VLineTo(cy));
            }
            'C' => {
                let x1 = parse_number(&mut chars);
                let y1 = parse_number(&mut chars);
                let x2 = parse_number(&mut chars);
                let y2 = parse_number(&mut chars);
                let x = parse_number(&mut chars);
                let y = parse_number(&mut chars);
                cx = x;
                cy = y;
                commands.push(ParsedCommand::CurveTo(x1, y1, x2, y2, x, y));
            }
            'c' => {
                let dx1 = parse_number(&mut chars);
                let dy1 = parse_number(&mut chars);
                let dx2 = parse_number(&mut chars);
                let dy2 = parse_number(&mut chars);
                let dx = parse_number(&mut chars);
                let dy = parse_number(&mut chars);
                commands.push(ParsedCommand::CurveTo(
                    cx + dx1,
                    cy + dy1,
                    cx + dx2,
                    cy + dy2,
                    cx + dx,
                    cy + dy,
                ));
                cx += dx;
                cy += dy;
            }
            'Q' => {
                let x1 = parse_number(&mut chars);
                let y1 = parse_number(&mut chars);
                let x = parse_number(&mut chars);
                let y = parse_number(&mut chars);
                cx = x;
                cy = y;
                commands.push(ParsedCommand::QuadTo(x1, y1, x, y));
            }
            'q' => {
                let dx1 = parse_number(&mut chars);
                let dy1 = parse_number(&mut chars);
                let dx = parse_number(&mut chars);
                let dy = parse_number(&mut chars);
                commands.push(ParsedCommand::QuadTo(cx + dx1, cy + dy1, cx + dx, cy + dy));
                cx += dx;
                cy += dy;
            }
            'Z' | 'z' => {
                commands.push(ParsedCommand::Close);
            }
            'A' => {
                let rx = parse_number(&mut chars);
                let ry = parse_number(&mut chars);
                let x_rotation = parse_number(&mut chars);
                let large_arc = parse_number(&mut chars) != 0.0;
                let sweep = parse_number(&mut chars) != 0.0;
                let x = parse_number(&mut chars);
                let y = parse_number(&mut chars);

                let curves = arc_to_beziers(cx, cy, rx, ry, x_rotation, large_arc, sweep, x, y);
                for (x1, y1, x2, y2, ex, ey) in curves {
                    commands.push(ParsedCommand::CurveTo(x1, y1, x2, y2, ex, ey));
                }
                cx = x;
                cy = y;
            }
            'a' => {
                let rx = parse_number(&mut chars);
                let ry = parse_number(&mut chars);
                let x_rotation = parse_number(&mut chars);
                let large_arc = parse_number(&mut chars) != 0.0;
                let sweep = parse_number(&mut chars) != 0.0;
                let dx = parse_number(&mut chars);
                let dy = parse_number(&mut chars);
                let x = cx + dx;
                let y = cy + dy;

                let curves = arc_to_beziers(cx, cy, rx, ry, x_rotation, large_arc, sweep, x, y);
                for (x1, y1, x2, y2, ex, ey) in curves {
                    commands.push(ParsedCommand::CurveTo(x1, y1, x2, y2, ex, ey));
                }
                cx = x;
                cy = y;
            }
            _ => {
                if chars.peek().is_none() {
                    break;
                }
                chars.next();
            }
        }
    }

    commands
}

fn parse_number<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> f32 {
    while chars
        .peek()
        .map(|c| *c == ',' || c.is_whitespace())
        .unwrap_or(false)
    {
        chars.next();
    }

    let mut s = String::new();

    if chars.peek() == Some(&'-') {
        s.push(chars.next().unwrap());
    } else if chars.peek() == Some(&'+') {
        chars.next();
    }

    let mut has_dot = false;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            s.push(chars.next().unwrap());
        } else if c == '.' {
            if has_dot {
                break;
            }
            has_dot = true;
            s.push(chars.next().unwrap());
        } else {
            break;
        }
    }

    if chars.peek() == Some(&'e') || chars.peek() == Some(&'E') {
        s.push(chars.next().unwrap());
        if chars.peek() == Some(&'-') || chars.peek() == Some(&'+') {
            s.push(chars.next().unwrap());
        }
        while chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            s.push(chars.next().unwrap());
        }
    }

    s.parse()
        .unwrap_or_else(|_| panic!("Failed to parse number: '{}'", s))
}

pub fn generate_svg_icon_path(args: &SvgIconArgs) -> Result<TokenStream, String> {
    let icon_path = args.path.value();
    let body = get_svg_body(&icon_path)?;
    let commands = parse_svg_body(&body);

    if commands.is_empty() {
        return Err(format!("No path commands found in icon '{}'", icon_path));
    }

    let path_ops = generate_path_building_ops(&commands);

    let size_expr = &args.size;
    let cx_expr = &args.cx;
    let cy_expr = &args.cy;

    Ok(quote! {
        {
            use tiny_skia::PathBuilder;

            let __icon_size: f32 = #size_expr;
            let __icon_cx: f32 = #cx_expr;
            let __icon_cy: f32 = #cy_expr;


            let __icon_scale = __icon_size / 24.0;
            let __icon_offset_x = __icon_cx - 12.0 * __icon_scale;
            let __icon_offset_y = __icon_cy - 12.0 * __icon_scale;


            #[allow(unused_assignments)]
            let mut __icon_cur_x = 0.0_f32;
            #[allow(unused_assignments)]
            let mut __icon_cur_y = 0.0_f32;

            let mut __pb = PathBuilder::new();
            #path_ops
            __pb.finish()
        }
    })
}

fn generate_path_building_ops(commands: &[ParsedCommand]) -> TokenStream {
    let ops: Vec<TokenStream> = commands.iter().map(|cmd| {
        match cmd {
            ParsedCommand::MoveTo(x, y) => quote! {
                {
                    __icon_cur_x = #x * __icon_scale + __icon_offset_x;
                    __icon_cur_y = #y * __icon_scale + __icon_offset_y;
                    __pb.move_to(__icon_cur_x, __icon_cur_y);
                }
            },
            ParsedCommand::LineTo(x, y) => quote! {
                {
                    __icon_cur_x = #x * __icon_scale + __icon_offset_x;
                    __icon_cur_y = #y * __icon_scale + __icon_offset_y;
                    __pb.line_to(__icon_cur_x, __icon_cur_y);
                }
            },
            ParsedCommand::HLineTo(x) => quote! {
                {
                    __icon_cur_x = #x * __icon_scale + __icon_offset_x;
                    __pb.line_to(__icon_cur_x, __icon_cur_y);
                }
            },
            ParsedCommand::VLineTo(y) => quote! {
                {
                    __icon_cur_y = #y * __icon_scale + __icon_offset_y;
                    __pb.line_to(__icon_cur_x, __icon_cur_y);
                }
            },
            ParsedCommand::CurveTo(x1, y1, x2, y2, x, y) => quote! {
                {
                    __pb.cubic_to(
                        #x1 * __icon_scale + __icon_offset_x, #y1 * __icon_scale + __icon_offset_y,
                        #x2 * __icon_scale + __icon_offset_x, #y2 * __icon_scale + __icon_offset_y,
                        #x * __icon_scale + __icon_offset_x, #y * __icon_scale + __icon_offset_y
                    );
                    __icon_cur_x = #x * __icon_scale + __icon_offset_x;
                    __icon_cur_y = #y * __icon_scale + __icon_offset_y;
                }
            },
            ParsedCommand::QuadTo(x1, y1, x, y) => quote! {
                {
                    __pb.quad_to(
                        #x1 * __icon_scale + __icon_offset_x, #y1 * __icon_scale + __icon_offset_y,
                        #x * __icon_scale + __icon_offset_x, #y * __icon_scale + __icon_offset_y
                    );
                    __icon_cur_x = #x * __icon_scale + __icon_offset_x;
                    __icon_cur_y = #y * __icon_scale + __icon_offset_y;
                }
            },
            ParsedCommand::Circle(cx, cy, r) => {

                quote! {
                    {
                        let __ccx = #cx * __icon_scale + __icon_offset_x;
                        let __ccy = #cy * __icon_scale + __icon_offset_y;
                        let __cr = #r * __icon_scale;
                        let __ck = 0.5522847498 * __cr;
                        __pb.move_to(__ccx + __cr, __ccy);
                        __pb.cubic_to(__ccx + __cr, __ccy + __ck, __ccx + __ck, __ccy + __cr, __ccx, __ccy + __cr);
                        __pb.cubic_to(__ccx - __ck, __ccy + __cr, __ccx - __cr, __ccy + __ck, __ccx - __cr, __ccy);
                        __pb.cubic_to(__ccx - __cr, __ccy - __ck, __ccx - __ck, __ccy - __cr, __ccx, __ccy - __cr);
                        __pb.cubic_to(__ccx + __ck, __ccy - __cr, __ccx + __cr, __ccy - __ck, __ccx + __cr, __ccy);
                        __pb.close();
                        __icon_cur_x = __ccx + __cr;
                        __icon_cur_y = __ccy;
                    }
                }
            },
            ParsedCommand::Close => quote! { __pb.close(); },
        }
    }).collect();

    quote! { #(#ops)* }
}
