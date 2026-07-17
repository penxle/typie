use kurbo::{BezPath, CubicBez, PathEl, Point, Shape};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::types::MajorMinor;
use skrifa::raw::{FontRef, TableProvider};
use skrifa::{GlyphId, MetadataProvider, Tag};
use write_fonts::FontBuilder;
use write_fonts::from_obj::ToOwnedTable;
use write_fonts::tables::glyf::{Bbox, Glyf, GlyfLocaBuilder, Glyph, SimpleGlyph};
use write_fonts::tables::hmtx::{Hmtx, LongMetric};
use write_fonts::tables::loca::{Loca, LocaFormat};
use write_fonts::tables::maxp::Maxp;
use write_fonts::types::FWord;

use crate::ServerError;

struct BezPathPen(BezPath);

impl OutlinePen for BezPathPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to((f64::from(x), f64::from(y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to((f64::from(x), f64::from(y)));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.quad_to(
            (f64::from(cx0), f64::from(cy0)),
            (f64::from(x), f64::from(y)),
        );
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.curve_to(
            (f64::from(cx0), f64::from(cy0)),
            (f64::from(cx1), f64::from(cy1)),
            (f64::from(x), f64::from(y)),
        );
    }

    fn close(&mut self) {
        self.0.close_path();
    }
}

fn extract_glyph_paths(font: &FontRef) -> Result<Vec<BezPath>, ServerError> {
    let num_glyphs = font
        .maxp()
        .map_err(|e| ServerError::InvalidFont(e.to_string()))?
        .num_glyphs();
    let outlines = font.outline_glyphs();
    let mut paths = Vec::with_capacity(num_glyphs as usize);
    for gid in 0..num_glyphs {
        let glyph = outlines
            .get(GlyphId::new(u32::from(gid)))
            .ok_or_else(|| ServerError::InvalidFont(format!("missing outline for glyph {gid}")))?;
        let mut pen = BezPathPen(BezPath::new());
        glyph
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut pen,
            )
            .map_err(|e| ServerError::InvalidFont(format!("failed to draw glyph {gid}: {e}")))?;
        paths.push(pen.0);
    }
    Ok(paths)
}

fn quadify_path(path: &BezPath, accuracy: f64) -> Option<BezPath> {
    let mut out = BezPath::new();
    let mut start = Point::ZERO;
    let mut last = Point::ZERO;
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                out.move_to(p);
                start = p;
                last = p;
            }
            PathEl::LineTo(p) => {
                out.line_to(p);
                last = p;
            }
            PathEl::QuadTo(p1, p2) => {
                out.quad_to(p1, p2);
                last = p2;
            }
            PathEl::CurveTo(p1, p2, p3) => {
                let spline = CubicBez::new(last, p1, p2, p3).approx_spline(accuracy)?;
                for quad in spline.to_quads() {
                    out.quad_to(quad.p1, quad.p2);
                }
                last = p3;
            }
            PathEl::ClosePath => {
                out.close_path();
                last = start;
            }
        }
    }
    Some(out)
}

fn glyphs_from_paths(
    paths: &[BezPath],
    accuracy: f64,
) -> Result<Vec<Option<SimpleGlyph>>, ServerError> {
    paths
        .iter()
        .enumerate()
        .map(|(gid, path)| {
            if path.elements().is_empty() {
                return Ok(None);
            }
            let quads = quadify_path(path, accuracy).ok_or_else(|| {
                ServerError::InvalidFont(format!("failed to quadify glyph {gid}"))
            })?;
            let coord_ok =
                |p: Point| (-32768.5..32767.5).contains(&p.x) && (-32768.5..32767.5).contains(&p.y);
            let in_range = quads.elements().iter().all(|el| match *el {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => coord_ok(p),
                PathEl::QuadTo(p1, p2) => coord_ok(p1) && coord_ok(p2),
                PathEl::CurveTo(p1, p2, p3) => coord_ok(p1) && coord_ok(p2) && coord_ok(p3),
                PathEl::ClosePath => true,
            });
            if !in_range {
                return Err(ServerError::InvalidFont(format!(
                    "glyph {gid} has out-of-range coordinates"
                )));
            }
            SimpleGlyph::from_bezpath(&quads).map(Some).map_err(|e| {
                ServerError::InvalidFont(format!("failed to build glyph {gid}: {e:?}"))
            })
        })
        .collect()
}

struct BuiltOutlines {
    glyf: Glyf,
    loca: Loca,
    loca_format: LocaFormat,
    maxp: Maxp,
    bboxes: Vec<Option<Bbox>>,
}

