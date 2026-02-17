use anyhow::{Context, Result};
use serde::Serialize;
use skrifa::raw::tables::glyf::{Glyf, Glyph};
use skrifa::raw::tables::gsub::{
    AlternateSubstFormat1, LigatureSubstFormat1, SingleSubst, SubstitutionSubtables,
};
use skrifa::raw::tables::loca::Loca;
use skrifa::raw::{FontRef, TableProvider};
use skrifa::{GlyphId, MetadataProvider, Tag};
use std::collections::{HashMap, HashSet};
use write_fonts::FontBuilder;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi))]
pub struct EncodedFont {
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
    pub base: Vec<u8>,
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array[]"))]
    pub chunks: Vec<serde_bytes::ByteBuf>,
}

pub fn get_font_codepoints(ttf_data: &[u8]) -> Result<Vec<u32>> {
    let font = FontRef::new(ttf_data)?;
    let mut codepoints: Vec<u32> = font.charmap().mappings().map(|(cp, _)| cp).collect();
    codepoints.sort();
    codepoints.dedup();
    Ok(codepoints)
}

pub fn encode_font(ttf_data: &[u8], chunk_codepoints: &[Vec<u32>]) -> Result<EncodedFont> {
    let font = FontRef::new(ttf_data)?;

    let has_glyf = font.glyf().is_ok();
    let has_cbdt = font.table_data(Tag::new(b"CBDT")).is_some()
        && font.table_data(Tag::new(b"CBLC")).is_some();

    let charmap = font.charmap();
    let mut cp_to_gid: HashMap<u32, u16> = HashMap::new();
    for (cp, gid) in charmap.mappings() {
        cp_to_gid.insert(cp, gid.to_u32() as u16);
    }

    let num_glyphs = font.maxp()?.num_glyphs();
    let gsub_alternates = resolve_gsub_alternates(&font);

    let mut per_glyph: HashMap<u16, (usize, Vec<u8>)> = HashMap::new();
    let mut composite_deps: HashMap<u16, HashSet<u16>> = HashMap::new();
    let mut table_overrides: Vec<(Tag, Vec<u8>)> = Vec::new();
    let mut split_tag: Option<Tag> = None;

    if has_cbdt {
        let cbdt_raw = font.table_data(Tag::new(b"CBDT")).context("CBDT missing")?;
        let cblc = font.cblc()?;

        for size in cblc.bitmap_sizes() {
            let start_gid = size.start_glyph_index().to_u32();
            let end_gid = size.end_glyph_index().to_u32();
            for gid in start_gid..=end_gid {
                if let Ok(loc) = size.location(cblc.offset_data(), GlyphId::new(gid)) {
                    if loc.data_size > 0 {
                        let start = loc.data_offset;
                        let end = start + loc.data_size;
                        per_glyph
                            .insert(gid as u16, (start, cbdt_raw.as_ref()[start..end].to_vec()));
                    }
                }
            }
        }

        table_overrides.push((Tag::new(b"CBDT"), vec![0u8; cbdt_raw.len()]));
        let cblc_raw = font.table_data(Tag::new(b"CBLC")).context("CBLC missing")?;
        table_overrides.push((Tag::new(b"CBLC"), cblc_raw.as_ref().to_vec()));
        split_tag = Some(Tag::new(b"CBDT"));
    } else if has_glyf {
        let glyf_raw = font.table_data(Tag::new(b"glyf")).context("glyf missing")?;
        let loca_raw = font.table_data(Tag::new(b"loca")).context("loca missing")?;
        let loca = font.loca(None)?;
        let glyf = font.glyf()?;

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

    let mut builder = FontBuilder::new();
    for (tag, data) in &table_overrides {
        builder.add_raw(*tag, data.as_slice());
    }
    builder.copy_missing_tables(font);
    let base_data = builder.build();

    let split_offset = if let Some(tag) = split_tag {
        let built_font = FontRef::new(&base_data)?;
        built_font
            .table_directory()
            .table_records()
            .iter()
            .find(|r| r.tag() == tag)
            .map(|r| r.offset())
            .unwrap_or(0)
    } else {
        0
    };

    let mut base_with_header = Vec::with_capacity(4 + base_data.len());
    base_with_header.extend_from_slice(&split_offset.to_be_bytes());
    base_with_header.extend_from_slice(&base_data);

    Ok(EncodedFont {
        base: base_with_header,
        chunks: chunks.into_iter().map(serde_bytes::ByteBuf::from).collect(),
    })
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
                                for lig in lig_set.ligatures().iter() {
                                    if let Ok(lig) = lig {
                                        alternates
                                            .entry(src)
                                            .or_default()
                                            .insert(lig.ligature_glyph().to_u32() as u16);
                                    }
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
