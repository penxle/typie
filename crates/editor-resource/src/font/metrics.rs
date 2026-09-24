//! A font with the source's shaping and advance metrics, but no visible glyphs.
//! Browsers may recolor find-in-page text despite CSS transparency.
use skrifa::Tag;
use skrifa::raw::{FontRef, TableProvider};
use write_fonts::tables::{
    cmap::{Cmap, CmapSubtable, ConstantMapGroup, EncodingRecord, PlatformId},
    hhea::Hhea,
    maxp::Maxp,
    post::Post,
};
use write_fonts::{FontBuilder, from_obj::ToOwnedTable};

use crate::ResourceError;

pub(super) fn metrics_font(data: &[u8]) -> Result<Vec<u8>, ResourceError> {
    let font = FontRef::new(data).map_err(|e| ResourceError::InvalidFont(e.to_string()))?;
    font.head()
        .map_err(|e| ResourceError::InvalidFont(e.to_string()))?;
    let mut count = font
        .maxp()
        .map_err(|e| ResourceError::InvalidFont(e.to_string()))?
        .num_glyphs() as usize;
    let mut builder = FontBuilder::new();
    // Preserve shaping, glyph IDs, advances and vertical metrics. Omit outlines,
    // color/bitmap glyphs and their hinting programs entirely.
    for tag in [
        b"hhea", b"maxp", b"OS/2", b"hmtx", b"cmap", b"name", b"post", b"GDEF", b"GPOS", b"GSUB",
        b"kern", b"vhea", b"vmtx", b"BASE", b"morx", b"mort", b"kerx", b"ankr", b"trak", b"feat",
    ] {
        let tag = Tag::new(tag);
        if let Some(table) = font.table_data(tag) {
            builder.add_raw(tag, table.as_bytes());
        }
    }
    let mut head = font
        .table_data(Tag::new(b"head"))
        .ok_or_else(|| ResourceError::InvalidFont("head missing".into()))?
        .as_ref()
        .to_vec();
    head[50..52].copy_from_slice(&0_i16.to_be_bytes());
    builder.add_raw(Tag::new(b"head"), &head);
    // The engine's placeholder contains only .notdef and has no cmap entries.
    // Browsers would fall back to a visible system font for every character.
    // Give its empty glyph a nonzero ID and cover the full Unicode repertoire.
    if count == 1 {
        count = 2;
        let mut maxp: Maxp = font.maxp().unwrap().to_owned_table();
        maxp.num_glyphs = 2;
        maxp.max_points = Some(1);
        maxp.max_contours = Some(1);
        let mut hhea: Hhea = font
            .hhea()
            .map_err(|e| ResourceError::InvalidFont(e.to_string()))?
            .to_owned_table();
        hhea.number_of_h_metrics = 2;
        let hmtx = font
            .hmtx()
            .map_err(|e| ResourceError::InvalidFont(e.to_string()))?;
        let advance = hmtx
            .h_metrics()
            .first()
            .ok_or_else(|| ResourceError::InvalidFont("placeholder advance missing".into()))?;
        let mut metrics = Vec::with_capacity(8);
        for _ in 0..2 {
            metrics.extend_from_slice(&advance.advance().to_be_bytes());
            metrics.extend_from_slice(&advance.side_bearing().to_be_bytes());
        }
        let cmap = Cmap::new(vec![EncodingRecord::new(
            PlatformId::Windows,
            10,
            CmapSubtable::format_13(28, 0, 1, vec![ConstantMapGroup::new(0, 0x10ffff, 1)]),
        )]);
        builder
            .add_table(&maxp)
            .and_then(|b| b.add_table(&hhea))
            .and_then(|b| b.add_table(&cmap))
            .and_then(|b| b.add_table(&Post::new_v2([".notdef", "space"])))
            .map_err(|e| ResourceError::InvalidFont(e.to_string()))?;
        builder.add_raw(Tag::new(b"hmtx"), metrics);
    }
    // A zero-area contour for .notdef keeps the glyf table nonempty. Some browser
    // sanitizers reject fonts whose entire glyf table has zero length.
    let glyf = [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x31, 0];
    let mut loca = vec![0u8; (count + 1) * 2];
    for offset in loca.as_chunks_mut::<2>().0.iter_mut().skip(1) {
        offset.copy_from_slice(&8_u16.to_be_bytes());
    }
    builder.add_raw(Tag::new(b"glyf"), &glyf);
    builder.add_raw(Tag::new(b"loca"), &loca);
    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_shaping_and_metrics_without_glyph_outlines() {
        let source = include_bytes!("../../../editor-view/assets/test-font.ttf");
        let bytes = metrics_font(source).unwrap();
        let original = FontRef::new(source).unwrap();
        let font = FontRef::new(&bytes).unwrap();
        for tag in [b"hmtx", b"cmap", b"GSUB", b"GPOS"] {
            assert_eq!(
                original.table_data(Tag::new(tag)).map(|t| t.as_bytes()),
                font.table_data(Tag::new(tag)).map(|t| t.as_bytes())
            );
        }
        let loca = font.loca(None).unwrap();
        for gid in 1..font.maxp().unwrap().num_glyphs() as usize {
            assert_eq!(loca.get_raw(gid), loca.get_raw(gid + 1));
        }
        assert_eq!(font.glyf().unwrap().offset_data().as_bytes().len(), 16);
    }

    #[test]
    fn placeholder_covers_characters_without_browser_font_fallback() {
        let source = include_bytes!("../../assets/placeholder.ttf");
        let bytes = metrics_font(source).unwrap();
        let original = FontRef::new(source).unwrap();
        let font = FontRef::new(&bytes).unwrap();
        assert_eq!(font.maxp().unwrap().num_glyphs(), 2);
        assert_eq!(
            font.hmtx().unwrap().h_metrics()[1].advance(),
            original.hmtx().unwrap().h_metrics()[0].advance()
        );
        assert_eq!(
            skrifa::MetadataProvider::charmap(&skrifa::FontRef::new(&bytes).unwrap())
                .map('한')
                .unwrap()
                .to_u32(),
            1
        );
        let loca = font.loca(None).unwrap();
        for gid in 1..font.maxp().unwrap().num_glyphs() as usize {
            assert_eq!(loca.get_raw(gid), loca.get_raw(gid + 1));
        }
        assert_eq!(font.glyf().unwrap().offset_data().as_bytes().len(), 16);
    }
}
