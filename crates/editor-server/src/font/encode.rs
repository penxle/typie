use editor_macros::ffi;
use editor_resource::compress_zstd;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use skrifa::raw::tables::glyf::{Glyf, Glyph};
use skrifa::raw::tables::gsub::{
    AlternateSubstFormat1, LigatureSubstFormat1, SingleSubst, SubstitutionSubtables,
};
use skrifa::raw::tables::loca::Loca;
use skrifa::raw::{FontRef, TableProvider};
use skrifa::{GlyphId, MetadataProvider, Tag};
use write_fonts::FontBuilder;

use crate::ServerError;

#[ffi]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltFont {
    pub hash: String,
    /// chunk별 flat 페어 `[start0, end0, start1, end1, ...]` (inclusive).
    pub coverage: Vec<Vec<u32>>,
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
    pub base: Vec<u8>,
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array[]"))]
    pub chunks: Vec<serde_bytes::ByteBuf>,
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
    pub manifest: Vec<u8>,
}

pub fn get_font_codepoints(ttf_data: &[u8]) -> Result<Vec<u32>, ServerError> {
    let font = FontRef::new(ttf_data).map_err(|e| ServerError::InvalidFont(e.to_string()))?;
    let mut codepoints: Vec<u32> = font
        .charmap()
        .mappings()
        .map(|(cp, _)| cp)
        .filter(|cp| !(0xD800..=0xDFFF).contains(cp))
        .collect();
    codepoints.sort();
    codepoints.dedup();
    Ok(codepoints)
}