fn build_outline_tables(glyphs: &[Option<SimpleGlyph>]) -> Result<BuiltOutlines, ServerError> {
    let mut builder = GlyfLocaBuilder::new();
    let mut max_points = 0u16;
    let mut max_contours = 0u16;
    let mut bboxes = Vec::with_capacity(glyphs.len());
    for (gid, glyph) in glyphs.iter().enumerate() {
        match glyph {
            Some(g) => {
                let points: usize = g.contours.iter().map(|c| c.len()).sum();
                if points > usize::from(u16::MAX) {
                    return Err(ServerError::InvalidFont(format!(
                        "glyph {gid} exceeds glyf point limit: {points}"
                    )));
                }
                if g.contours.len() > 32_766 {
                    return Err(ServerError::InvalidFont(format!(
                        "glyph {gid} exceeds glyf contour limit: {}",
                        g.contours.len()
                    )));
                }
                let mut last = (0i32, 0i32);
                for pt in g.contours.iter().flat_map(|c| c.iter()) {
                    let dx = i32::from(pt.x) - last.0;
                    let dy = i32::from(pt.y) - last.1;
                    if !(-32_768..=32_767).contains(&dx) || !(-32_768..=32_767).contains(&dy) {
                        return Err(ServerError::InvalidFont(format!(
                            "glyph {gid} exceeds glyf delta limit"
                        )));
                    }
                    last = (i32::from(pt.x), i32::from(pt.y));
                }
                max_points = max_points.max(points as u16);
                max_contours = max_contours.max(g.contours.len() as u16);
                bboxes.push(Some(g.bbox));
                builder.add_glyph(g).map_err(|e| {
                    ServerError::InvalidFont(format!("failed to compile glyph {gid}: {e}"))
                })?;
            }
            None => {
                bboxes.push(None);
                builder.add_glyph(&Glyph::Empty).map_err(|e| {
                    ServerError::InvalidFont(format!("failed to compile glyph {gid}: {e}"))
                })?;
            }
        }
    }
    let (glyf, loca, loca_format) = builder.build();
    let maxp = Maxp {
        num_glyphs: glyphs.len() as u16,
        max_points: Some(max_points),
        max_contours: Some(max_contours),
        max_composite_points: Some(0),
        max_composite_contours: Some(0),
        max_zones: Some(1),
        max_twilight_points: Some(0),
        max_storage: Some(0),
        max_function_defs: Some(0),
        max_instruction_defs: Some(0),
        max_stack_elements: Some(0),
        max_size_of_instructions: Some(0),
        max_component_elements: Some(0),
        max_component_depth: Some(0),
    };
    Ok(BuiltOutlines {
        glyf,
        loca,
        loca_format,
        maxp,
        bboxes,
    })
}

const DROPPED_TABLES: [Tag; 17] = [
    Tag::new(b"CFF "),
    Tag::new(b"CFF2"),
    Tag::new(b"VORG"),
    Tag::new(b"DSIG"),
    Tag::new(b"fvar"),
    Tag::new(b"avar"),
    Tag::new(b"MVAR"),
    Tag::new(b"HVAR"),
    Tag::new(b"VVAR"),
    Tag::new(b"fpgm"),
    Tag::new(b"prep"),
    Tag::new(b"cvt "),
    Tag::new(b"cvar"),
    Tag::new(b"gvar"),
    Tag::new(b"hdmx"),
    Tag::new(b"LTSH"),
    Tag::new(b"VDMX"),
];

const REBUILT_TABLES: [Tag; 8] = [
    Tag::new(b"glyf"),
    Tag::new(b"loca"),
    Tag::new(b"maxp"),
    Tag::new(b"head"),
    Tag::new(b"hmtx"),
    Tag::new(b"hhea"),
    Tag::new(b"vmtx"),
    Tag::new(b"vhea"),
];

