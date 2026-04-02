use std::collections::BTreeMap;

use bitcode::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct FontManifest {
    pub chunk_count: u16,
    chunk_map: Vec<u8>,
    chunk_map_sup: Vec<u32>,
}

impl FontManifest {
    pub fn new(chunk_count: u16, chunk_map: Vec<u8>, chunk_map_sup: Vec<u32>) -> Self {
        Self {
            chunk_count,
            chunk_map,
            chunk_map_sup,
        }
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
        }
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
        entries.iter().flat_map(|&(cp, idx)| [cp, idx]).collect()
    }

    pub fn chunk_index(&self, cp: u32) -> Option<u16> {
        if cp <= 0xFFFF {
            self.bmp_lookup(cp)
        } else {
            self.supplementary_lookup(cp)
        }
    }

    pub fn has_codepoint(&self, cp: u32) -> bool {
        self.chunk_index(cp).is_some()
    }

    pub fn chunk_indices(&self, codepoints: &[u32]) -> Vec<u16> {
        let mut seen = hashbrown::HashSet::new();
        let mut result = Vec::new();
        for &cp in codepoints {
            if let Some(idx) = self.chunk_index(cp) {
                if seen.insert(idx) {
                    result.push(idx);
                }
            }
        }
        result
    }

    pub fn all_chunk_indices(&self) -> std::ops::Range<u16> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a chunk_map where block `hi` maps low byte `lo` to `chunk_idx`.
    /// 0xff means "not covered".
    fn make_manifest(entries: &[(u8, u8, u8)], sup: &[u32], chunk_count: u16) -> FontManifest {
        let mut chunk_map = vec![0xffu8; 256];
        let mut l2_blocks: BTreeMap<u8, [u8; 256]> = BTreeMap::new();

        for &(hi, lo, chunk_idx) in entries {
            let l2 = l2_blocks.entry(hi).or_insert([0xff; 256]);
            l2[lo as usize] = chunk_idx;
        }

        for (i, (&hi, _)) in l2_blocks.iter().enumerate() {
            chunk_map[hi as usize] = i as u8;
        }

        for (_, block) in &l2_blocks {
            chunk_map.extend_from_slice(block);
        }

        FontManifest {
            chunk_count,
            chunk_map,
            chunk_map_sup: sup.to_vec(),
        }
    }

    #[test]
    fn chunk_index_bmp_exists() {
        let m = make_manifest(&[(0x00, 0x41, 3)], &[], 8);
        assert_eq!(m.chunk_index(0x0041), Some(3));
    }

    #[test]
    fn chunk_index_bmp_l1_unmapped() {
        let m = make_manifest(&[(0x00, 0x41, 3)], &[], 8);
        assert_eq!(m.chunk_index(0x0100), None);
    }

    #[test]
    fn chunk_index_bmp_l2_unmapped() {
        let m = make_manifest(&[(0x00, 0x41, 3)], &[], 8);
        assert_eq!(m.chunk_index(0x0042), None);
    }

    #[test]
    fn chunk_index_supplementary_exists() {
        let m = make_manifest(&[], &[0x1F600, 5], 8);
        assert_eq!(m.chunk_index(0x1F600), Some(5));
    }

    #[test]
    fn chunk_index_supplementary_missing() {
        let m = make_manifest(&[], &[0x1F600, 5], 8);
        assert_eq!(m.chunk_index(0x1F601), None);
    }

    #[test]
    fn chunk_index_supplementary_empty_sup() {
        let m = make_manifest(&[], &[], 8);
        assert_eq!(m.chunk_index(0x10000), None);
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
    fn chunk_indices_deduplicates() {
        let m = make_manifest(&[(0x00, 0x41, 3), (0x00, 0x42, 3)], &[], 8);
        let result = m.chunk_indices(&[0x0041, 0x0042]);
        assert_eq!(result, vec![3]);
    }

    #[test]
    fn chunk_indices_multiple_chunks() {
        let m = make_manifest(&[(0x00, 0x41, 3), (0x00, 0x42, 5)], &[], 8);
        let mut result = m.chunk_indices(&[0x0041, 0x0042]);
        result.sort();
        assert_eq!(result, vec![3, 5]);
    }

    #[test]
    fn chunk_indices_mixed_existing_and_missing() {
        let m = make_manifest(&[(0x00, 0x41, 3)], &[], 8);
        let result = m.chunk_indices(&[0x0041, 0x0099]);
        assert_eq!(result, vec![3]);
    }

    #[test]
    fn chunk_indices_empty_input() {
        let m = make_manifest(&[(0x00, 0x41, 3)], &[], 8);
        let result = m.chunk_indices(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn chunk_indices_empty_chunk_map() {
        let m = FontManifest::new(0, vec![], vec![]);
        let result = m.chunk_indices(&[0x0041]);
        assert!(result.is_empty());
    }

    #[test]
    fn from_chunk_codepoints_bmp_single_chunk() {
        let m = FontManifest::from_chunk_codepoints(&[vec![0x0041, 0x0042]]);
        assert_eq!(m.chunk_count, 1);
        assert_eq!(m.chunk_index(0x0041), Some(0));
        assert_eq!(m.chunk_index(0x0042), Some(0));
        assert_eq!(m.chunk_index(0x0043), None);
    }

    #[test]
    fn from_chunk_codepoints_bmp_multiple_chunks() {
        let m = FontManifest::from_chunk_codepoints(&[
            vec![0x0041],
            vec![0x0100, 0x0101],
            vec![0xAC00],
        ]);
        assert_eq!(m.chunk_count, 3);
        assert_eq!(m.chunk_index(0x0041), Some(0));
        assert_eq!(m.chunk_index(0x0100), Some(1));
        assert_eq!(m.chunk_index(0x0101), Some(1));
        assert_eq!(m.chunk_index(0xAC00), Some(2));
    }

    #[test]
    fn from_chunk_codepoints_supplementary() {
        let m = FontManifest::from_chunk_codepoints(&[vec![0x0041], vec![0x1F600, 0x1F601]]);
        assert_eq!(m.chunk_index(0x0041), Some(0));
        assert_eq!(m.chunk_index(0x1F600), Some(1));
        assert_eq!(m.chunk_index(0x1F601), Some(1));
        assert_eq!(m.chunk_index(0x1F602), None);
    }

    #[test]
    fn from_chunk_codepoints_mixed_bmp_and_supplementary() {
        let m =
            FontManifest::from_chunk_codepoints(&[vec![0x0041, 0x1F600], vec![0xAC00, 0x20000]]);
        assert_eq!(m.chunk_index(0x0041), Some(0));
        assert_eq!(m.chunk_index(0x1F600), Some(0));
        assert_eq!(m.chunk_index(0xAC00), Some(1));
        assert_eq!(m.chunk_index(0x20000), Some(1));
    }

    #[test]
    fn from_chunk_codepoints_empty() {
        let m = FontManifest::from_chunk_codepoints(&[]);
        assert_eq!(m.chunk_count, 0);
        assert_eq!(m.chunk_index(0x0041), None);
    }

    #[test]
    fn from_chunk_codepoints_bitcode_roundtrip() {
        let m = FontManifest::from_chunk_codepoints(&[vec![0x0041, 0x0042], vec![0x1F600]]);
        let encoded = bitcode::encode(&m);
        let decoded: FontManifest = bitcode::decode(&encoded).unwrap();
        assert_eq!(decoded.chunk_index(0x0041), Some(0));
        assert_eq!(decoded.chunk_index(0x1F600), Some(1));
    }
}