pub fn build_font(
    ttf_data: &[u8],
    chunk_codepoints: &[Vec<u32>],
) -> Result<BuiltFont, ServerError> {
    let font = FontRef::new(ttf_data).map_err(|e| ServerError::InvalidFont(e.to_string()))?;

    let has_glyf = font.glyf().is_ok();
    let has_cbdt = font.table_data(Tag::new(b"CBDT")).is_some()
        && font.table_data(Tag::new(b"CBLC")).is_some();

    if !has_glyf && !has_cbdt {
        return Err(ServerError::InvalidFont(
            "unsplittable font: no glyf/CBDT table (CFF/CFF2 unsupported)".into(),
        ));
    }

    let charmap = font.charmap();
    let mut cp_to_gid: HashMap<u32, u16> = HashMap::new();
    for (cp, gid) in charmap.mappings() {
        cp_to_gid.insert(cp, gid.to_u32() as u16);
    }

    let num_glyphs = font
        .maxp()
        .map_err(|e| ServerError::InvalidFont(e.to_string()))?
        .num_glyphs();
    let gsub_alternates = resolve_gsub_alternates(&font);

    let mut per_glyph: HashMap<u16, (usize, Vec<u8>)> = HashMap::new();
    let mut composite_deps: HashMap<u16, HashSet<u16>> = HashMap::new();
    let mut table_overrides: Vec<(Tag, Vec<u8>)> = Vec::new();
    let mut split_tag: Option<Tag> = None;

    if has_cbdt {
        let cbdt_raw = font
            .table_data(Tag::new(b"CBDT"))
            .ok_or_else(|| ServerError::InvalidFont("CBDT missing".into()))?;
        let cblc = font
            .cblc()
            .map_err(|e| ServerError::InvalidFont(e.to_string()))?;

        for size in cblc.bitmap_sizes() {
            let start_gid = size.start_glyph_index().to_u32();
            let end_gid = size.end_glyph_index().to_u32();
            for gid in start_gid..=end_gid {
                if let Ok(loc) = size.location(cblc.offset_data(), GlyphId::new(gid))
                    && loc.data_size > 0
                {
                    let start = loc.data_offset;
                    let end = start + loc.data_size;
                    per_glyph.insert(gid as u16, (start, cbdt_raw.as_ref()[start..end].to_vec()));
                }
            }
        }

        table_overrides.push((Tag::new(b"CBDT"), vec![0u8; cbdt_raw.len()]));
        let cblc_raw = font
            .table_data(Tag::new(b"CBLC"))
            .ok_or_else(|| ServerError::InvalidFont("CBLC missing".into()))?;
        table_overrides.push((Tag::new(b"CBLC"), cblc_raw.as_ref().to_vec()));
        split_tag = Some(Tag::new(b"CBDT"));
    } else if has_glyf {
        let glyf_raw = font
            .table_data(Tag::new(b"glyf"))
            .ok_or_else(|| ServerError::InvalidFont("glyf missing".into()))?;
        let loca_raw = font
            .table_data(Tag::new(b"loca"))
            .ok_or_else(|| ServerError::InvalidFont("loca missing".into()))?;
        let loca = font
            .loca(None)
            .map_err(|e| ServerError::InvalidFont(e.to_string()))?;
        let glyf = font
            .glyf()
            .map_err(|e| ServerError::InvalidFont(e.to_string()))?;

        let glyf_bytes = glyf_raw.as_ref();
        for gid in 0..num_glyphs {
            let start = loca.get_raw(gid as usize).unwrap_or(0) as usize;
            let end = loca.get_raw(gid as usize + 1).unwrap_or(0) as usize;
            if start < end {
                per_glyph.insert(gid, (start, glyf_bytes[start..end].to_vec()));
                let deps = resolve_composite_deps(&loca, &glyf, gid, num_glyphs);
                if !deps.is_empty() {
                    composite_deps.insert(gid, deps);
                }
            }
        }

        table_overrides.push((Tag::new(b"glyf"), vec![0u8; glyf_raw.len()]));
        table_overrides.push((Tag::new(b"loca"), loca_raw.as_ref().to_vec()));
        split_tag = Some(Tag::new(b"glyf"));
    }

    let mut chunks: Vec<Vec<u8>> = Vec::new();

    if split_tag.is_some() {
        for cps in chunk_codepoints {
            let mut gids_needed: HashSet<u16> = HashSet::new();
            for &cp in cps {
                if let Some(&gid) = cp_to_gid.get(&cp) {
                    gids_needed.insert(gid);
                    if let Some(alts) = gsub_alternates.get(&gid) {
                        gids_needed.extend(alts);
                    }
                }
            }

            let mut expanded: HashSet<u16> = HashSet::new();
            for &gid in &gids_needed {
                if let Some(deps) = composite_deps.get(&gid) {
                    expanded.extend(deps);
                }
            }
            gids_needed.extend(expanded);

            let mut entries: Vec<(usize, &[u8])> = Vec::new();
            let mut sorted_gids: Vec<u16> = gids_needed.iter().copied().collect();
            sorted_gids.sort();
            for gid in sorted_gids {
                if let Some((offset, data)) = per_glyph.get(&gid) {
                    entries.push((*offset, data.as_slice()));
                }
            }

            chunks.push(build_chunk_binary(&entries));
        }
    }

    let glyph_bounds = if has_glyf {
        let mut x_min = i16::MAX;
        let mut y_min = i16::MAX;
        let mut x_max = i16::MIN;
        let mut y_max = i16::MIN;
        let mut has_any = false;
        for (_, data) in per_glyph.values() {
            if data.len() >= 10 {
                x_min = x_min.min(i16::from_be_bytes([data[2], data[3]]));
                y_min = y_min.min(i16::from_be_bytes([data[4], data[5]]));
                x_max = x_max.max(i16::from_be_bytes([data[6], data[7]]));
                y_max = y_max.max(i16::from_be_bytes([data[8], data[9]]));
                has_any = true;
            }
        }
        has_any.then_some((x_min, y_min, x_max, y_max))
    } else {
        None
    };

    let head_tag = Tag::new(b"head");
    let needs_head_patch = if let Ok(head) = font.head() {
        head.x_min() == 0 && head.y_min() == 0 && head.x_max() == 0 && head.y_max() == 0
    } else {
        false
    };

    if needs_head_patch
        && let (Some((gx_min, gy_min, gx_max, gy_max)), Some(head_data)) =
            (glyph_bounds, font.table_data(head_tag))
    {
        let mut patched_head = head_data.as_ref().to_vec();
        patched_head[36..38].copy_from_slice(&gx_min.to_be_bytes());
        patched_head[38..40].copy_from_slice(&gy_min.to_be_bytes());
        patched_head[40..42].copy_from_slice(&gx_max.to_be_bytes());
        patched_head[42..44].copy_from_slice(&gy_max.to_be_bytes());
        table_overrides.push((head_tag, patched_head));
    }

    let mut builder = FontBuilder::new();
    for (tag, data) in &table_overrides {
        builder.add_raw(*tag, data.as_slice());
    }
    builder.copy_missing_tables(font);
    let base_data = builder.build();

    let hash = compute_hash(&base_data, &chunks);

    let coverage: Vec<Vec<u32>> = chunk_codepoints
        .iter()
        .map(|cps| codepoints_to_ranges(cps))
        .collect();

    let manifest = build_font_manifest(&coverage)?;

    let base = compress_zstd(&base_data);
    let chunks: Vec<serde_bytes::ByteBuf> = chunks
        .iter()
        .map(|c| serde_bytes::ByteBuf::from(compress_zstd(c)))
        .collect();

    Ok(BuiltFont {
        hash,
        coverage,
        base,
        chunks,
        manifest,
    })
}

