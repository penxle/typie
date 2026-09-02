use std::collections::BTreeMap;

use crate::error::ResourceError;

const MANIFEST_VERSION_V1: u8 = 1;
const MANIFEST_VERSION_V2: u8 = 2;
const MANIFEST_MAX_BYTES: usize = 1024 * 1024;
const MAX_CHUNKS: u16 = 255;

#[derive(Clone, Debug, PartialEq)]
pub struct FontManifest {
    pub chunk_count: u16,
    chunk_map: Vec<u8>,
    chunk_map_sup: Vec<u32>,
    num_glyphs: Option<u16>,
    glyph_chunks: Vec<Vec<u16>>,
}

impl FontManifest {
    pub fn new(chunk_count: u16, chunk_map: Vec<u8>, chunk_map_sup: Vec<u32>) -> Self {
        Self {
            chunk_count,
            chunk_map,
            chunk_map_sup,
            num_glyphs: None,
            glyph_chunks: Vec::new(),
        }
    }

    /// `coverages[i]` — chunk `i`의 flat 페어 `[start, end, start, end, ...]` (inclusive).
    pub fn from_coverages(coverages: &[Vec<u32>]) -> Self {
        let chunk_count = coverages.len() as u16;
        let mut per_chunk: Vec<Vec<u32>> = vec![Vec::new(); chunk_count as usize];
        for (idx, ranges) in coverages.iter().enumerate() {
            let bucket = &mut per_chunk[idx];
            for pair in ranges.as_chunks::<2>().0 {
                for cp in pair[0]..=pair[1] {
                    bucket.push(cp);
                }
            }
        }
        Self::from_chunk_codepoints(&per_chunk)
    }

    pub fn from_chunk_codepoints(chunk_codepoints: &[Vec<u32>]) -> Self {
        let chunk_count = chunk_codepoints.len() as u16;
        let mut bmp: Vec<(u32, u8)> = Vec::new();
        let mut sup: Vec<(u32, u32)> = Vec::new();

        for (idx, cps) in chunk_codepoints.iter().enumerate() {
            let idx_u8 = idx as u8;
            let idx_u32 = idx as u32;
            for &cp in cps {
                if cp <= 0xFFFF {
                    bmp.push((cp, idx_u8));
                } else {
                    sup.push((cp, idx_u32));
                }
            }
        }

        let chunk_map = Self::build_bmp_chunk_map(&bmp);
        let chunk_map_sup = Self::build_sup_chunk_map(&mut sup);

        Self {
            chunk_count,
            chunk_map,
            chunk_map_sup,
            num_glyphs: None,
            glyph_chunks: Vec::new(),
        }
    }

    pub fn with_glyph_chunks(
        mut self,
        num_glyphs: u16,
        glyph_chunks: Vec<Vec<u16>>,
    ) -> Result<Self, ResourceError> {
        Self::validate_glyph_chunks(self.chunk_count, num_glyphs, &glyph_chunks)?;
        self.num_glyphs = Some(num_glyphs);
        self.glyph_chunks = glyph_chunks;
        Ok(self)
    }

    pub fn num_glyphs(&self) -> Option<u16> {
        self.num_glyphs
    }

    pub fn chunk_id_for_glyph(&self, gid: u16) -> Option<u16> {
        self.glyph_chunks
            .iter()
            .position(|glyphs| glyphs.binary_search(&gid).is_ok())
            .map(|chunk_id| chunk_id as u16)
    }

    pub fn glyphs_in_chunk(&self, chunk_id: u16) -> Option<&[u16]> {
        self.num_glyphs?;
        self.glyph_chunks.get(chunk_id as usize).map(Vec::as_slice)
    }

    fn validate_glyph_chunks(
        chunk_count: u16,
        num_glyphs: u16,
        glyph_chunks: &[Vec<u16>],
    ) -> Result<(), ResourceError> {
        let err = |m: &str| ResourceError::InvalidFont(format!("manifest: {m}"));
        if glyph_chunks.len() != chunk_count as usize {
            return Err(err("glyph chunk count mismatch"));
        }
        for glyphs in glyph_chunks {
            let mut previous = None;
            for &gid in glyphs {
                if gid >= num_glyphs {
                    return Err(err("glyph id out of range"));
                }
                if previous.is_some_and(|previous| gid <= previous) {
                    return Err(err("glyph ids not strictly sorted"));
                }
                previous = Some(gid);
            }
        }
        Ok(())
    }

