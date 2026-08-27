use editor_macros::ffi;
use editor_resource::compress_zstd;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use skrifa::raw::collections::IntSet;
use skrifa::raw::tables::glyf::{Glyf, Glyph};
use skrifa::raw::tables::loca::Loca;
use skrifa::raw::{FontRef, TableProvider};
use skrifa::{GlyphId, MetadataProvider, Tag};
use write_fonts::FontBuilder;

use super::convert::convert_to_glyf;
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
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
    pub manifest_v2: Vec<u8>,
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

    if !has_glyf && !has_cbdt {
        if font.table_data(Tag::new(b"CFF ")).is_none() {
            return Err(ServerError::InvalidFont(
                "unsplittable font: no glyf/CBDT/CFF table".into(),
            ));
        }
        let converted = convert_to_glyf(&font)?;
        return build_font(&converted, chunk_codepoints);
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
    let gsub_closure = if font.table_data(Tag::new(b"GSUB")).is_some() {
        let gsub = font
            .gsub()
            .map_err(|e| ServerError::InvalidFont(e.to_string()))?;
        let lookups = gsub
            .collect_lookups(&IntSet::all())
            .map_err(|e| ServerError::InvalidFont(e.to_string()))?;
        Some((gsub, lookups))
    } else {
        None
    };

    let mut per_glyph: HashMap<u16, (usize, Vec<u8>)> = HashMap::new();
    let mut composite_deps: HashMap<u16, HashSet<u16>> = HashMap::new();
    let mut colr_layers: HashMap<u16, Vec<u16>> = HashMap::new();
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

        if let Ok(colr) = font.colr()
            && let (Some(Ok(bases)), Some(Ok(layers))) =
                (colr.base_glyph_records(), colr.layer_records())
        {
            for base in bases {
                let start = usize::from(base.first_layer_index());
                let end = start + usize::from(base.num_layers());
                if end > layers.len() {
                    continue;
                }
                let gids: Vec<u16> = layers[start..end]
                    .iter()
                    .map(|l| l.glyph_id().to_u16())
                    .filter(|&g| g < num_glyphs)
                    .collect();
                if !gids.is_empty() {
                    colr_layers.insert(base.glyph_id().to_u16(), gids);
                }
            }
        }

        table_overrides.push((Tag::new(b"glyf"), vec![0u8; glyf_raw.len()]));
        table_overrides.push((Tag::new(b"loca"), loca_raw.as_ref().to_vec()));
        split_tag = Some(Tag::new(b"glyf"));
    }

    let mut chunks: Vec<Vec<u8>> = Vec::new();
    let mut chunk_render_gids: Vec<HashSet<u16>> = Vec::new();

    if split_tag.is_some() {
        let mut chunk_payload_gids: Vec<HashSet<u16>> = Vec::new();
        for cps in chunk_codepoints {
            let mut reachable: HashSet<u16> = HashSet::new();
            for &cp in cps {
                if let Some(&gid) = cp_to_gid.get(&cp) {
                    reachable.insert(gid);
                }
            }

            if let Some((gsub, lookups)) = &gsub_closure {
                let mut glyphs: IntSet<GlyphId> = reachable
                    .iter()
                    .map(|&gid| GlyphId::new(u32::from(gid)))
                    .collect();
                gsub.closure_glyphs(lookups, &mut glyphs)
                    .map_err(|e| ServerError::InvalidFont(e.to_string()))?;
                reachable.extend(glyphs.iter().map(|gid| gid.to_u32() as u16));
            }

            let render_gids: HashSet<u16> = reachable
                .into_iter()
                .filter(|&gid| {
                    gid != 0 && (per_glyph.contains_key(&gid) || colr_layers.contains_key(&gid))
                })
                .collect();
            let payload_gids =
                expand_render_dependencies(&render_gids, &colr_layers, &composite_deps);
            chunk_render_gids.push(render_gids);
            chunk_payload_gids.push(payload_gids);
        }

        let mut globally_reachable: HashSet<u16> = chunk_codepoints
            .iter()
            .flatten()
            .filter_map(|cp| cp_to_gid.get(cp).copied())
            .collect();
        if let Some((gsub, lookups)) = &gsub_closure {
            let mut glyphs: IntSet<GlyphId> = globally_reachable
                .iter()
                .map(|&gid| GlyphId::new(u32::from(gid)))
                .collect();
            gsub.closure_glyphs(lookups, &mut glyphs)
                .map_err(|e| ServerError::InvalidFont(e.to_string()))?;
            globally_reachable.extend(glyphs.iter().map(|gid| gid.to_u32() as u16));
        }
        let owned: HashSet<u16> = chunk_render_gids.iter().flatten().copied().collect();
        let mut unowned: Vec<u16> = globally_reachable
            .into_iter()
            .filter(|gid| {
                *gid != 0
                    && !owned.contains(gid)
                    && (per_glyph.contains_key(gid) || colr_layers.contains_key(gid))
            })
            .collect();
        unowned.sort_unstable();

        if !unowned.is_empty() && chunk_payload_gids.is_empty() {
            return Err(ServerError::InvalidFont(
                "split font has renderable glyphs but no chunks".into(),
            ));
        }

        let mut chunk_payload_sizes: Vec<usize> = chunk_payload_gids
            .iter()
            .map(|gids| {
                gids.iter()
                    .filter_map(|gid| per_glyph.get(gid))
                    .map(|(_, data)| data.len())
                    .sum()
            })
            .collect();
        for gid in unowned {
            let chunk_id = chunk_payload_gids
                .iter()
                .enumerate()
                .min_by_key(|(chunk_id, _)| (chunk_payload_sizes[*chunk_id], *chunk_id))
                .map(|(chunk_id, _)| chunk_id)
                .expect("non-empty chunks were checked above");
            chunk_render_gids[chunk_id].insert(gid);
            let dependencies =
                expand_render_dependencies(&HashSet::from([gid]), &colr_layers, &composite_deps);
            for dependency in dependencies {
                if chunk_payload_gids[chunk_id].insert(dependency) {
                    chunk_payload_sizes[chunk_id] += per_glyph
                        .get(&dependency)
                        .map(|(_, data)| data.len())
                        .unwrap_or(0);
                }
            }
        }

        for gids_needed in chunk_payload_gids {
            let mut entries: Vec<(usize, &[u8])> = Vec::new();
            let mut sorted_gids: Vec<u16> = gids_needed.iter().copied().collect();
            sorted_gids.sort_unstable();
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

    let coverage: Vec<Vec<u32>> = chunk_codepoints
        .iter()
        .map(|cps| codepoints_to_ranges(cps))
        .collect();

    let manifest = build_font_manifest(&coverage)?;
    let glyph_chunks: Vec<Vec<u16>> = chunk_render_gids
        .into_iter()
        .map(|gids| {
            let mut gids: Vec<u16> = gids.into_iter().collect();
            gids.sort_unstable();
            gids
        })
        .collect();
    let manifest_v2 = build_font_manifest_v2(&coverage, num_glyphs, glyph_chunks)?;
    let hash = compute_hash(&base_data, &chunks, &manifest_v2);

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
        manifest_v2,
    })
}

