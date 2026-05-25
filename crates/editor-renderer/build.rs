use proc_macro2::TokenStream;
use quote::quote;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let icon_output = generate_icon_data();
    let out_dir = env::var("OUT_DIR").unwrap();
    let icon_dest = Path::new(&out_dir).join("icon_data.rs");
    fs::write(&icon_dest, icon_output.to_string()).unwrap();
}

#[derive(Debug, Clone)]
enum SvgCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    CurveTo(f32, f32, f32, f32, f32, f32),
    Close,
}

fn parse_svg_number<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> f32 {
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
        .unwrap_or_else(|_| panic!("Failed to parse number: '{s}'"))
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
    let (sin_phi, cos_phi) = (phi.sin(), phi.cos());
    let dx = (x1 - x2) / 2.0;
    let dy = (y1 - y2) / 2.0;
    let x1p = cos_phi * dx + sin_phi * dy;
    let y1p = -sin_phi * dx + cos_phi * dy;
    let lambda = x1p * x1p / (rx * rx) + y1p * y1p / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let (rx2, ry2) = (rx * rx, ry * ry);
    let (x1p2, y1p2) = (x1p * x1p, y1p * y1p);
    let sq = ((rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2) / (rx2 * y1p2 + ry2 * x1p2))
        .max(0.0)
        .sqrt();
    let sq = if large_arc == sweep { -sq } else { sq };
    let cxp = sq * rx * y1p / ry;
    let cyp = -sq * ry * x1p / rx;
    let cx = cos_phi * cxp - sin_phi * cyp + (x1 + x2) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (y1 + y2) / 2.0;
    let ux = (x1p - cxp) / rx;
    let uy = (y1p - cyp) / ry;
    let n = (ux * ux + uy * uy).sqrt();
    let theta1 = (ux / n).acos() * if uy < 0.0 { -1.0 } else { 1.0 };
    let vx = (-x1p - cxp) / rx;
    let vy = (-y1p - cyp) / ry;
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
    let num_segments = (dtheta.abs() / (std::f32::consts::PI / 2.0))
        .ceil()
        .max(1.0) as usize;
    let delta = dtheta / num_segments as f32;
    let t = (delta / 4.0).tan();
    let alpha = delta.sin() * ((4.0 + 3.0 * t * t).sqrt() - 1.0) / 3.0;
    let mut theta = theta1;
    let (mut px, mut py) = (x1, y1);
    for _ in 0..num_segments {
        let (ct, st) = (theta.cos(), theta.sin());
        let (ct2, st2) = ((theta + delta).cos(), (theta + delta).sin());
        let ex = cx + rx * cos_phi * ct2 - ry * sin_phi * st2;
        let ey = cy + rx * sin_phi * ct2 + ry * cos_phi * st2;
        let dx1 = -rx * cos_phi * st - ry * sin_phi * ct;
        let dy1 = -rx * sin_phi * st + ry * cos_phi * ct;
        let dx2 = -rx * cos_phi * st2 - ry * sin_phi * ct2;
        let dy2 = -rx * sin_phi * st2 + ry * cos_phi * ct2;
        result.push((
            px + alpha * dx1,
            py + alpha * dy1,
            ex - alpha * dx2,
            ey - alpha * dy2,
            ex,
            ey,
        ));
        theta += delta;
        px = ex;
        py = ey;
    }
    result
}