pub(crate) fn convert_to_glyf(font: &FontRef) -> Result<Vec<u8>, ServerError> {
    if font.table_data(Tag::new(b"CFF2")).is_some() {
        return Err(ServerError::InvalidFont(
            "unsplittable font: CFF2 unsupported".into(),
        ));
    }
    if font.table_data(Tag::new(b"VARC")).is_some() {
        return Err(ServerError::InvalidFont(
            "unsplittable font: VARC unsupported".into(),
        ));
    }
    if font.table_data(Tag::new(b"fvar")).is_some() {
        return Err(ServerError::InvalidFont(
            "unsplittable font: variable CFF unsupported".into(),
        ));
    }

    let head = font
        .head()
        .map_err(|e| ServerError::InvalidFont(e.to_string()))?;
    let accuracy = f64::from(head.units_per_em()) / 1000.0;

    let paths = extract_glyph_paths(font)?;
    let glyphs = glyphs_from_paths(&paths, accuracy)?;
    let built = build_outline_tables(&glyphs)?;
    let num_glyphs = glyphs.len() as u16;

    let mut new_head: write_fonts::tables::head::Head = head.to_owned_table();
    new_head.index_to_loc_format = match built.loca_format {
        LocaFormat::Short => 0,
        LocaFormat::Long => 1,
    };
    let mut global: Option<Bbox> = None;
    for bbox in built.bboxes.iter().flatten() {
        global = Some(match global {
            Some(g) => Bbox {
                x_min: g.x_min.min(bbox.x_min),
                y_min: g.y_min.min(bbox.y_min),
                x_max: g.x_max.max(bbox.x_max),
                y_max: g.y_max.max(bbox.y_max),
            },
            None => *bbox,
        });
    }
    if let Some(g) = global {
        new_head.x_min = g.x_min;
        new_head.y_min = g.y_min;
        new_head.x_max = g.x_max;
        new_head.y_max = g.y_max;
    } else {
        new_head.x_min = 0;
        new_head.y_min = 0;
        new_head.x_max = 0;
        new_head.y_max = 0;
    }

    let orig_hmtx = font
        .hmtx()
        .map_err(|e| ServerError::InvalidFont(e.to_string()))?;
    let metrics = orig_hmtx.h_metrics();
    if metrics.is_empty() || metrics.len() > glyphs.len() {
        return Err(ServerError::InvalidFont("malformed hmtx".into()));
    }
    let lsb_of = |gid: usize| built.bboxes[gid].map(|b| b.x_min).unwrap_or(0);
    let h_metrics: Vec<LongMetric> = metrics
        .iter()
        .enumerate()
        .map(|(gid, m)| LongMetric {
            advance: m.advance(),
            side_bearing: lsb_of(gid),
        })
        .collect();
    let left_side_bearings: Vec<i16> = (metrics.len()..glyphs.len()).map(lsb_of).collect();
    let new_hmtx = Hmtx {
        h_metrics,
        left_side_bearings,
    };

    let mut new_hhea: write_fonts::tables::hhea::Hhea = font
        .hhea()
        .map_err(|e| ServerError::InvalidFont(e.to_string()))?
        .to_owned_table();
    let advance_of = |gid: usize| -> i32 {
        metrics
            .get(gid)
            .or_else(|| metrics.last())
            .map(|m| i32::from(m.advance()))
            .unwrap_or(0)
    };
    let mut min_lsb = i16::MAX;
    let mut min_rsb = i32::MAX;
    let mut max_extent = i16::MIN;
    let mut has_outline = false;
    for (gid, bbox) in built.bboxes.iter().enumerate() {
        let Some(b) = bbox else { continue };
        has_outline = true;
        min_lsb = min_lsb.min(b.x_min);
        min_rsb = min_rsb.min(advance_of(gid) - i32::from(b.x_max));
        max_extent = max_extent.max(b.x_max);
    }
    if has_outline {
        new_hhea.min_left_side_bearing = FWord::new(min_lsb);
        new_hhea.min_right_side_bearing = FWord::new(i16::try_from(min_rsb).map_err(|_| {
            ServerError::InvalidFont(format!("minRightSideBearing not representable: {min_rsb}"))
        })?);
        new_hhea.x_max_extent = FWord::new(max_extent);
    } else {
        new_hhea.min_left_side_bearing = FWord::new(0);
        new_hhea.min_right_side_bearing = FWord::new(0);
        new_hhea.x_max_extent = FWord::new(0);
    }

    let vorg = if font.table_data(Tag::new(b"VORG")).is_some() {
        let v = font
            .vorg()
            .map_err(|e| ServerError::InvalidFont(format!("malformed VORG: {e}")))?;
        if v.version() != MajorMinor::VERSION_1_0 {
            return Err(ServerError::InvalidFont("unsupported VORG version".into()));
        }
        let metrics = v.vert_origin_y_metrics();
        if metrics.len() != usize::from(v.num_vert_origin_y_metrics()) {
            return Err(ServerError::InvalidFont("malformed VORG".into()));
        }
        if !metrics
            .windows(2)
            .all(|w| w[0].glyph_index() < w[1].glyph_index())
        {
            return Err(ServerError::InvalidFont("VORG not sorted".into()));
        }
        Some(v)
    } else {
        None
    };

    let vertical = if font.table_data(Tag::new(b"vmtx")).is_some() {
        let orig_vmtx = font
            .vmtx()
            .map_err(|e| ServerError::InvalidFont(format!("malformed vmtx: {e}")))?;
        let v_metrics_orig = orig_vmtx.v_metrics();
        if v_metrics_orig.is_empty()
            || v_metrics_orig.len() + orig_vmtx.top_side_bearings().len() != glyphs.len()
        {
            return Err(ServerError::InvalidFont("malformed vmtx".into()));
        }
        let orig_tsb = |gid: usize| {
            orig_vmtx
                .side_bearing(GlyphId::new(gid as u32))
                .unwrap_or(0)
        };
        let origin_y = |gid: usize| -> Option<i32> {
            if let Some(v) = &vorg {
                let metrics = v.vert_origin_y_metrics();
                let found = metrics
                    .binary_search_by(|m| m.glyph_index().to_u32().cmp(&(gid as u32)))
                    .ok()
                    .map(|i| i32::from(metrics[i].vert_origin_y()));
                Some(found.unwrap_or_else(|| i32::from(v.default_vert_origin_y())))
            } else if paths[gid].elements().is_empty() {
                None
            } else {
                Some(i32::from(orig_tsb(gid)) + (paths[gid].bounding_box().y1 + 0.5).floor() as i32)
            }
        };
        let mut new_tsbs: Vec<i16> = Vec::with_capacity(glyphs.len());
        for gid in 0..glyphs.len() {
            let tsb = match (built.bboxes[gid], origin_y(gid)) {
                (Some(b), Some(o)) => {
                    let v = o - i32::from(b.y_max);
                    i16::try_from(v).map_err(|_| {
                        ServerError::InvalidFont(format!(
                            "glyph {gid} vertical origin not representable: {v}"
                        ))
                    })?
                }
                _ => orig_tsb(gid),
            };
            new_tsbs.push(tsb);
        }
        let v_metrics: Vec<LongMetric> = v_metrics_orig
            .iter()
            .enumerate()
            .map(|(gid, m)| LongMetric {
                advance: m.advance(),
                side_bearing: new_tsbs[gid],
            })
            .collect();
        let top_side_bearings: Vec<i16> = new_tsbs[v_metrics_orig.len()..].to_vec();

        let mut new_vhea: write_fonts::tables::vhea::Vhea = font
            .vhea()
            .map_err(|e| ServerError::InvalidFont(format!("malformed vhea: {e}")))?
            .to_owned_table();
        let v_advance_of = |gid: usize| -> i32 {
            v_metrics_orig
                .get(gid)
                .or_else(|| v_metrics_orig.last())
                .map(|m| i32::from(m.advance()))
                .unwrap_or(0)
        };
        let mut min_tsb = i16::MAX;
        let mut min_bsb = i32::MAX;
        let mut max_v_extent = i32::from(i16::MIN);
        let mut has_v_outline = false;
        for (gid, bbox) in built.bboxes.iter().enumerate() {
            let Some(b) = bbox else { continue };
            has_v_outline = true;
            let tsb = new_tsbs[gid];
            let height = i32::from(b.y_max) - i32::from(b.y_min);
            min_tsb = min_tsb.min(tsb);
            min_bsb = min_bsb.min(v_advance_of(gid) - i32::from(tsb) - height);
            max_v_extent = max_v_extent.max(i32::from(tsb) + height);
        }
        if has_v_outline {
            new_vhea.min_top_side_bearing = FWord::new(min_tsb);
            new_vhea.min_bottom_side_bearing =
                FWord::new(i16::try_from(min_bsb).map_err(|_| {
                    ServerError::InvalidFont(format!(
                        "minBottomSideBearing not representable: {min_bsb}"
                    ))
                })?);
            new_vhea.y_max_extent = FWord::new(i16::try_from(max_v_extent).map_err(|_| {
                ServerError::InvalidFont(format!("yMaxExtent not representable: {max_v_extent}"))
            })?);
        } else {
            new_vhea.min_top_side_bearing = FWord::new(0);
            new_vhea.min_bottom_side_bearing = FWord::new(0);
            new_vhea.y_max_extent = FWord::new(0);
        }
        Some((
            write_fonts::tables::vmtx::Vmtx {
                v_metrics,
                top_side_bearings,
            },
            new_vhea,
        ))
    } else {
        None
    };

    let copied_tables = font
        .table_directory()
        .table_records()
        .iter()
        .filter(|r| {
            let tag = r.tag();
            !DROPPED_TABLES.contains(&tag) && !REBUILT_TABLES.contains(&tag)
        })
        .count();
    let rebuilt_tables = 6 + if vertical.is_some() { 2 } else { 0 };
    if copied_tables + rebuilt_tables > usize::from(u16::MAX) {
        return Err(ServerError::InvalidFont("too many tables".into()));
    }

    let mut builder = FontBuilder::new();
    builder
        .add_table(&built.glyf)
        .and_then(|b| b.add_table(&built.loca))
        .and_then(|b| b.add_table(&built.maxp))
        .and_then(|b| b.add_table(&new_head))
        .and_then(|b| b.add_table(&new_hmtx))
        .and_then(|b| b.add_table(&new_hhea))
        .map_err(|e| ServerError::EncodingFailed(e.to_string()))?;
    if let Some((new_vmtx, new_vhea)) = &vertical {
        builder
            .add_table(new_vmtx)
            .and_then(|b| b.add_table(new_vhea))
            .map_err(|e| ServerError::EncodingFailed(e.to_string()))?;
    }
    for record in font.table_directory().table_records() {
        let tag = record.tag();
        if DROPPED_TABLES.contains(&tag) || REBUILT_TABLES.contains(&tag) {
            continue;
        }
        if let Some(data) = font.table_data(tag) {
            builder.add_raw(tag, data.as_ref().to_vec());
        }
    }
    let out = builder.build();

    let rebuilt = FontRef::new(&out)
        .map_err(|e| ServerError::InvalidFont(format!("converted font unreadable: {e}")))?;
    rebuilt
        .glyf()
        .map_err(|e| ServerError::InvalidFont(format!("converted font missing glyf: {e}")))?;
    let rebuilt_count = rebuilt
        .maxp()
        .map_err(|e| ServerError::InvalidFont(e.to_string()))?
        .num_glyphs();
    if rebuilt_count != num_glyphs {
        return Err(ServerError::InvalidFont(format!(
            "glyph count mismatch after conversion: {rebuilt_count} != {num_glyphs}"
        )));
    }
    let rebuilt_hmtx = rebuilt
        .hmtx()
        .map_err(|e| ServerError::InvalidFont(e.to_string()))?;
    for gid in 0..num_glyphs {
        let id = GlyphId::new(u32::from(gid));
        if orig_hmtx.advance(id) != rebuilt_hmtx.advance(id) {
            return Err(ServerError::InvalidFont(format!(
                "advance mismatch after conversion at glyph {gid}"
            )));
        }
    }
    if vertical.is_some() {
        let orig_vmtx = font
            .vmtx()
            .map_err(|e| ServerError::InvalidFont(e.to_string()))?;
        let rebuilt_vmtx = rebuilt
            .vmtx()
            .map_err(|e| ServerError::InvalidFont(e.to_string()))?;
        for gid in 0..num_glyphs {
            let id = GlyphId::new(u32::from(gid));
            if orig_vmtx.advance(id) != rebuilt_vmtx.advance(id) {
                return Err(ServerError::InvalidFont(format!(
                    "vertical advance mismatch after conversion at glyph {gid}"
                )));
            }
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use kurbo::{ParamCurve, ParamCurveNearest, PathSeg};
    use proptest::prelude::*;

    use super::*;

    const CFF_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/SourceSans3-Regular.otf"
    ));

    const CFF2_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/SourceSans3VF-Upright.otf"
    ));

    #[test]
    fn fixtures_are_cff_without_glyf() {
        let font = FontRef::new(CFF_FIXTURE).unwrap();
        assert!(font.table_data(skrifa::Tag::new(b"CFF ")).is_some());
        assert!(font.glyf().is_err());

        let font2 = FontRef::new(CFF2_FIXTURE).unwrap();
        assert!(font2.table_data(skrifa::Tag::new(b"CFF2")).is_some());
        assert!(font2.glyf().is_err());
    }

    #[test]
    fn extract_paths_from_cff_fixture() {
        let font = FontRef::new(CFF_FIXTURE).unwrap();
        let num_glyphs = font.maxp().unwrap().num_glyphs();
        let paths = extract_glyph_paths(&font).unwrap();
        assert_eq!(paths.len(), num_glyphs as usize);

        let gid_a = font.charmap().map('A').unwrap().to_u32() as usize;
        assert!(!paths[gid_a].elements().is_empty());
        assert!(paths.iter().any(|p| {
            p.elements()
                .iter()
                .any(|el| matches!(el, PathEl::CurveTo(..)))
        }));

        let gid_space = font.charmap().map(' ').unwrap().to_u32() as usize;
        assert!(paths[gid_space].elements().is_empty());
    }

    fn contours_of(path: &BezPath) -> Vec<BezPath> {
        let mut out = Vec::new();
        let mut current = BezPath::new();
        for el in path.elements() {
            if matches!(el, PathEl::MoveTo(_)) && !current.elements().is_empty() {
                out.push(std::mem::replace(&mut current, BezPath::new()));
            }
            current.push(*el);
        }
        if !current.elements().is_empty() {
            out.push(current);
        }
        out
    }

    fn one_way_deviation(reference: &BezPath, candidate: &BezPath) -> f64 {
        let cand_segs: Vec<PathSeg> = candidate.segments().collect();
        let mut worst = 0.0f64;
        for seg in reference.segments() {
            for i in 0..=512 {
                let t = f64::from(i) / 512.0;
                let p = seg.eval(t);
                let d = cand_segs
                    .iter()
                    .map(|s| s.nearest(p, 1e-9).distance_sq)
                    .fold(f64::INFINITY, f64::min);
                worst = worst.max(d.sqrt());
            }
        }
        worst
    }

    fn max_deviation(a: &BezPath, b: &BezPath) -> f64 {
        let a_contours = contours_of(a);
        let b_contours = contours_of(b);
        assert_eq!(a_contours.len(), b_contours.len(), "contour count mismatch");
        a_contours
            .iter()
            .zip(&b_contours)
            .map(|(x, y)| one_way_deviation(x, y).max(one_way_deviation(y, x)))
            .fold(0.0, f64::max)
    }

    fn has_no_cubics(path: &BezPath) -> bool {
        path.elements()
            .iter()
            .all(|el| !matches!(el, PathEl::CurveTo(..)))
    }

    #[test]
    fn quadify_passes_through_lines_and_quads() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((100.0, 0.0));
        path.quad_to((150.0, 50.0), (100.0, 100.0));
        path.close_path();
        let out = quadify_path(&path, 1.0).unwrap();
        assert_eq!(out.elements(), path.elements());
    }

    #[test]
    fn quadify_converts_single_cubic_within_accuracy() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.curve_to((30.0, 100.0), (70.0, 100.0), (100.0, 0.0));
        let out = quadify_path(&path, 1.0).unwrap();
        assert!(has_no_cubics(&out));
        assert!(max_deviation(&path, &out) <= 1.0 + 1e-6);
    }

    #[test]
    fn quadify_handles_multiple_contours() {
        let mut path = BezPath::new();
        path.move_to((10.0, 10.0));
        path.curve_to((20.0, 60.0), (80.0, 60.0), (90.0, 10.0));
        path.close_path();
        path.move_to((200.0, 200.0));
        path.curve_to((210.0, 260.0), (280.0, 260.0), (290.0, 200.0));
        path.close_path();
        let out = quadify_path(&path, 1.0).unwrap();
        let first = out.elements().first().unwrap();
        assert!(matches!(first, PathEl::MoveTo(p) if *p == Point::new(10.0, 10.0)));
        assert_eq!(
            out.elements()
                .iter()
                .filter(|el| matches!(el, PathEl::MoveTo(_)))
                .count(),
            2
        );
    }

    proptest! {
        #[test]
        fn quadify_random_cubics_stay_within_accuracy(
            cubics in proptest::collection::vec(
                proptest::array::uniform8(-500.0..1500.0f64),
                1..5,
            ),
            closed in proptest::bool::ANY,
        ) {
            let accuracy = 1.0;
            let mut path = BezPath::new();
            path.move_to((cubics[0][0], cubics[0][1]));
            for c in &cubics {
                path.curve_to((c[2], c[3]), (c[4], c[5]), (c[6], c[7]));
            }
            if closed {
                path.close_path();
            }
            let out = quadify_path(&path, accuracy).expect("quadify must succeed");
            prop_assert!(has_no_cubics(&out));

            let last = cubics.last().unwrap();
            let last_drawn_point = out
                .elements()
                .iter()
                .rev()
                .find_map(|el| match *el {
                    PathEl::MoveTo(p) | PathEl::LineTo(p) => Some(p),
                    PathEl::QuadTo(_, p) => Some(p),
                    PathEl::CurveTo(_, _, p) => Some(p),
                    PathEl::ClosePath => None,
                })
                .unwrap();
            prop_assert_eq!(last_drawn_point, Point::new(last[6], last[7]));

            let skeleton = |p: &BezPath| {
                p.elements()
                    .iter()
                    .filter_map(|el| match el {
                        PathEl::MoveTo(_) => Some('M'),
                        PathEl::ClosePath => Some('Z'),
                        _ => None,
                    })
                    .collect::<String>()
            };
            prop_assert_eq!(skeleton(&path), skeleton(&out));

            let dev = max_deviation(&path, &out);
            prop_assert!(dev <= accuracy + 1e-6, "deviation {dev}");
        }
    }

    fn square_path() -> BezPath {
        let mut path = BezPath::new();
        path.move_to((10.0, 20.0));
        path.line_to((10.0, 120.0));
        path.line_to((110.0, 120.0));
        path.line_to((110.0, 20.0));
        path.close_path();
        path
    }

    #[test]
    fn glyphs_from_paths_builds_bbox_and_skips_empty() {
        let glyphs = glyphs_from_paths(&[square_path(), BezPath::new()], 1.0).unwrap();
        let g = glyphs[0].as_ref().unwrap();
        assert_eq!(
            (g.bbox.x_min, g.bbox.y_min, g.bbox.x_max, g.bbox.y_max),
            (10, 20, 110, 120)
        );
        assert!(glyphs[1].is_none());
    }

    #[test]
    fn build_outline_tables_computes_maxp_stats() {
        let glyphs = glyphs_from_paths(&[square_path(), BezPath::new()], 1.0).unwrap();
        let built = build_outline_tables(&glyphs).unwrap();
        assert_eq!(built.maxp.num_glyphs, 2);
        assert_eq!(built.maxp.max_points, Some(4));
        assert_eq!(built.maxp.max_contours, Some(1));
        assert_eq!(built.maxp.max_component_elements, Some(0));
        assert_eq!(built.maxp.max_zones, Some(1));
        assert_eq!(built.bboxes[0].unwrap().x_min, 10);
        assert!(built.bboxes[1].is_none());
    }

    #[test]
    fn build_outline_tables_rejects_point_overflow() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        for i in 1..=66_000u32 {
            path.line_to((f64::from(i % 1000), f64::from(i / 1000)));
        }
        path.close_path();
        let glyphs = glyphs_from_paths(&[path], 1.0).unwrap();
        assert!(matches!(
            build_outline_tables(&glyphs),
            Err(ServerError::InvalidFont(_))
        ));
    }

    #[test]
    fn glyphs_from_paths_rejects_out_of_range_coordinates() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((40_000.0, 0.0));
        path.line_to((0.0, 10.0));
        path.close_path();
        assert!(matches!(
            glyphs_from_paths(&[path], 1.0),
            Err(ServerError::InvalidFont(_))
        ));
    }

    #[test]
    fn build_outline_tables_rejects_delta_overflow() {
        let mut path = BezPath::new();
        path.move_to((-20_000.0, 0.0));
        path.line_to((20_000.0, 0.0));
        path.line_to((0.0, 10.0));
        path.close_path();
        let glyphs = glyphs_from_paths(&[path], 1.0).unwrap();
        assert!(matches!(
            build_outline_tables(&glyphs),
            Err(ServerError::InvalidFont(_))
        ));
    }

    #[test]
    fn glyphs_from_paths_boundary_coordinates() {
        let mut ok_path = BezPath::new();
        ok_path.move_to((32_767.25, 0.0));
        ok_path.line_to((0.0, 0.0));
        ok_path.line_to((0.0, 10.0));
        ok_path.close_path();
        assert!(glyphs_from_paths(&[ok_path], 1.0).is_ok());

        let mut bad_path = BezPath::new();
        bad_path.move_to((32_767.5, 0.0));
        bad_path.line_to((0.0, 0.0));
        bad_path.line_to((0.0, 10.0));
        bad_path.close_path();
        assert!(matches!(
            glyphs_from_paths(&[bad_path], 1.0),
            Err(ServerError::InvalidFont(_))
        ));
    }

    #[test]
    fn build_outline_tables_accepts_minimum_delta() {
        let mut path = BezPath::new();
        path.move_to((32_000.0, 0.0));
        path.line_to((-768.0, 0.0));
        path.line_to((-768.0, 10.0));
        path.close_path();
        let glyphs = glyphs_from_paths(&[path], 1.0).unwrap();
        assert!(build_outline_tables(&glyphs).is_ok());
    }

    #[test]
    fn build_outline_tables_rejects_contour_overflow_at_writer_boundary() {
        let mut path = BezPath::new();
        for i in 0..32_767u32 {
            let x = f64::from(i % 180);
            let y = f64::from(i / 180);
            path.move_to((x, y));
            path.line_to((x + 1.0, y));
            path.close_path();
        }
        let glyphs = glyphs_from_paths(&[path], 1.0).unwrap();
        assert!(matches!(
            build_outline_tables(&glyphs),
            Err(ServerError::InvalidFont(_))
        ));
    }

    fn draw_from_bytes(data: &[u8], ch: char) -> BezPath {
        let font = FontRef::new(data).unwrap();
        let gid = font.charmap().map(ch).unwrap();
        let glyph = font.outline_glyphs().get(gid).unwrap();
        let mut pen = BezPathPen(BezPath::new());
        glyph
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut pen,
            )
            .unwrap();
        pen.0
    }

    #[test]
    fn convert_cff_fixture_produces_valid_ttf() {
        let font = FontRef::new(CFF_FIXTURE).unwrap();
        let out = convert_to_glyf(&font).unwrap();

        let converted = FontRef::new(&out).unwrap();
        assert!(converted.glyf().is_ok());
        assert!(converted.loca(None).is_ok());
        assert!(converted.table_data(skrifa::Tag::new(b"CFF ")).is_none());
        assert!(converted.table_data(skrifa::Tag::new(b"VORG")).is_none());
        assert_eq!(
            converted.maxp().unwrap().num_glyphs(),
            font.maxp().unwrap().num_glyphs()
        );
        assert_eq!(
            converted.charmap().map('A').unwrap(),
            font.charmap().map('A').unwrap()
        );
        assert!(converted.table_data(skrifa::Tag::new(b"GSUB")).is_some());
        assert!(converted.table_data(skrifa::Tag::new(b"GPOS")).is_some());
        assert_eq!(
            converted.table_data(skrifa::Tag::new(b"STAT")).is_some(),
            font.table_data(skrifa::Tag::new(b"STAT")).is_some(),
            "정적 CFF의 STAT은 보존되어야 한다"
        );
    }

    #[test]
    fn convert_preserves_advances_and_lsb_matches_xmin() {
        let font = FontRef::new(CFF_FIXTURE).unwrap();
        let out = convert_to_glyf(&font).unwrap();
        let converted = FontRef::new(&out).unwrap();

        let num_glyphs = font.maxp().unwrap().num_glyphs();
        let orig_hmtx = font.hmtx().unwrap();
        let conv_hmtx = converted.hmtx().unwrap();
        for gid in 0..num_glyphs {
            let id = GlyphId::new(u32::from(gid));
            assert_eq!(orig_hmtx.advance(id), conv_hmtx.advance(id), "gid {gid}");
        }

        let loca = converted.loca(None).unwrap();
        let glyf = converted.glyf().unwrap();
        let mut checked = 0;
        for gid in 0..num_glyphs {
            let id = GlyphId::new(u32::from(gid));
            if let Ok(Some(glyph)) = loca.get_glyf(id, &glyf) {
                assert_eq!(
                    conv_hmtx.side_bearing(id).unwrap(),
                    glyph.x_min(),
                    "gid {gid}"
                );
                checked += 1;
            }
        }
        assert!(checked > 100);
    }

    #[test]
    fn convert_outline_equivalence_within_tolerance() {
        let font = FontRef::new(CFF_FIXTURE).unwrap();
        let accuracy = f64::from(font.head().unwrap().units_per_em()) / 1000.0;
        let out = convert_to_glyf(&font).unwrap();

        for ch in ['A', 'g', 'Q', '&', '8', '@', 's', 'R', 'e', 'o'] {
            let original = draw_from_bytes(CFF_FIXTURE, ch);
            let converted = draw_from_bytes(&out, ch);
            let dev = max_deviation(&original, &converted);
            assert!(dev <= 2.0 * accuracy + 1.5, "char {ch}: deviation {dev}");
        }
    }

    #[test]
    fn convert_is_deterministic() {
        let font = FontRef::new(CFF_FIXTURE).unwrap();
        assert_eq!(
            convert_to_glyf(&font).unwrap(),
            convert_to_glyf(&font).unwrap()
        );
    }

    #[test]
    fn convert_rejects_cff2() {
        let font = FontRef::new(CFF2_FIXTURE).unwrap();
        let result = convert_to_glyf(&font);
        assert!(
            matches!(&result, Err(ServerError::InvalidFont(msg)) if msg.contains("unsplittable font")),
            "{result:?}"
        );
    }

    #[test]
    fn convert_rejects_varc_and_variable_cff() {
        let font = FontRef::new(CFF_FIXTURE).unwrap();
        for tag in [b"VARC", b"fvar"] {
            let mut builder = FontBuilder::new();
            builder.add_raw(Tag::new(tag), vec![0u8; 4]);
            builder.copy_missing_tables(font.clone());
            let patched = builder.build();
            let result = convert_to_glyf(&FontRef::new(&patched).unwrap());
            assert!(
                matches!(&result, Err(ServerError::InvalidFont(msg)) if msg.contains("unsplittable font")),
                "{result:?}"
            );
        }
    }

    #[test]
    #[ignore = "SOURCE_HAN_PATH 환경 변수로 CID-keyed 한국어 OTF 경로 지정 필요"]
    fn convert_cid_keyed_korean_font() {
        let path = std::env::var("SOURCE_HAN_PATH").expect("SOURCE_HAN_PATH not set");
        let data = std::fs::read(path).expect("font file must be readable");
        let font = FontRef::new(&data).unwrap();
        let accuracy = f64::from(font.head().unwrap().units_per_em()) / 1000.0;
        let out = convert_to_glyf(&font).unwrap();
        let converted = FontRef::new(&out).unwrap();

        let num_glyphs = font.maxp().unwrap().num_glyphs();
        let orig_hmtx = font.hmtx().unwrap();
        let conv_hmtx = converted.hmtx().unwrap();
        for gid in 0..num_glyphs {
            let id = GlyphId::new(u32::from(gid));
            assert_eq!(orig_hmtx.advance(id), conv_hmtx.advance(id), "gid {gid}");
        }

        let orig_vmtx = font.vmtx().expect("CID 한국어 폰트에는 vmtx가 있어야 한다");
        let vorg = font.vorg().expect("CID 한국어 폰트에는 VORG가 있어야 한다");
        assert!(converted.table_data(skrifa::Tag::new(b"VORG")).is_none());
        let conv_vmtx = converted.vmtx().unwrap();
        let loca = converted.loca(None).unwrap();
        let glyf = converted.glyf().unwrap();
        assert!(
            out.len() < 64 * 1024 * 1024,
            "converted font must fit the client base cap: {} bytes",
            out.len()
        );
        let mut checked_origins = 0u32;
        let mut expect_min_tsb = i16::MAX;
        let mut expect_min_bsb = i32::MAX;
        let mut expect_extent = i32::from(i16::MIN);
        for gid in 0..num_glyphs {
            let id = GlyphId::new(u32::from(gid));
            assert_eq!(
                orig_vmtx.advance(id),
                conv_vmtx.advance(id),
                "vmtx gid {gid}"
            );

            let Ok(Some(glyph)) = loca.get_glyf(id, &glyf) else {
                continue;
            };
            let metrics = vorg.vert_origin_y_metrics();
            let origin = metrics
                .binary_search_by(|m| m.glyph_index().to_u32().cmp(&u32::from(gid)))
                .ok()
                .map(|i| metrics[i].vert_origin_y())
                .unwrap_or_else(|| vorg.default_vert_origin_y());
            let tsb = conv_vmtx.side_bearing(id).unwrap();
            assert_eq!(
                i32::from(tsb) + i32::from(glyph.y_max()),
                i32::from(origin),
                "vertical origin gid {gid}"
            );
            let height = i32::from(glyph.y_max()) - i32::from(glyph.y_min());
            expect_min_tsb = expect_min_tsb.min(tsb);
            expect_min_bsb = expect_min_bsb
                .min(i32::from(conv_vmtx.advance(id).unwrap()) - i32::from(tsb) - height);
            expect_extent = expect_extent.max(i32::from(tsb) + height);
            checked_origins += 1;
        }
        assert!(checked_origins > 10_000, "checked {checked_origins}");

        let conv_vhea = converted.vhea().unwrap();
        assert_eq!(conv_vhea.min_top_side_bearing().to_i16(), expect_min_tsb);
        assert_eq!(
            i32::from(conv_vhea.min_bottom_side_bearing().to_i16()),
            expect_min_bsb
        );
        assert_eq!(i32::from(conv_vhea.y_max_extent().to_i16()), expect_extent);

        for ch in ['가', '힣', '한', '글', '뷁', '개'] {
            let original = draw_from_bytes(&data, ch);
            let converted_path = draw_from_bytes(&out, ch);
            let dev = max_deviation(&original, &converted_path);
            assert!(dev <= 2.0 * accuracy + 1.5, "char {ch}: deviation {dev}");
        }
    }
}