const MANIFEST_MAX_BYTES: usize = 1024 * 1024;

pub fn build_font_manifest(coverages: &[Vec<u32>]) -> Result<Vec<u8>, ServerError> {
    if coverages.len() > 255 {
        return Err(ServerError::InvalidFont(format!(
            "too many chunks: {}",
            coverages.len()
        )));
    }
    const MAX_TOTAL_CODEPOINTS: u64 = 300_000;
    let mut total: u64 = 0;
    for ranges in coverages {
        if ranges.len() % 2 != 0 {
            return Err(ServerError::InvalidFont("coverage odd tail".into()));
        }
        for pair in ranges.chunks_exact(2) {
            if pair[0] > pair[1] || pair[1] > 0x10FFFF {
                return Err(ServerError::InvalidFont(format!(
                    "invalid coverage range: {}..={}",
                    pair[0], pair[1]
                )));
            }
            if pair[0] <= 0xDFFF && pair[1] >= 0xD800 {
                return Err(ServerError::InvalidFont(format!(
                    "coverage overlaps surrogate range: {}..={}",
                    pair[0], pair[1]
                )));
            }
            total = total
                .checked_add(u64::from(pair[1] - pair[0]) + 1)
                .ok_or_else(|| ServerError::InvalidFont("coverage size overflow".into()))?;
            if total > MAX_TOTAL_CODEPOINTS {
                return Err(ServerError::InvalidFont(format!(
                    "coverage too large: {total} codepoints"
                )));
            }
        }
    }
    let bytes = editor_resource::FontManifest::from_coverages(coverages).to_bytes();
    if bytes.len() > MANIFEST_MAX_BYTES {
        return Err(ServerError::InvalidFont(format!(
            "manifest too large: {}",
            bytes.len()
        )));
    }
    editor_resource::FontManifest::from_bytes(&bytes)
        .map_err(|e| ServerError::InvalidFont(format!("manifest self-validation failed: {e:?}")))?;
    Ok(compress_zstd(&bytes))
}

fn build_chunk_binary(entries: &[(usize, &[u8])]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&(entries.len() as u32).to_be_bytes());
    for &(offset, data) in entries {
        buf.extend_from_slice(&(offset as u32).to_be_bytes());
        buf.extend_from_slice(&(data.len() as u32).to_be_bytes());
        buf.extend_from_slice(data);
    }
    buf
}