fn parse_svg_path_d(d: &str) -> Vec<SvgCommand> {
    let mut commands = Vec::new();
    let mut chars = d.chars().peekable();
    let mut cmd = ' ';
    let mut cx = 0.0_f32;
    let mut cy = 0.0_f32;

    while chars.peek().is_some() {
        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            chars.next();
        }
        if let Some(&c) = chars.peek()
            && c.is_ascii_alphabetic()
        {
            cmd = c;
            chars.next();
        }
        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            chars.next();
        }
        if chars.peek().is_none() && !matches!(cmd, 'Z' | 'z') {
            break;
        }

        match cmd {
            'M' => {
                let x = parse_svg_number(&mut chars);
                let y = parse_svg_number(&mut chars);
                cx = x;
                cy = y;
                commands.push(SvgCommand::MoveTo(x, y));
                cmd = 'L';
            }
            'm' => {
                let dx = parse_svg_number(&mut chars);
                let dy = parse_svg_number(&mut chars);
                cx += dx;
                cy += dy;
                commands.push(SvgCommand::MoveTo(cx, cy));
                cmd = 'l';
            }
            'L' => {
                let x = parse_svg_number(&mut chars);
                let y = parse_svg_number(&mut chars);
                cx = x;
                cy = y;
                commands.push(SvgCommand::LineTo(x, y));
            }
            'l' => {
                let dx = parse_svg_number(&mut chars);
                let dy = parse_svg_number(&mut chars);
                cx += dx;
                cy += dy;
                commands.push(SvgCommand::LineTo(cx, cy));
            }
            'H' => {
                let x = parse_svg_number(&mut chars);
                cx = x;
                commands.push(SvgCommand::LineTo(cx, cy));
            }
            'h' => {
                let dx = parse_svg_number(&mut chars);
                cx += dx;
                commands.push(SvgCommand::LineTo(cx, cy));
            }
            'V' => {
                let y = parse_svg_number(&mut chars);
                cy = y;
                commands.push(SvgCommand::LineTo(cx, cy));
            }
            'v' => {
                let dy = parse_svg_number(&mut chars);
                cy += dy;
                commands.push(SvgCommand::LineTo(cx, cy));
            }
            'C' => {
                let x1 = parse_svg_number(&mut chars);
                let y1 = parse_svg_number(&mut chars);
                let x2 = parse_svg_number(&mut chars);
                let y2 = parse_svg_number(&mut chars);
                let x = parse_svg_number(&mut chars);
                let y = parse_svg_number(&mut chars);
                cx = x;
                cy = y;
                commands.push(SvgCommand::CurveTo(x1, y1, x2, y2, x, y));
            }
            'c' => {
                let dx1 = parse_svg_number(&mut chars);
                let dy1 = parse_svg_number(&mut chars);
                let dx2 = parse_svg_number(&mut chars);
                let dy2 = parse_svg_number(&mut chars);
                let dx = parse_svg_number(&mut chars);
                let dy = parse_svg_number(&mut chars);
                commands.push(SvgCommand::CurveTo(
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
                let x1 = parse_svg_number(&mut chars);
                let y1 = parse_svg_number(&mut chars);
                let x = parse_svg_number(&mut chars);
                let y = parse_svg_number(&mut chars);
                cx = x;
                cy = y;
                commands.push(SvgCommand::QuadTo(x1, y1, x, y));
            }
            'q' => {
                let dx1 = parse_svg_number(&mut chars);
                let dy1 = parse_svg_number(&mut chars);
                let dx = parse_svg_number(&mut chars);
                let dy = parse_svg_number(&mut chars);
                commands.push(SvgCommand::QuadTo(cx + dx1, cy + dy1, cx + dx, cy + dy));
                cx += dx;
                cy += dy;
            }
            'A' => {
                let rx = parse_svg_number(&mut chars);
                let ry = parse_svg_number(&mut chars);
                let rot = parse_svg_number(&mut chars);
                let la = parse_svg_number(&mut chars) != 0.0;
                let sw = parse_svg_number(&mut chars) != 0.0;
                let x = parse_svg_number(&mut chars);
                let y = parse_svg_number(&mut chars);
                for (x1, y1, x2, y2, ex, ey) in arc_to_beziers(cx, cy, rx, ry, rot, la, sw, x, y) {
                    commands.push(SvgCommand::CurveTo(x1, y1, x2, y2, ex, ey));
                }
                cx = x;
                cy = y;
            }
            'a' => {
                let rx = parse_svg_number(&mut chars);
                let ry = parse_svg_number(&mut chars);
                let rot = parse_svg_number(&mut chars);
                let la = parse_svg_number(&mut chars) != 0.0;
                let sw = parse_svg_number(&mut chars) != 0.0;
                let dx = parse_svg_number(&mut chars);
                let dy = parse_svg_number(&mut chars);
                let x = cx + dx;
                let y = cy + dy;
                for (x1, y1, x2, y2, ex, ey) in arc_to_beziers(cx, cy, rx, ry, rot, la, sw, x, y) {
                    commands.push(SvgCommand::CurveTo(x1, y1, x2, y2, ex, ey));
                }
                cx = x;
                cy = y;
            }
            'Z' | 'z' => {
                commands.push(SvgCommand::Close);
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

fn svg_command_to_tokens(cmd: &SvgCommand) -> TokenStream {
    match cmd {
        SvgCommand::MoveTo(x, y) => quote! { PathElement::MoveTo { x: #x, y: #y } },
        SvgCommand::LineTo(x, y) => quote! { PathElement::LineTo { x: #x, y: #y } },
        SvgCommand::QuadTo(x1, y1, x, y) => {
            quote! { PathElement::QuadTo { x1: #x1, y1: #y1, x: #x, y: #y } }
        }
        SvgCommand::CurveTo(x1, y1, x2, y2, x, y) => {
            quote! { PathElement::CurveTo { x1: #x1, y1: #y1, x2: #x2, y2: #y2, x: #x, y: #y } }
        }
        SvgCommand::Close => quote! { PathElement::Close },
    }
}

fn generate_icon_data() -> TokenStream {
    println!("cargo:rerun-if-changed=icons.toml");
    println!("cargo:rerun-if-changed=../../assets/icons.json");

    let toml_str = fs::read_to_string("icons.toml").expect("failed to read icons.toml");
    let toml: toml::Value = toml::from_str(&toml_str).expect("failed to parse icons.toml");
    let toml_table = toml.as_table().expect("icons.toml must be a table");

    let mut needed: Vec<String> = Vec::new();
    for (section, value) in toml_table {
        let icons = value
            .get("icons")
            .and_then(|v| v.as_array())
            .expect("each section must have an icons array");
        for icon in icons {
            let name = icon.as_str().expect("icon name must be a string");
            needed.push(format!("{section}/{name}"));
        }
    }

    let json_str =
        fs::read_to_string("../../assets/icons.json").expect("failed to read icons.json");
    let json: serde_json::Value =
        serde_json::from_str(&json_str).expect("failed to parse icons.json");
    let json_obj = json.as_object().expect("icons.json must be an object");

    let entries: Vec<TokenStream> = needed
        .iter()
        .map(|icon_name| {
            let icon = json_obj
                .get(icon_name.as_str())
                .unwrap_or_else(|| panic!("icon '{icon_name}' not found in icons.json"));

            let vp = icon["viewport"].as_array().unwrap();
            let vw = vp[0].as_f64().unwrap() as f32;
            let vh = vp[1].as_f64().unwrap() as f32;

            let elements: Vec<TokenStream> = icon["elements"]
                .as_array()
                .unwrap()
                .iter()
                .map(|elem| {
                    let d = elem["d"].as_str().unwrap();
                    let commands = parse_svg_path_d(d);
                    let path_tokens: Vec<TokenStream> =
                        commands.iter().map(svg_command_to_tokens).collect();

                    let style = elem["style"].as_str().unwrap();
                    match style {
                        "fill" => {
                            let fill_rule = match elem.get("fill_rule").and_then(|v| v.as_str()) {
                                Some("evenodd") => quote! { FillRule::EvenOdd },
                                _ => quote! { FillRule::NonZero },
                            };
                            quote! {
                                IconElement::Fill {
                                    path: &[#(#path_tokens),*],
                                    fill_rule: #fill_rule,
                                }
                            }
                        }
                        "stroke" => {
                            let stroke_cap = match elem.get("stroke_cap").and_then(|v| v.as_str()) {
                                Some("round") => quote! { StrokeCap::Round },
                                Some("square") => quote! { StrokeCap::Square },
                                _ => quote! { StrokeCap::Butt },
                            };
                            let stroke_join = match elem.get("stroke_join").and_then(|v| v.as_str())
                            {
                                Some("round") => quote! { StrokeJoin::Round },
                                Some("bevel") => quote! { StrokeJoin::Bevel },
                                _ => quote! { StrokeJoin::Miter },
                            };
                            quote! {
                                IconElement::Stroke {
                                    path: &[#(#path_tokens),*],
                                    stroke_cap: #stroke_cap,
                                    stroke_join: #stroke_join,
                                }
                            }
                        }
                        other => panic!("unknown style: {other}"),
                    }
                })
                .collect();

            let name = icon_name.as_str();
            quote! { #name => IconData { viewport: (#vw, #vh), elements: &[#(#elements),*] } }
        })
        .collect();

    quote! {
        pub static ICONS: phf::Map<&'static str, IconData> = phf::phf_map! {
            #(#entries,)*
        };
    }
}