fn expand_render_dependencies(
    render_gids: &HashSet<u16>,
    colr_layers: &HashMap<u16, Vec<u16>>,
    composite_deps: &HashMap<u16, HashSet<u16>>,
) -> HashSet<u16> {
    let mut payload_gids = render_gids.clone();
    for gid in render_gids {
        if let Some(layers) = colr_layers.get(gid) {
            payload_gids.extend(layers);
        }
    }
    let outlined_gids: Vec<u16> = payload_gids.iter().copied().collect();
    for gid in outlined_gids {
        if let Some(deps) = composite_deps.get(&gid) {
            payload_gids.extend(deps);
        }
    }
    payload_gids
}

const MANIFEST_MAX_BYTES: usize = 1024 * 1024;

pub fn build_font_manifest(coverages: &[Vec<u32>]) -> Result<Vec<u8>, ServerError> {
    validate_coverages(coverages)?;
    encode_manifest(editor_resource::FontManifest::from_coverages(coverages))
}

fn build_font_manifest_v2(
    coverages: &[Vec<u32>],
    num_glyphs: u16,
    glyph_chunks: Vec<Vec<u16>>,
) -> Result<Vec<u8>, ServerError> {
    validate_coverages(coverages)?;
    let manifest = editor_resource::FontManifest::from_coverages(coverages)
        .with_glyph_chunks(num_glyphs, glyph_chunks)
        .map_err(|e| ServerError::InvalidFont(format!("invalid glyph chunks: {e:?}")))?;
    encode_manifest(manifest)
}