/// `[start0, end0, start1, end1, ...]` flat pair representation (inclusive).
fn codepoints_to_ranges(cps: &[u32]) -> Vec<u32> {
    let mut sorted = cps.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut ranges = Vec::new();
    let mut iter = sorted.into_iter().peekable();
    while let Some(start) = iter.next() {
        let mut end = start;
        while let Some(&next) = iter.peek() {
            if next == end + 1 {
                end = next;
                iter.next();
            } else {
                break;
            }
        }
        ranges.push(start);
        ranges.push(end);
    }
    ranges
}

fn compute_hash(base_data: &[u8], chunks: &[Vec<u8>]) -> String {
    use std::hash::Hasher;
    let mut hasher = rapidhash::quality::RapidHasher::default();
    hasher.write(&(base_data.len() as u64).to_be_bytes());
    hasher.write(base_data);
    hasher.write(&(chunks.len() as u32).to_be_bytes());
    for chunk in chunks {
        hasher.write(&(chunk.len() as u64).to_be_bytes());
        hasher.write(chunk);
    }
    hex::encode(hasher.finish().to_be_bytes())
}

fn resolve_composite_deps(loca: &Loca, glyf: &Glyf, gid: u16, num_glyphs: u16) -> HashSet<u16> {
    let mut result = HashSet::new();
    let mut stack = vec![gid];
    let mut visited = HashSet::new();

    while let Some(current) = stack.pop() {
        if !visited.insert(current) {
            continue;
        }
        if let Ok(Some(Glyph::Composite(composite))) =
            loca.get_glyf(GlyphId::new(current as u32), glyf)
        {
            for (comp_gid, _) in composite.component_glyphs_and_flags() {
                let comp = comp_gid.to_u32() as u16;
                if comp < num_glyphs && comp != current {
                    result.insert(comp);
                    stack.push(comp);
                }
            }
        }
    }

    result
}