    fn build_bmp_chunk_map(entries: &[(u32, u8)]) -> Vec<u8> {
        if entries.is_empty() {
            return Vec::new();
        }

        let mut pages: BTreeMap<u8, [u8; 256]> = BTreeMap::new();

        for &(cp, idx) in entries {
            let hi = (cp >> 8) as u8;
            let lo = (cp & 0xFF) as usize;
            let page = pages.entry(hi).or_insert([0xFF; 256]);
            page[lo] = idx;
        }

        let mut l1 = [0xFFu8; 256];
        let mut l2_data: Vec<u8> = Vec::new();

        for (i, (&hi, page)) in pages.iter().enumerate() {
            l1[hi as usize] = i as u8;
            l2_data.extend_from_slice(page);
        }

        let mut chunk_map = Vec::with_capacity(256 + l2_data.len());
        chunk_map.extend_from_slice(&l1);
        chunk_map.extend_from_slice(&l2_data);
        chunk_map
    }

    fn build_sup_chunk_map(entries: &mut [(u32, u32)]) -> Vec<u32> {
        if entries.is_empty() {
            return Vec::new();
        }
        entries.sort_by_key(|&(cp, _)| cp);
        let mut result: Vec<u32> = Vec::with_capacity(entries.len() * 2);
        for &(cp, idx) in entries.iter() {
            if result.len() >= 2 && result[result.len() - 2] == cp {
                let last = result.len() - 1;
                result[last] = idx;
            } else {
                result.push(cp);
                result.push(idx);
            }
        }
        result
    }

    pub fn chunk_id(&self, cp: u32) -> Option<u16> {
        if cp <= 0xFFFF {
            self.bmp_lookup(cp)
        } else {
            self.supplementary_lookup(cp)
        }
    }

    pub fn has_codepoint(&self, cp: u32) -> bool {
        self.chunk_id(cp).is_some()
    }

    pub fn chunk_ids(&self, codepoints: &[u32]) -> Vec<u16> {
        let mut seen = hashbrown::HashSet::new();
        let mut result = Vec::new();
        for &cp in codepoints {
            if let Some(idx) = self.chunk_id(cp)
                && seen.insert(idx)
            {
                result.push(idx);
            }
        }
        result
    }

    pub fn all_chunk_ids(&self) -> std::ops::Range<u16> {
        0..self.chunk_count
    }

    fn bmp_lookup(&self, cp: u32) -> Option<u16> {
        if self.chunk_map.len() < 256 {
            return None;
        }
        let hi = (cp >> 8) as usize;
        let lo = (cp & 0xFF) as usize;
        let l2_idx = self.chunk_map[hi];
        if l2_idx == 0xFF {
            return None;
        }
        let offset = 256 + (l2_idx as usize) * 256 + lo;
        let chunk = *self.chunk_map.get(offset)?;
        if chunk == 0xFF {
            None
        } else {
            Some(chunk as u16)
        }
    }

    fn supplementary_lookup(&self, cp: u32) -> Option<u16> {
        let pairs = &self.chunk_map_sup;
        if pairs.is_empty() {
            return None;
        }
        let len = pairs.len() / 2;
        let mut lo = 0usize;
        let mut hi = len.wrapping_sub(1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            let key = pairs[mid * 2];
            if cp < key {
                if mid == 0 {
                    break;
                }
                hi = mid - 1;
            } else if cp > key {
                lo = mid + 1;
            } else {
                return Some(pairs[mid * 2 + 1] as u16);
            }
        }
        None
    }
}