fn validate_coverages(coverages: &[Vec<u32>]) -> Result<(), ServerError> {
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
    Ok(())
}

fn encode_manifest(manifest: editor_resource::FontManifest) -> Result<Vec<u8>, ServerError> {
    let bytes = manifest.to_bytes();
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

fn compute_hash(base_data: &[u8], chunks: &[Vec<u8>], manifest_v2: &[u8]) -> String {
    use std::hash::Hasher;
    let mut hasher = rapidhash::quality::RapidHasher::default();
    hasher.write(&(base_data.len() as u64).to_be_bytes());
    hasher.write(base_data);
    hasher.write(&(chunks.len() as u32).to_be_bytes());
    for chunk in chunks {
        hasher.write(&(chunk.len() as u64).to_be_bytes());
        hasher.write(chunk);
    }
    hasher.write(&(manifest_v2.len() as u64).to_be_bytes());
    hasher.write(manifest_v2);
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
    fn gsub_closure_glyphs_are_included_in_chunk() {
        let chunk_codepoints = vec![vec![u32::from('i')]];
        let converted = convert_to_glyf(&FontRef::new(CFF_FIXTURE).unwrap()).unwrap();
        let font = FontRef::new(&converted).unwrap();
        let mut expected: IntSet<GlyphId> = chunk_codepoints[0]
            .iter()
            .filter_map(|&cp| font.charmap().map(cp))
            .collect();
        let nominal_count = expected.len();
        let gsub = font.gsub().unwrap();
        let lookups = gsub.collect_lookups(&IntSet::all()).unwrap();
        gsub.closure_glyphs(&lookups, &mut expected).unwrap();
        assert!(
            expected.len() > nominal_count,
            "GSUB 치환이 실제로 발생해야 한다"
        );

        let loca = font.loca(None).unwrap();
        let expected: HashSet<u16> = expected
            .iter()
            .filter(|gid| {
                let gid = gid.to_u32() as usize;
                loca.get_raw(gid).unwrap_or(0) < loca.get_raw(gid + 1).unwrap_or(0)
            })
            .map(|gid| gid.to_u32() as u16)
            .collect();
        let built = build_font(CFF_FIXTURE, &chunk_codepoints).unwrap();
        let base = decompress_zstd(&built.base).unwrap();
        let actual = chunk_gids(&built.chunks[0], &base);
        let manifest_v2 = editor_resource::FontManifest::from_bytes(
            &decompress_zstd(&built.manifest_v2).unwrap(),
        )
        .unwrap();

        assert!(
            expected.is_subset(&actual),
            "누락된 GSUB closure 글리프: {:?}",
            expected.difference(&actual).collect::<Vec<_>>()
        );
        assert_eq!(
            manifest_v2.num_glyphs(),
            Some(font.maxp().unwrap().num_glyphs())
        );
        assert!(
            expected
                .iter()
                .all(|&gid| manifest_v2.chunk_id_for_glyph(gid).is_some()),
            "v2 manifest must map every render-ready GSUB closure glyph"
        );
    }

    #[test]
    fn devanagari_shaping_glyphs_are_included_in_chunks_and_manifest() {
        // HarfBuzz output for the pinned fixture. These are the original missing-glyph
        // reports; the production path remains script-agnostic.
        let shaped_cases: &[(&str, &[u16])] =
            &[("शि", &[594, 58]), ("ष्टो", &[547, 78]), ("त्त", &[506])];
        let mut codepoints = shaped_cases
            .iter()
            .flat_map(|(text, _)| text.chars().map(u32::from))
            .collect::<Vec<_>>();
        codepoints.sort_unstable();
        codepoints.dedup();
        let chunk_codepoints = vec![codepoints];

        let built = build_font(DEVANAGARI_FIXTURE, &chunk_codepoints).unwrap();
        let base = decompress_zstd(&built.base).unwrap();
        let payload_gids = chunk_gids(&built.chunks[0], &base);
        let manifest_v2 = editor_resource::FontManifest::from_bytes(
            &decompress_zstd(&built.manifest_v2).unwrap(),
        )
        .unwrap();

        for (text, expected_gids) in shaped_cases {
            assert!(
                expected_gids.iter().all(|gid| payload_gids.contains(gid)),
                "{text}의 shaping GID가 청크 payload에 모두 있어야 한다"
            );
            assert!(
                expected_gids
                    .iter()
                    .all(|&gid| manifest_v2.chunk_id_for_glyph(gid).is_some()),
                "{text}의 shaping GID가 v2 manifest에서 청크로 해석되어야 한다"
            );
        }
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
    fn build_font_rejects_corrupt_cff() {
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
        assert!(matches!(result, Err(ServerError::InvalidFont(_))));
    }

    #[test]
    fn build_font_rejects_font_without_any_outline_table() {
        let Some(data) = load_test_font() else {
            return;
        };
        let mut font = data.clone();
        let num_tables = u16::from_be_bytes([font[4], font[5]]);
        for i in 0..num_tables {
            let off = 12 + (i as usize) * 16;
            if &font[off..off + 4] == b"glyf" {
                font[off..off + 4].copy_from_slice(b"XXXX");
            }
        }
        let result = build_font(&font, &[vec![0x41]]);
        assert!(
            matches!(&result, Err(ServerError::InvalidFont(msg)) if msg.contains("unsplittable"))
        );
    }

    const CFF_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/SourceSans3-Regular.otf"
    ));
    const DEVANAGARI_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/NotoSansDevanagariUI-Regular.ttf"
    ));

    #[derive(Default)]
    struct RecordingPen(Vec<(u8, f32, f32)>);

    impl skrifa::outline::OutlinePen for RecordingPen {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.push((0, x, y));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.push((1, x, y));
        }
        fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
            self.0.push((2, cx0, cy0));
            self.0.push((2, x, y));
        }
        fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
            self.0.push((3, cx0, cy0));
            self.0.push((3, cx1, cy1));
            self.0.push((3, x, y));
        }
        fn close(&mut self) {
            self.0.push((4, 0.0, 0.0));
        }
    }

    fn draw_points(data: &[u8], ch: char) -> Vec<(u8, f32, f32)> {
        use skrifa::instance::{LocationRef, Size};
        use skrifa::outline::DrawSettings;
        let font = FontRef::new(data).unwrap();
        let gid = font.charmap().map(ch).unwrap();
        let glyph = font.outline_glyphs().get(gid).unwrap();
        let mut pen = RecordingPen::default();
        glyph
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut pen,
            )
            .unwrap();
        pen.0
    }

    #[test]
    fn build_font_converts_cff_and_splits() {
        let cps = get_font_codepoints(CFF_FIXTURE).unwrap();
        let chunk_cps: Vec<Vec<u32>> = cps.chunks(200).map(|c| c.to_vec()).collect();

        let built = build_font(CFF_FIXTURE, &chunk_cps).unwrap();
        assert_eq!(built.chunks.len(), chunk_cps.len());

        let base_raw = decompress_zstd(&built.base).unwrap();
        let base_font = FontRef::new(&base_raw).unwrap();
        assert!(base_font.glyf().is_ok());
        assert!(base_font.table_data(Tag::new(b"CFF ")).is_none());
    }

    #[test]
    fn build_font_cff_chunks_patch_back_into_base() {
        let cps = get_font_codepoints(CFF_FIXTURE).unwrap();
        let chunk_cps: Vec<Vec<u32>> = cps.chunks(200).map(|c| c.to_vec()).collect();
        let built = build_font(CFF_FIXTURE, &chunk_cps).unwrap();

        let mut base_raw = decompress_zstd(&built.base).unwrap();
        assert!(
            draw_points(&base_raw, 'A').is_empty(),
            "0으로 채워진 glyf의 글리프는 패치 전에 빈 아웃라인이어야 한다"
        );
        let glyf_offset = {
            let font = FontRef::new(&base_raw).unwrap();
            font.table_directory()
                .table_records()
                .iter()
                .find(|r| r.tag() == Tag::new(b"glyf"))
                .unwrap()
                .offset() as usize
        };

        for chunk in &built.chunks {
            let raw = decompress_zstd(chunk).unwrap();
            let count = u32::from_be_bytes(raw[0..4].try_into().unwrap()) as usize;
            let mut pos = 4;
            for _ in 0..count {
                let offset = u32::from_be_bytes(raw[pos..pos + 4].try_into().unwrap()) as usize;
                let len = u32::from_be_bytes(raw[pos + 4..pos + 8].try_into().unwrap()) as usize;
                let dst = glyf_offset + offset;
                base_raw[dst..dst + len].copy_from_slice(&raw[pos + 8..pos + 8 + len]);
                pos += 8 + len;
            }
        }

        let converted = convert_to_glyf(&FontRef::new(CFF_FIXTURE).unwrap()).unwrap();
        let patched_points = draw_points(&base_raw, 'A');
        assert!(
            !patched_points.is_empty(),
            "패치 후에는 실제 아웃라인이 그려져야 한다"
        );
        assert_eq!(
            patched_points,
            draw_points(&converted, 'A'),
            "패치 복원된 글리프는 변환본과 동일하게 그려져야 한다"
        );
    }

    #[test]
    fn build_font_hash_stable_for_cff_input() {
        let cp_groups = vec![vec![0x41u32]];
        let a = build_font(CFF_FIXTURE, &cp_groups).unwrap();
        let b = build_font(CFF_FIXTURE, &cp_groups).unwrap();
        assert_eq!(a.hash, b.hash);
    }

    const CFF2_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/SourceSans3VF-Upright.otf"
    ));

    #[test]
    fn build_font_rejects_cff2() {
        let result = build_font(CFF2_FIXTURE, &[vec![0x41]]);
        assert!(
            matches!(&result, Err(ServerError::InvalidFont(msg)) if msg.contains("unsplittable font")),
            "{result:?}"
        );
    }

    #[test]
    fn build_font_rejects_cff2_even_with_glyf() {
        let Some(data) = load_test_font() else {
            return;
        };
        let font = FontRef::new(&data).unwrap();
        let mut builder = FontBuilder::new();
        builder.add_raw(Tag::new(b"CFF2"), vec![0u8; 4]);
        builder.copy_missing_tables(font);
        let hybrid = builder.build();

        let result = build_font(&hybrid, &[vec![0x41]]);
        assert!(
            matches!(&result, Err(ServerError::InvalidFont(msg)) if msg.contains("unsplittable font")),
            "{result:?}"
        );
    }

    fn build_colr_test_font() -> Vec<u8> {
        use kurbo::Shape;
        use write_fonts::tables::cmap::Cmap;
        use write_fonts::tables::colr::{BaseGlyph as ColrBaseGlyph, Colr, Layer as ColrLayer};
        use write_fonts::tables::glyf::{
            Anchor, Bbox, Component, ComponentFlags, CompositeGlyph, GlyfLocaBuilder, Glyph,
            SimpleGlyph, Transform,
        };
        use write_fonts::tables::head::Head;
        use write_fonts::tables::loca::LocaFormat;
        use write_fonts::tables::maxp::Maxp;
        use write_fonts::types::{GlyphId, GlyphId16};

        let square = |o: f64| {
            let path = kurbo::Rect::new(o, 0.0, o + 100.0, 100.0).to_path(0.1);
            SimpleGlyph::from_bezpath(&path).unwrap()
        };

        let mut builder = GlyfLocaBuilder::new();
        builder.add_glyph(&square(0.0)).unwrap();
        builder.add_glyph(&Glyph::Empty).unwrap();
        builder.add_glyph(&square(100.0)).unwrap();
        let composite = CompositeGlyph::new(
            Component::new(
                GlyphId16::new(4),
                Anchor::Offset { x: 0, y: 0 },
                Transform::default(),
                ComponentFlags::default(),
            ),
            Bbox {
                x_min: 0,
                y_min: 0,
                x_max: 100,
                y_max: 100,
            },
        );
        builder.add_glyph(&composite).unwrap();
        builder.add_glyph(&square(200.0)).unwrap();
        builder.add_glyph(&square(300.0)).unwrap();
        let (glyf, loca, loca_format) = builder.build();

        let cmap = Cmap::from_mappings([('A', GlyphId::new(1)), ('B', GlyphId::new(5))]).unwrap();
        let colr = Colr::new(
            1,
            Some(vec![ColrBaseGlyph::new(GlyphId16::new(1), 0, 2)]),
            Some(vec![
                ColrLayer::new(GlyphId16::new(2), 0),
                ColrLayer::new(GlyphId16::new(3), 1),
            ]),
            2,
        );
        let head = Head {
            index_to_loc_format: match loca_format {
                LocaFormat::Short => 0,
                LocaFormat::Long => 1,
            },
            ..Head::default()
        };

        let mut fb = write_fonts::FontBuilder::new();
        fb.add_table(&glyf).unwrap();
        fb.add_table(&loca).unwrap();
        fb.add_table(&head).unwrap();
        fb.add_table(&Maxp::new(6)).unwrap();
        fb.add_table(&cmap).unwrap();
        fb.add_table(&colr).unwrap();
        fb.build()
    }

    fn chunk_gids(chunk: &[u8], font_data: &[u8]) -> HashSet<u16> {
        let font = FontRef::new(font_data).unwrap();
        let loca = font.loca(None).unwrap();
        let num_glyphs = font.maxp().unwrap().num_glyphs();
        let mut offset_to_gid: HashMap<usize, u16> = HashMap::new();
        for gid in 0..num_glyphs {
            let start = loca.get_raw(gid as usize).unwrap_or(0) as usize;
            let end = loca.get_raw(gid as usize + 1).unwrap_or(0) as usize;
            if start < end {
                offset_to_gid.insert(start, gid);
            }
        }
        let chunk = decompress_zstd(chunk).unwrap();
        let count = u32::from_be_bytes(chunk[0..4].try_into().unwrap()) as usize;
        let mut gids = HashSet::new();
        let mut pos = 4;
        for _ in 0..count {
            let offset = u32::from_be_bytes(chunk[pos..pos + 4].try_into().unwrap()) as usize;
            let len = u32::from_be_bytes(chunk[pos + 4..pos + 8].try_into().unwrap()) as usize;
            gids.insert(offset_to_gid[&offset]);
            pos += 8 + len;
        }
        gids
    }

    #[test]
    fn colr_layers_included_in_chunk() {
        let data = build_colr_test_font();
        let built = build_font(&data, &[vec![u32::from('A')]]).unwrap();
        let gids = chunk_gids(&built.chunks[0], &data);
        assert_eq!(gids, HashSet::from([2, 3, 4]), "레이어·composite 성분 포함");
    }

    #[test]
    fn colr_layers_not_pulled_for_unrelated_codepoint() {
        let data = build_colr_test_font();
        let built = build_font(&data, &[vec![u32::from('B')]]).unwrap();
        let gids = chunk_gids(&built.chunks[0], &data);
        assert_eq!(gids, HashSet::from([5]));
    }

    const MONA_COLR_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/Mona12ColorEmoji.ttf"
    ));

    fn patch_chunk_into_base(base: &[u8], chunk: &[u8]) -> Vec<u8> {
        let mut out = base.to_vec();
        let font = FontRef::new(base).unwrap();
        let glyf_start = font
            .table_directory()
            .table_records()
            .iter()
            .find(|r| r.tag() == Tag::new(b"glyf"))
            .unwrap()
            .offset() as usize;
        let chunk = decompress_zstd(chunk).unwrap();
        let count = u32::from_be_bytes(chunk[0..4].try_into().unwrap()) as usize;
        let mut pos = 4;
        for _ in 0..count {
            let offset = u32::from_be_bytes(chunk[pos..pos + 4].try_into().unwrap()) as usize;
            let len = u32::from_be_bytes(chunk[pos + 4..pos + 8].try_into().unwrap()) as usize;
            out[glyf_start + offset..glyf_start + offset + len]
                .copy_from_slice(&chunk[pos + 8..pos + 8 + len]);
            pos += 8 + len;
        }
        out
    }

    #[test]
    fn mona_colr_layers_survive_split_and_patch() {
        let cp = 0x1F600u32;
        let built = build_font(MONA_COLR_FIXTURE, &[vec![cp]]).unwrap();

        let font = FontRef::new(MONA_COLR_FIXTURE).unwrap();
        let base_gid = font.charmap().map(cp).unwrap().to_u32() as u16;
        let colr = font.colr().unwrap();
        let bases = colr.base_glyph_records().unwrap().unwrap();
        let record = bases
            .iter()
            .find(|b| b.glyph_id().to_u16() == base_gid)
            .unwrap();
        let layers = colr.layer_records().unwrap().unwrap();
        let start = usize::from(record.first_layer_index());
        let expected: HashSet<u16> = layers[start..start + usize::from(record.num_layers())]
            .iter()
            .map(|l| l.glyph_id().to_u16())
            .collect();
        assert!(!expected.is_empty());

        let gids = chunk_gids(&built.chunks[0], MONA_COLR_FIXTURE);
        assert!(
            expected.is_subset(&gids),
            "누락 레이어: {:?}",
            expected.difference(&gids).collect::<Vec<_>>()
        );

        let base_raw = decompress_zstd(&built.base).unwrap();
        let patched = patch_chunk_into_base(&base_raw, &built.chunks[0]);
        let orig_glyf_start = font
            .table_directory()
            .table_records()
            .iter()
            .find(|r| r.tag() == Tag::new(b"glyf"))
            .unwrap()
            .offset() as usize;
        let patched_font = FontRef::new(&patched).unwrap();
        let patched_glyf_start = patched_font
            .table_directory()
            .table_records()
            .iter()
            .find(|r| r.tag() == Tag::new(b"glyf"))
            .unwrap()
            .offset() as usize;
        let loca = font.loca(None).unwrap();
        for &gid in &expected {
            let s = loca.get_raw(gid as usize).unwrap() as usize;
            let e = loca.get_raw(gid as usize + 1).unwrap() as usize;
            assert!(s < e, "레이어 {gid}가 빈 글리프");
            assert_eq!(
                &patched[patched_glyf_start + s..patched_glyf_start + e],
                &MONA_COLR_FIXTURE[orig_glyf_start + s..orig_glyf_start + e],
                "레이어 {gid} glyf가 패치 후 원본과 불일치"
            );
        }
    }
}