fn resolve_gsub_alternates(font: &FontRef) -> HashMap<u16, HashSet<u16>> {
    let mut alternates: HashMap<u16, HashSet<u16>> = HashMap::new();
    let Ok(gsub) = font.gsub() else {
        return alternates;
    };

    let Ok(lookup_list) = gsub.lookup_list() else {
        return alternates;
    };

    let Ok(feature_list) = gsub.feature_list() else {
        return alternates;
    };

    for feature_record in feature_list.feature_records() {
        let Ok(feature) = feature_record.feature(feature_list.offset_data()) else {
            continue;
        };
        for lookup_idx in feature.lookup_list_indices() {
            let Ok(lookup) = lookup_list.lookups().get(lookup_idx.get() as usize) else {
                continue;
            };
            let Ok(subtables) = lookup.subtables() else {
                continue;
            };
            match subtables {
                SubstitutionSubtables::Single(tables) => {
                    for table in tables.iter() {
                        let Ok(table) = table else { continue };
                        match table {
                            SingleSubst::Format1(t) => {
                                let Ok(coverage) = t.coverage() else { continue };
                                let delta = t.delta_glyph_id();
                                for gid in coverage.iter() {
                                    let src = gid.to_u32() as u16;
                                    let dst = (src as i32 + delta as i32) as u16;
                                    alternates.entry(src).or_default().insert(dst);
                                }
                            }
                            SingleSubst::Format2(t) => {
                                let Ok(coverage) = t.coverage() else { continue };
                                let substitutes = t.substitute_glyph_ids();
                                for (i, gid) in coverage.iter().enumerate() {
                                    let src = gid.to_u32() as u16;
                                    if let Some(dst) = substitutes.get(i) {
                                        let dst = dst.get().to_u32() as u16;
                                        alternates.entry(src).or_default().insert(dst);
                                    }
                                }
                            }
                        }
                    }
                }
                SubstitutionSubtables::Alternate(tables) => {
                    for table in tables.iter() {
                        let Ok(table) = table else { continue };
                        let table: AlternateSubstFormat1 = table;
                        let Ok(coverage) = table.coverage() else {
                            continue;
                        };
                        let alt_sets = table.alternate_sets();
                        for (i, gid) in coverage.iter().enumerate() {
                            let src = gid.to_u32() as u16;
                            if let Ok(alt_set) = alt_sets.get(i) {
                                for alt_gid in alt_set.alternate_glyph_ids() {
                                    alternates
                                        .entry(src)
                                        .or_default()
                                        .insert(alt_gid.get().to_u32() as u16);
                                }
                            }
                        }
                    }
                }
                SubstitutionSubtables::Ligature(tables) => {
                    for table in tables.iter() {
                        let Ok(table) = table else { continue };
                        let table: LigatureSubstFormat1 = table;
                        let Ok(coverage) = table.coverage() else {
                            continue;
                        };
                        let lig_sets = table.ligature_sets();
                        for (i, gid) in coverage.iter().enumerate() {
                            let src = gid.to_u32() as u16;
                            if let Ok(lig_set) = lig_sets.get(i) {
                                for lig in lig_set.ligatures().iter().flatten() {
                                    alternates
                                        .entry(src)
                                        .or_default()
                                        .insert(lig.ligature_glyph().to_u32() as u16);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    alternates
}

#[cfg(test)]
mod tests {
    use editor_resource::decompress_zstd;

    use super::*;

    fn load_test_font() -> Option<Vec<u8>> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../editor-view/assets/test-font.ttf"
        ))
        .ok()
    }

    #[test]
    fn codepoints_invalid_data() {
        let result = get_font_codepoints(&[0, 1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn codepoints_nonempty() {
        let Some(data) = load_test_font() else {
            return;
        };
        let cps = get_font_codepoints(&data).unwrap();
        assert!(!cps.is_empty());
    }

    #[test]
    fn codepoints_sorted_and_deduped() {
        let Some(data) = load_test_font() else {
            return;
        };
        let cps = get_font_codepoints(&data).unwrap();
        for w in cps.windows(2) {
            assert!(w[0] < w[1], "not sorted/deduped: {} >= {}", w[0], w[1]);
        }
    }

    #[test]
    fn codepoints_exclude_surrogates() {
        let Some(data) = load_test_font() else {
            return;
        };
        let font = FontRef::new(&data).unwrap();

        let mut cmap = Vec::new();
        for v in [0u16, 1, 3, 1] {
            cmap.extend_from_slice(&v.to_be_bytes());
        }
        cmap.extend_from_slice(&12u32.to_be_bytes());
        for v in [
            4u16, 32, 0, 4, 4, 1, 0, 0xD801, 0xFFFF, 0, 0xD7A4, 0xFFFF, 10333, 1, 0, 0,
        ] {
            cmap.extend_from_slice(&v.to_be_bytes());
        }

        let mut builder = FontBuilder::new();
        builder.add_raw(Tag::new(b"cmap"), cmap.as_slice());
        builder.copy_missing_tables(font);
        let patched = builder.build();

        let cps = get_font_codepoints(&patched).unwrap();
        assert!(cps.iter().any(|cp| (0xD7A4..0xD800).contains(cp)));
        assert!(!cps.iter().any(|cp| (0xD800..=0xDFFF).contains(cp)));
    }

    #[test]
    fn encode_invalid_data() {
        let result = build_font(&[0, 1, 2, 3], &[vec![0x41]]);
        assert!(result.is_err());
    }

    #[test]
    fn encode_produces_base_and_chunks() {
        let Some(data) = load_test_font() else {
            return;
        };
        let cps = get_font_codepoints(&data).unwrap();
        let chunk_cps: Vec<Vec<u32>> = cps.chunks(200).map(|c| c.to_vec()).collect();

        let encoded = build_font(&data, &chunk_cps).unwrap();
        assert!(!encoded.hash.is_empty());
        assert_eq!(encoded.hash.len(), 16);
        assert!(!encoded.base.is_empty());
        assert_eq!(encoded.chunks.len(), chunk_cps.len());
        assert_eq!(encoded.coverage.len(), chunk_cps.len());
        for cov in &encoded.coverage {
            assert!(cov.len() % 2 == 0, "coverage must be flat pairs");
        }
        assert!(!encoded.manifest.is_empty());
    }

    #[test]
    fn encode_base_has_no_split_prefix_and_is_zstd() {
        let Some(data) = load_test_font() else {
            return;
        };
        let encoded = build_font(&data, &[]).unwrap();
        let base_raw = decompress_zstd(&encoded.base).unwrap();
        let _ = FontRef::new(&base_raw).expect("decompressed base is a valid TTF");
    }

    #[test]
    fn encode_empty_chunks() {
        let Some(data) = load_test_font() else {
            return;
        };
        let encoded = build_font(&data, &[]).unwrap();
        assert!(!encoded.base.is_empty());
        assert!(encoded.chunks.is_empty());
        assert!(encoded.coverage.is_empty());
    }

    #[test]
    fn encode_hash_stable_for_same_input() {
        let Some(data) = load_test_font() else {
            return;
        };
        let cp_groups = vec![vec![0x41u32]];
        let a = build_font(&data, &cp_groups).unwrap();
        let b = build_font(&data, &cp_groups).unwrap();
        assert_eq!(a.hash, b.hash);
    }

    #[test]
    fn build_font_manifest_matches_from_coverages() {
        let coverages = vec![vec![0x41, 0x43], vec![0xAC00, 0xAC02]];
        let bytes = build_font_manifest(&coverages).unwrap();
        let decompressed = editor_resource::decompress_zstd(&bytes).unwrap();
        let manifest = editor_resource::FontManifest::from_bytes(&decompressed).unwrap();
        assert_eq!(manifest.chunk_id(0x42), Some(0));
        assert_eq!(manifest.chunk_id(0xAC01), Some(1));
        assert_eq!(manifest.chunk_id(0x9999), None);
    }

    #[test]
    fn build_font_manifest_rejects_over_255_chunks() {
        let coverages: Vec<Vec<u32>> = (0..256u32).map(|i| vec![i, i]).collect();
        assert!(build_font_manifest(&coverages).is_err());
    }

    #[test]
    fn build_font_manifest_rejects_over_1mb_serialized_size() {
        let coverages = vec![vec![0x10000, 0x10000 + 140_000 - 1]];
        let result = build_font_manifest(&coverages);
        assert!(
            matches!(&result, Err(ServerError::InvalidFont(msg)) if msg.starts_with("manifest too large")),
            "{result:?}"
        );
    }

    #[test]
    fn build_font_manifest_rejects_invalid_coverage_input() {
        assert!(build_font_manifest(&[vec![0x41]]).is_err(), "odd tail");
        assert!(
            build_font_manifest(&[vec![0x120000, 0x120000]]).is_err(),
            "out of unicode range"
        );
        assert!(
            build_font_manifest(&[vec![0xD000, 0xE000]]).is_err(),
            "overlaps surrogates"
        );
        assert!(
            build_font_manifest(&[vec![0x0, 0xD7FF], vec![0xE000, 0x10FFFF]]).is_err(),
            "total codepoints over cap — 팽창 전 사전 거부(임시 벡터 미생성)"
        );
    }

    #[test]
    fn build_font_rejects_cff_font_without_glyf_or_cbdt() {
        let Some(data) = load_test_font() else {
            return;
        };
        let mut font = data.clone();
        let num_tables = u16::from_be_bytes([font[4], font[5]]);
        for i in 0..num_tables {
            let off = 12 + (i as usize) * 16;
            if &font[off..off + 4] == b"glyf" {
                font[off..off + 4].copy_from_slice(b"CFF ");
            }
        }
        let result = build_font(&font, &[vec![0x41]]);
        assert!(result.is_err());
    }
}