impl FontManifest {
    pub fn to_bytes(&self) -> Vec<u8> {
        let glyph_entry_count: usize = self.glyph_chunks.iter().map(Vec::len).sum();
        let glyph_tail_len = self
            .num_glyphs
            .map(|_| 2 + (self.glyph_chunks.len() + 1) * 4 + glyph_entry_count * 2)
            .unwrap_or(0);
        let mut out = Vec::with_capacity(
            11 + self.chunk_map.len() + self.chunk_map_sup.len() * 4 + glyph_tail_len,
        );
        out.push(if self.num_glyphs.is_some() {
            MANIFEST_VERSION_V2
        } else {
            MANIFEST_VERSION_V1
        });
        out.extend_from_slice(&self.chunk_count.to_le_bytes());
        out.extend_from_slice(&(self.chunk_map.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.chunk_map);
        out.extend_from_slice(&(self.chunk_map_sup.len() as u32).to_le_bytes());
        for v in &self.chunk_map_sup {
            out.extend_from_slice(&v.to_le_bytes());
        }
        if let Some(num_glyphs) = self.num_glyphs {
            out.extend_from_slice(&num_glyphs.to_le_bytes());
            let mut offset = 0u32;
            out.extend_from_slice(&offset.to_le_bytes());
            for glyphs in &self.glyph_chunks {
                offset += glyphs.len() as u32;
                out.extend_from_slice(&offset.to_le_bytes());
            }
            for &gid in self.glyph_chunks.iter().flatten() {
                out.extend_from_slice(&gid.to_le_bytes());
            }
        }
        out
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, ResourceError> {
        let err = |m: &str| ResourceError::InvalidFont(format!("manifest: {m}"));
        if data.len() < 11 {
            return Err(err("too short"));
        }
        if data.len() > MANIFEST_MAX_BYTES {
            return Err(err("over size limit"));
        }
        let version = data[0];
        if version != MANIFEST_VERSION_V1 && version != MANIFEST_VERSION_V2 {
            return Err(err("unknown version"));
        }
        let chunk_count = u16::from_le_bytes(data[1..3].try_into().unwrap());
        if chunk_count > MAX_CHUNKS {
            return Err(err("chunk_count over limit"));
        }
        let map_len = u32::from_le_bytes(data[3..7].try_into().unwrap()) as usize;
        let map_end = 7usize.checked_add(map_len).ok_or_else(|| err("overflow"))?;
        let sup_header_end = map_end.checked_add(4).ok_or_else(|| err("overflow"))?;
        if data.len() < sup_header_end {
            return Err(err("chunk_map truncated"));
        }
        let chunk_map = data[7..map_end].to_vec();
        let sup_count =
            u32::from_le_bytes(data[map_end..sup_header_end].try_into().unwrap()) as usize;
        let sup_start = sup_header_end;
        let sup_end = sup_start
            .checked_add(sup_count.checked_mul(4).ok_or_else(|| err("overflow"))?)
            .ok_or_else(|| err("overflow"))?;
        if data.len() < sup_end || (version == MANIFEST_VERSION_V1 && data.len() != sup_end) {
            return Err(err("sup length mismatch"));
        }
        let chunk_map_sup: Vec<u32> = data[sup_start..sup_end]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();

        if !chunk_map.is_empty() {
            if chunk_map.len() < 256 || !(chunk_map.len() - 256).is_multiple_of(256) {
                return Err(err("chunk_map shape"));
            }
            let pages = (chunk_map.len() - 256) / 256;
            if pages > 255 {
                return Err(err("too many pages"));
            }
            for &l1 in &chunk_map[..256] {
                if l1 != 0xFF && (l1 as usize) >= pages {
                    return Err(err("l1 out of range"));
                }
            }
            for &id in &chunk_map[256..] {
                if id != 0xFF && u16::from(id) >= chunk_count {
                    return Err(err("chunk id out of range"));
                }
            }
        }
        if !chunk_map_sup.len().is_multiple_of(2) {
            return Err(err("sup not pairs"));
        }
        let mut prev: Option<u32> = None;
        for pair in chunk_map_sup.as_chunks::<2>().0 {
            if !(0x10000..=0x10FFFF).contains(&pair[0]) {
                return Err(err("sup key out of supplementary range"));
            }
            if let Some(p) = prev
                && pair[0] <= p
            {
                return Err(err("sup not sorted"));
            }
            if pair[1] >= u32::from(chunk_count) {
                return Err(err("sup chunk id out of range"));
            }
            prev = Some(pair[0]);
        }

        let (num_glyphs, glyph_chunks) = if version == MANIFEST_VERSION_V2 {
            let num_glyphs_end = sup_end.checked_add(2).ok_or_else(|| err("overflow"))?;
            let offsets_len = (usize::from(chunk_count) + 1)
                .checked_mul(4)
                .ok_or_else(|| err("overflow"))?;
            let offsets_end = num_glyphs_end
                .checked_add(offsets_len)
                .ok_or_else(|| err("overflow"))?;
            if data.len() < offsets_end {
                return Err(err("glyph table truncated"));
            }
            let num_glyphs = u16::from_le_bytes(data[sup_end..num_glyphs_end].try_into().unwrap());
            let offsets: Vec<usize> = data[num_glyphs_end..offsets_end]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|bytes| u32::from_le_bytes(*bytes) as usize)
                .collect();
            let entry_count = *offsets.last().expect("glyph offsets include the end");
            let glyphs_end = offsets_end
                .checked_add(entry_count.checked_mul(2).ok_or_else(|| err("overflow"))?)
                .ok_or_else(|| err("overflow"))?;
            if data.len() != glyphs_end {
                return Err(err("glyph table length mismatch"));
            }
            if offsets.first() != Some(&0) || offsets.windows(2).any(|pair| pair[0] > pair[1]) {
                return Err(err("invalid glyph offsets"));
            }
            let glyphs: Vec<u16> = data[offsets_end..glyphs_end]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|bytes| u16::from_le_bytes(*bytes))
                .collect();
            let glyph_chunks: Vec<Vec<u16>> = offsets
                .windows(2)
                .map(|pair| glyphs[pair[0]..pair[1]].to_vec())
                .collect();
            Self::validate_glyph_chunks(chunk_count, num_glyphs, &glyph_chunks)?;
            (Some(num_glyphs), glyph_chunks)
        } else {
            (None, Vec::new())
        };

        Ok(Self {
            chunk_count,
            chunk_map,
            chunk_map_sup,
            num_glyphs,
            glyph_chunks,
        })
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// Build a chunk_map where block `hi` maps low byte `lo` to `chunk_id`.
    /// 0xff means "not covered".
    fn make_manifest(entries: &[(u8, u8, u8)], sup: &[u32], chunk_count: u16) -> FontManifest {
        let mut chunk_map = vec![0xffu8; 256];
        let mut l2_blocks: BTreeMap<u8, [u8; 256]> = BTreeMap::new();

        for &(hi, lo, chunk_id) in entries {
            let l2 = l2_blocks.entry(hi).or_insert([0xff; 256]);
            l2[lo as usize] = chunk_id;
        }

        for (i, (&hi, _)) in l2_blocks.iter().enumerate() {
            chunk_map[hi as usize] = i as u8;
        }

        for block in l2_blocks.values() {
            chunk_map.extend_from_slice(block);
        }

        FontManifest {
            chunk_count,
            chunk_map,
            chunk_map_sup: sup.to_vec(),
            num_glyphs: None,
            glyph_chunks: Vec::new(),
        }
    }

    #[test]
    fn chunk_id_bmp_exists() {
        let m = make_manifest(&[(0x00, 0x41, 3)], &[], 8);
        assert_eq!(m.chunk_id(0x0041), Some(3));
    }

    #[test]
    fn chunk_id_bmp_l1_unmapped() {
        let m = make_manifest(&[(0x00, 0x41, 3)], &[], 8);
        assert_eq!(m.chunk_id(0x0100), None);
    }

    #[test]
    fn chunk_id_bmp_l2_unmapped() {
        let m = make_manifest(&[(0x00, 0x41, 3)], &[], 8);
        assert_eq!(m.chunk_id(0x0042), None);
    }

    #[test]
    fn chunk_id_supplementary_exists() {
        let m = make_manifest(&[], &[0x1F600, 5], 8);
        assert_eq!(m.chunk_id(0x1F600), Some(5));
    }

    #[test]
    fn chunk_id_supplementary_missing() {
        let m = make_manifest(&[], &[0x1F600, 5], 8);
        assert_eq!(m.chunk_id(0x1F601), None);
    }

    #[test]
    fn chunk_id_supplementary_empty_sup() {
        let m = make_manifest(&[], &[], 8);
        assert_eq!(m.chunk_id(0x10000), None);
    }

    #[test]
    fn has_codepoint_true() {
        let m = make_manifest(&[(0x00, 0x41, 3)], &[], 8);
        assert!(m.has_codepoint(0x0041));
    }

    #[test]
    fn has_codepoint_false() {
        let m = make_manifest(&[(0x00, 0x41, 3)], &[], 8);
        assert!(!m.has_codepoint(0x0042));
    }

    #[test]
    fn chunk_ids_deduplicates() {
        let m = make_manifest(&[(0x00, 0x41, 3), (0x00, 0x42, 3)], &[], 8);
        let result = m.chunk_ids(&[0x0041, 0x0042]);
        assert_eq!(result, vec![3]);
    }

    #[test]
    fn chunk_ids_multiple_subsets() {
        let m = make_manifest(&[(0x00, 0x41, 3), (0x00, 0x42, 5)], &[], 8);
        let mut result = m.chunk_ids(&[0x0041, 0x0042]);
        result.sort();
        assert_eq!(result, vec![3, 5]);
    }

    #[test]
    fn chunk_ids_mixed_existing_and_missing() {
        let m = make_manifest(&[(0x00, 0x41, 3)], &[], 8);
        let result = m.chunk_ids(&[0x0041, 0x0099]);
        assert_eq!(result, vec![3]);
    }

    #[test]
    fn chunk_ids_empty_input() {
        let m = make_manifest(&[(0x00, 0x41, 3)], &[], 8);
        let result = m.chunk_ids(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn chunk_ids_empty_chunk_map() {
        let m = FontManifest::new(0, vec![], vec![]);
        let result = m.chunk_ids(&[0x0041]);
        assert!(result.is_empty());
    }

    #[test]
    fn from_chunk_codepoints_bmp_single_subset() {
        let m = FontManifest::from_chunk_codepoints(&[vec![0x0041, 0x0042]]);
        assert_eq!(m.chunk_count, 1);
        assert_eq!(m.chunk_id(0x0041), Some(0));
        assert_eq!(m.chunk_id(0x0042), Some(0));
        assert_eq!(m.chunk_id(0x0043), None);
    }

    #[test]
    fn from_chunk_codepoints_bmp_multiple_subsets() {
        let m = FontManifest::from_chunk_codepoints(&[
            vec![0x0041],
            vec![0x0100, 0x0101],
            vec![0xAC00],
        ]);
        assert_eq!(m.chunk_count, 3);
        assert_eq!(m.chunk_id(0x0041), Some(0));
        assert_eq!(m.chunk_id(0x0100), Some(1));
        assert_eq!(m.chunk_id(0x0101), Some(1));
        assert_eq!(m.chunk_id(0xAC00), Some(2));
    }

    #[test]
    fn from_chunk_codepoints_supplementary() {
        let m = FontManifest::from_chunk_codepoints(&[vec![0x0041], vec![0x1F600, 0x1F601]]);
        assert_eq!(m.chunk_id(0x0041), Some(0));
        assert_eq!(m.chunk_id(0x1F600), Some(1));
        assert_eq!(m.chunk_id(0x1F601), Some(1));
        assert_eq!(m.chunk_id(0x1F602), None);
    }

    #[test]
    fn from_chunk_codepoints_mixed_bmp_and_supplementary() {
        let m =
            FontManifest::from_chunk_codepoints(&[vec![0x0041, 0x1F600], vec![0xAC00, 0x20000]]);
        assert_eq!(m.chunk_id(0x0041), Some(0));
        assert_eq!(m.chunk_id(0x1F600), Some(0));
        assert_eq!(m.chunk_id(0xAC00), Some(1));
        assert_eq!(m.chunk_id(0x20000), Some(1));
    }

    #[test]
    fn from_chunk_codepoints_empty() {
        let m = FontManifest::from_chunk_codepoints(&[]);
        assert_eq!(m.chunk_count, 0);
        assert_eq!(m.chunk_id(0x0041), None);
    }

    #[test]
    fn from_coverages_basic() {
        let coverages = vec![vec![0x41, 0x43], vec![0x1F600, 0x1F600]];
        let m = FontManifest::from_coverages(&coverages);
        assert_eq!(m.chunk_count, 2);
        assert_eq!(m.chunk_id(0x41), Some(0));
        assert_eq!(m.chunk_id(0x42), Some(0));
        assert_eq!(m.chunk_id(0x43), Some(0));
        assert_eq!(m.chunk_id(0x44), None);
        assert_eq!(m.chunk_id(0x1F600), Some(1));
        assert_eq!(m.chunk_id(0x1F601), None);
    }

    #[test]
    fn from_coverages_empty() {
        let coverages: Vec<Vec<u32>> = vec![];
        let m = FontManifest::from_coverages(&coverages);
        assert_eq!(m.chunk_count, 0);
        assert_eq!(m.chunk_id(0x41), None);
    }

    #[test]
    fn from_coverages_sparse_ids() {
        // chunk 0, 1은 비어 있고 chunk 2에만 coverage — empty inner Vec로 표현.
        let coverages = vec![vec![], vec![], vec![0x41, 0x41]];
        let m = FontManifest::from_coverages(&coverages);
        assert_eq!(m.chunk_count, 3);
        assert_eq!(m.chunk_id(0x41), Some(2));
    }

    #[test]
    fn to_bytes_from_bytes_roundtrip() {
        let coverages = vec![vec![0x41, 0x43, 0xAC00, 0xAC02], vec![0x1F600, 0x1F601]];
        let original = FontManifest::from_coverages(&coverages);
        let restored = FontManifest::from_bytes(&original.to_bytes()).unwrap();
        assert_eq!(restored, original);
        for cp in [
            0x41u32, 0x42, 0x43, 0x44, 0xAC00, 0xAC01, 0x1F600, 0x1F601, 0x1F602,
        ] {
            assert_eq!(restored.chunk_id(cp), original.chunk_id(cp), "cp={cp:#x}");
        }
    }

    #[test]
    fn v2_roundtrip_preserves_glyph_chunk_membership() {
        let original =
            FontManifest::from_coverages(&[vec![0x41, 0x41], vec![0x42, 0x42], vec![0x43, 0x43]])
                .with_glyph_chunks(12, vec![vec![2, 7], vec![4, 7, 9], vec![]])
                .unwrap();

        let bytes = original.to_bytes();
        assert_eq!(bytes[0], 2);

        let restored = FontManifest::from_bytes(&bytes).unwrap();
        assert_eq!(restored, original);
        assert_eq!(restored.num_glyphs(), Some(12));
        assert_eq!(restored.chunk_id_for_glyph(2), Some(0));
        assert_eq!(restored.chunk_id_for_glyph(4), Some(1));
        assert_eq!(restored.chunk_id_for_glyph(7), Some(0));
        assert_eq!(restored.chunk_id_for_glyph(11), None);
        assert_eq!(restored.glyphs_in_chunk(0), Some(&[2, 7][..]));
        assert_eq!(restored.glyphs_in_chunk(1), Some(&[4, 7, 9][..]));
        assert_eq!(restored.glyphs_in_chunk(2), Some(&[][..]));
    }

    #[test]
    fn v1_roundtrip_has_no_glyph_chunk_membership() {
        let original = FontManifest::from_coverages(&[vec![0x41, 0x41]]);
        let bytes = original.to_bytes();
        assert_eq!(bytes[0], 1);

        let restored = FontManifest::from_bytes(&bytes).unwrap();
        assert_eq!(restored.num_glyphs(), None);
        assert_eq!(restored.chunk_id_for_glyph(1), None);
        assert_eq!(restored.glyphs_in_chunk(0), None);
    }

    #[test]
    fn v2_rejects_invalid_glyph_chunk_tables() {
        let manifest = FontManifest::from_coverages(&[vec![0x41, 0x41]]);

        assert!(
            manifest
                .clone()
                .with_glyph_chunks(4, vec![vec![1], vec![2]])
                .is_err(),
            "chunk count mismatch"
        );
        assert!(
            manifest
                .clone()
                .with_glyph_chunks(4, vec![vec![2, 1]])
                .is_err(),
            "glyph ids must be strictly sorted"
        );
        assert!(
            manifest.with_glyph_chunks(4, vec![vec![4]]).is_err(),
            "glyph ids must be below numGlyphs"
        );
    }

    #[test]
    fn v2_parser_rejects_malformed_glyph_table() {
        let good = FontManifest::from_coverages(&[vec![0x41, 0x41]])
            .with_glyph_chunks(4, vec![vec![1, 2]])
            .unwrap()
            .to_bytes();
        let map_len = u32::from_le_bytes(good[3..7].try_into().unwrap()) as usize;
        let sup_count_offset = 7 + map_len;
        let sup_count = u32::from_le_bytes(
            good[sup_count_offset..sup_count_offset + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        let glyph_tail = sup_count_offset + 4 + sup_count * 4;
        let offsets = glyph_tail + 2;
        let glyphs = offsets + 8;

        assert!(FontManifest::from_bytes(&good[..good.len() - 1]).is_err());

        let mut bad_first_offset = good.clone();
        bad_first_offset[offsets..offsets + 4].copy_from_slice(&1u32.to_le_bytes());
        assert!(FontManifest::from_bytes(&bad_first_offset).is_err());

        let mut bad_last_offset = good.clone();
        bad_last_offset[offsets + 4..offsets + 8].copy_from_slice(&3u32.to_le_bytes());
        assert!(FontManifest::from_bytes(&bad_last_offset).is_err());

        let mut duplicate_gid = good.clone();
        duplicate_gid[glyphs + 2..glyphs + 4].copy_from_slice(&1u16.to_le_bytes());
        assert!(FontManifest::from_bytes(&duplicate_gid).is_err());

        let mut out_of_range_gid = good;
        out_of_range_gid[glyphs + 2..glyphs + 4].copy_from_slice(&4u16.to_le_bytes());
        assert!(FontManifest::from_bytes(&out_of_range_gid).is_err());
    }

    #[test]
    fn from_bytes_rejects_truncated_and_version() {
        let m = FontManifest::from_coverages(&[vec![0x41, 0x41]]);
        let bytes = m.to_bytes();
        assert!(FontManifest::from_bytes(&bytes[..bytes.len() - 1]).is_err());
        assert!(FontManifest::from_bytes(&[]).is_err());
        let mut bad = bytes.clone();
        bad[0] = 99;
        assert!(FontManifest::from_bytes(&bad).is_err());
    }

    #[test]
    fn from_bytes_rejects_structural_corruption() {
        let m = FontManifest::from_coverages(&[vec![0x41, 0x41], vec![0x100, 0x100]]);
        let good = m.to_bytes();

        // chunk_count를 실제 ID보다 작게 조작 → ID >= chunk_count 검출
        let mut bad_count = good.clone();
        bad_count[1] = 1;
        bad_count[2] = 0;
        assert!(FontManifest::from_bytes(&bad_count).is_err());

        // chunk_map 길이를 페이지 단위가 아니게 조작
        let mut bad_map = good.clone();
        let map_len = u32::from_le_bytes(good[3..7].try_into().unwrap());
        bad_map[3..7].copy_from_slice(&(map_len - 1).to_le_bytes());
        bad_map.remove(7 + (map_len as usize) - 1);
        assert!(FontManifest::from_bytes(&bad_map).is_err());

        // chunk_count > 255 거부
        let mut bad_over = good.clone();
        bad_over[1..3].copy_from_slice(&256u16.to_le_bytes());
        assert!(FontManifest::from_bytes(&bad_over).is_err());
    }

    #[test]
    fn from_bytes_accepts_254_and_255_chunks_boundary() {
        for count in [254u32, 255] {
            let coverages: Vec<Vec<u32>> = (0..count).map(|i| vec![i, i]).collect();
            let m = FontManifest::from_coverages(&coverages);
            let restored = FontManifest::from_bytes(&m.to_bytes()).unwrap();
            assert_eq!(restored.chunk_count, count as u16);
            assert_eq!(restored.chunk_id(count - 1), Some((count - 1) as u16));
        }
    }

    #[test]
    fn duplicate_supplementary_normalizes_last_wins() {
        // 0x1F600이 chunk 0과 chunk 1 양쪽 coverage에 등장 — BMP와 동일하게 뒤 청크 승리
        let m = FontManifest::from_coverages(&[vec![0x1F600, 0x1F600], vec![0x1F600, 0x1F600]]);
        assert_eq!(m.chunk_id(0x1F600), Some(1));
        let restored = FontManifest::from_bytes(&m.to_bytes()).unwrap();
        assert_eq!(restored.chunk_id(0x1F600), Some(1));
    }

    #[test]
    fn from_bytes_rejects_bmp_key_in_sup() {
        // sup 배열에 BMP 코드포인트(0xFFFF 이하)가 들어오면 거부 — 도달 불가 엔트리
        let m = FontManifest::new(1, Vec::new(), vec![0x0041, 0]);
        assert!(FontManifest::from_bytes(&m.to_bytes()).is_err());
    }

    #[test]
    fn from_bytes_rejects_each_malformed_case() {
        // 검증 항목별 직접 케이스 — new()로 위반 구조를 만들어 to_bytes → from_bytes 거부 확인
        // ① odd sup
        assert!(
            FontManifest::from_bytes(&FontManifest::new(1, Vec::new(), vec![0x10000]).to_bytes())
                .is_err()
        );
        // ② 내림차순 sup
        assert!(
            FontManifest::from_bytes(
                &FontManifest::new(1, Vec::new(), vec![0x10001, 0, 0x10000, 0]).to_bytes()
            )
            .is_err()
        );
        // ③ Unicode 상한 초과 sup key
        assert!(
            FontManifest::from_bytes(
                &FontManifest::new(1, Vec::new(), vec![0x110000, 0]).to_bytes()
            )
            .is_err()
        );
        // ④ sup chunk ID 범위 초과
        assert!(
            FontManifest::from_bytes(
                &FontManifest::new(1, Vec::new(), vec![0x10000, 5]).to_bytes()
            )
            .is_err()
        );
        // ⑤ L1이 존재하지 않는 페이지를 참조
        let mut l1_bad = vec![0xFFu8; 256];
        l1_bad[0] = 3; // 페이지 0개인데 3번 참조
        assert!(
            FontManifest::from_bytes(&FontManifest::new(1, l1_bad, Vec::new()).to_bytes()).is_err()
        );
        // ⑥ trailing bytes
        let good = FontManifest::from_coverages(&[vec![0x41, 0x41]]).to_bytes();
        let mut trailing = good.clone();
        trailing.push(0);
        assert!(FontManifest::from_bytes(&trailing).is_err());
        // ⑦ 256 BMP 페이지 — 수조립 chunk_map(256 + 256×256 바이트)으로 직접 거부 확인
        let overfull = vec![0u8; 256 + 256 * 256];
        assert!(
            FontManifest::from_bytes(&FontManifest::new(1, overfull, Vec::new()).to_bytes())
                .is_err()
        );
    }

    #[test]
    fn triple_and_nonadjacent_duplicate_sup_normalizes_last_wins() {
        // 같은 sup 코드포인트가 3개 청크에 등장 — 마지막 청크 승리
        let m = FontManifest::from_coverages(&[
            vec![0x1F600, 0x1F600],
            vec![0x1F600, 0x1F600, 0x1F700, 0x1F700],
            vec![0x1F600, 0x1F600],
        ]);
        assert_eq!(m.chunk_id(0x1F600), Some(2));
        assert_eq!(m.chunk_id(0x1F700), Some(1));
        let restored = FontManifest::from_bytes(&m.to_bytes()).unwrap();
        assert_eq!(restored.chunk_id(0x1F600), Some(2));
    }

    #[test]
    fn decompress_zstd_capped_rejects_oversized() {
        let big = vec![0u8; 2 * 1024 * 1024];
        let compressed = crate::zstd::compress_zstd(&big);
        assert!(crate::zstd::decompress_zstd_capped(&compressed, 1024 * 1024).is_err());
        let small = crate::zstd::compress_zstd(&[1u8, 2, 3]);
        assert_eq!(
            crate::zstd::decompress_zstd_capped(&small, 1024 * 1024).unwrap(),
            vec![1u8, 2, 3]
        );
    }

    proptest::proptest! {
        #[test]
        fn roundtrip_preserves_lookup(
            coverages in proptest::collection::vec(
                proptest::collection::vec(0u32..0x2FFFF, 0..24).prop_map(|mut cps| {
                    // 서로게이트 영역은 유효 입력 계약(빌더가 거부) 밖 — generator에서 제외
                    for cp in cps.iter_mut() {
                        if (0xD800..=0xDFFF).contains(cp) {
                            *cp += 0x800;
                        }
                    }
                    cps.sort_unstable();
                    cps.dedup();
                    cps.iter().flat_map(|&c| [c, c]).collect::<Vec<u32>>()
                }),
                0..=255,
            ),
            samples in proptest::collection::vec(0u32..0x30000, 0..64),
        ) {
            let original = FontManifest::from_coverages(&coverages);
            let restored = FontManifest::from_bytes(&original.to_bytes()).unwrap();
            proptest::prop_assert_eq!(&restored, &original);
            for cp in samples {
                proptest::prop_assert_eq!(restored.chunk_id(cp), original.chunk_id(cp));
            }
        }
    }
}
