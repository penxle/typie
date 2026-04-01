use hashbrown::HashMap;
use smallvec::SmallVec;
use std::sync::Arc;

use super::data::FontData;
use super::tpft::decode_tpft;
use crate::error::ResourceError;

struct FontEntry {
    data: Arc<FontData>,
    split_offset: usize,
}

pub struct FontRegistry {
    families: HashMap<String, SmallVec<[u16; 9]>>,
    family_names: Vec<String>,
    family_index: HashMap<String, u16>,
    codepoint_mappings: HashMap<(u16, u16), HashMap<u32, (u16, u16)>>,
    font_entries: HashMap<(u16, u16), FontEntry>,
    font_versions: HashMap<(u16, u16), u64>,
}

impl FontRegistry {
    pub fn new() -> Self {
        Self {
            families: HashMap::default(),
            family_names: Vec::new(),
            family_index: HashMap::default(),
            codepoint_mappings: HashMap::default(),
            font_entries: HashMap::default(),
            font_versions: HashMap::default(),
        }
    }

    pub fn update(&mut self, families: HashMap<String, Vec<u16>>) {
        self.families.clear();
        self.codepoint_mappings.clear();
        for (name, mut weights) in families {
            weights.sort_unstable();
            weights.dedup();
            self.families.insert(name, SmallVec::from_vec(weights));
        }
    }

    pub fn has_family(&self, family: &str) -> bool {
        self.families.contains_key(family)
    }

    pub fn weights(&self, family: &str) -> Option<&[u16]> {
        self.families.get(family).map(|w| w.as_slice())
    }

    pub fn has_weight(&self, family: &str, weight: u16) -> bool {
        self.families
            .get(family)
            .map_or(false, |w| w.contains(&weight))
    }

    pub fn nearest_weight(&self, family: &str, target: u16) -> Option<u16> {
        let weights = self.families.get(family)?;
        if weights.is_empty() {
            return None;
        }

        let idx = weights.partition_point(|&w| w < target);

        let before = if idx > 0 {
            Some(weights[idx - 1])
        } else {
            None
        };

        let after = weights.get(idx).copied();

        match (before, after) {
            (Some(b), Some(a)) => {
                if (target - b) <= (a - target) {
                    Some(b)
                } else {
                    Some(a)
                }
            }
            (Some(b), None) => Some(b),
            (None, Some(a)) => Some(a),
            (None, None) => None,
        }
    }

    pub fn intern(&mut self, family: &str) -> u16 {
        if let Some(&id) = self.family_index.get(family) {
            return id;
        }

        let id = self.family_names.len() as u16;
        self.family_names.push(family.to_string());
        self.family_index.insert(family.to_string(), id);

        id
    }

    pub fn intern_id(&self, family: &str) -> Option<u16> {
        self.family_index.get(family).copied()
    }

    pub fn resolve(&self, id: u16) -> &str {
        &self.family_names[id as usize]
    }

    pub fn codepoint_map(&self, family_id: u16, weight: u16) -> Option<&HashMap<u32, (u16, u16)>> {
        self.codepoint_mappings.get(&(family_id, weight))
    }

    pub fn add_codepoint_mapping(
        &mut self,
        family_id: u16,
        weight: u16,
        codepoint: u32,
        resolved_family_id: u16,
        resolved_weight: u16,
    ) {
        self.codepoint_mappings
            .entry((family_id, weight))
            .or_default()
            .insert(codepoint, (resolved_family_id, resolved_weight));
    }

    pub fn add_font_base(
        &mut self,
        id: u16,
        weight: u16,
        data: &[u8],
    ) -> Result<(), ResourceError> {
        let data = decode_tpft(data)?;
        if data.len() < 4 {
            return Err(ResourceError::InvalidFont(
                "base font data too short".into(),
            ));
        }

        let split_offset = u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize;
        let sfnt = data[4..].to_vec();

        let key = (id, weight);
        self.font_entries.insert(
            key,
            FontEntry {
                data: Arc::new(FontData::new(sfnt)),
                split_offset,
            },
        );
        self.font_versions.insert(key, 0);

        Ok(())
    }

    pub fn add_font_chunk(
        &mut self,
        id: u16,
        weight: u16,
        data: &[u8],
    ) -> Result<(), ResourceError> {
        let chunk_data = decode_tpft(data)?;
        if chunk_data.len() < 4 {
            return Err(ResourceError::InvalidFont("chunk data too short".into()));
        }

        let key = (id, weight);
        let entry = self
            .font_entries
            .get(&key)
            .ok_or_else(|| ResourceError::InvalidFont("no base font registered".into()))?;

        let num_entries = u32::from_be_bytes(chunk_data[0..4].try_into().unwrap()) as usize;

        // Safety: &mut self (or exclusive write lock) guarantees no concurrent readers.
        let sfnt = unsafe { entry.data.as_mut_slice() };
        let mut pos = 4;
        for _ in 0..num_entries {
            let offset = u32::from_be_bytes(chunk_data[pos..pos + 4].try_into().unwrap()) as usize;
            let len = u32::from_be_bytes(chunk_data[pos + 4..pos + 8].try_into().unwrap()) as usize;
            let src = &chunk_data[pos + 8..pos + 8 + len];

            let dst = entry.split_offset + offset;
            sfnt[dst..dst + len].copy_from_slice(src);

            pos += 8 + len;
        }

        *self.font_versions.entry(key).or_insert(0) += 1;

        Ok(())
    }

    pub fn font_version(&self, id: u16, weight: u16) -> u64 {
        self.font_versions.get(&(id, weight)).copied().unwrap_or(0)
    }

    pub fn font_data(&self, id: u16, weight: u16) -> Option<&[u8]> {
        self.font_entries
            .get(&(id, weight))
            .map(|e| e.data.as_ref().as_ref())
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl FontRegistry {
    pub fn from_families<S: Into<String>>(
        families: impl IntoIterator<Item = (S, Vec<u16>)>,
    ) -> Self {
        let mut reg = Self::new();
        reg.update(families.into_iter().map(|(k, v)| (k.into(), v)).collect());
        reg
    }
}

impl Default for FontRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::{TPFT_HEADER_SIZE, TPFT_MAGIC, TPFT_VERSION};
    use super::*;

    fn zstd_compress(data: &[u8]) -> Vec<u8> {
        ruzstd::encoding::compress_to_vec(data, ruzstd::encoding::CompressionLevel::Fastest)
    }

    fn make_tpft(payload: &[u8]) -> Vec<u8> {
        let compressed = zstd_compress(payload);
        let mut buf = Vec::with_capacity(TPFT_HEADER_SIZE + compressed.len());
        buf.extend_from_slice(TPFT_MAGIC);
        buf.extend_from_slice(&TPFT_VERSION.to_be_bytes());
        buf.extend_from_slice(&compressed);
        buf
    }

    /// Build a TPFT-encoded base font.
    /// Layout: [split_offset: u32][sfnt_bytes...]
    fn make_base_tpft(split_offset: u32, sfnt: &[u8]) -> Vec<u8> {
        let mut payload = Vec::with_capacity(4 + sfnt.len());
        payload.extend_from_slice(&split_offset.to_be_bytes());
        payload.extend_from_slice(sfnt);
        make_tpft(&payload)
    }

    /// Build a TPFT-encoded chunk.
    /// Layout: [num_entries: u32][offset: u32, len: u32, data...]...
    fn make_chunk_tpft(entries: &[(u32, &[u8])]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for &(offset, data) in entries {
            payload.extend_from_slice(&offset.to_be_bytes());
            payload.extend_from_slice(&(data.len() as u32).to_be_bytes());
            payload.extend_from_slice(data);
        }
        make_tpft(&payload)
    }

    fn make_registry() -> FontRegistry {
        let mut reg = FontRegistry::new();
        let mut families = HashMap::default();
        families.insert("Pretendard".into(), vec![100, 300, 400, 500, 700, 900]);
        families.insert("Mono".into(), vec![400, 700]);
        reg.update(families);
        reg
    }

    #[test]
    fn has_family() {
        let reg = make_registry();
        assert!(reg.has_family("Pretendard"));
        assert!(!reg.has_family("Unknown"));
    }

    #[test]
    fn weights() {
        let reg = make_registry();
        assert_eq!(
            reg.weights("Pretendard"),
            Some(&[100, 300, 400, 500, 700, 900][..])
        );
        assert_eq!(reg.weights("Unknown"), None);
    }

    #[test]
    fn has_weight() {
        let reg = make_registry();
        assert!(reg.has_weight("Pretendard", 400));
        assert!(!reg.has_weight("Pretendard", 200));
    }

    #[test]
    fn nearest_weight_exact() {
        let reg = make_registry();
        assert_eq!(reg.nearest_weight("Pretendard", 400), Some(400));
    }

    #[test]
    fn nearest_weight_between() {
        let reg = make_registry();
        assert_eq!(reg.nearest_weight("Pretendard", 600), Some(500));
    }

    #[test]
    fn nearest_weight_below_min() {
        let reg = make_registry();
        assert_eq!(reg.nearest_weight("Pretendard", 50), Some(100));
    }

    #[test]
    fn nearest_weight_above_max() {
        let reg = make_registry();
        assert_eq!(reg.nearest_weight("Pretendard", 950), Some(900));
    }

    #[test]
    fn nearest_weight_unknown_family() {
        let reg = make_registry();
        assert_eq!(reg.nearest_weight("Unknown", 400), None);
    }

    #[test]
    fn update_replaces_all() {
        let mut reg = make_registry();
        let mut families = HashMap::default();
        families.insert("NewFont".into(), vec![400]);
        reg.update(families);
        assert!(!reg.has_family("Pretendard"));
        assert!(reg.has_family("NewFont"));
    }

    #[test]
    fn update_deduplicates_and_sorts() {
        let mut reg = FontRegistry::new();
        let mut families = HashMap::default();
        families.insert("Test".into(), vec![700, 400, 700, 100, 400]);
        reg.update(families);
        assert_eq!(reg.weights("Test"), Some(&[100, 400, 700][..]));
    }

    #[test]
    fn intern_and_resolve() {
        let mut reg = FontRegistry::new();
        let id = reg.intern("Arial");
        assert_eq!(reg.resolve(id), "Arial");
        assert_eq!(reg.intern("Arial"), id); // same ID
    }

    #[test]
    fn codepoint_mapping() {
        let mut reg = FontRegistry::new();
        let arial_id = reg.intern("Arial");
        let noto_id = reg.intern("NotoSansCJK");
        reg.add_codepoint_mapping(arial_id, 400, '한' as u32, noto_id, 400);

        let map = reg.codepoint_map(arial_id, 400).unwrap();
        assert_eq!(map.get(&('한' as u32)), Some(&(noto_id, 400)));
        assert!(reg.codepoint_map(arial_id, 700).is_none());
    }

    #[test]
    fn add_font_base_stores_data() {
        let mut reg = FontRegistry::new();
        // 20 bytes of sfnt, split_offset=8
        let sfnt = vec![0u8; 20];
        let tpft = make_base_tpft(8, &sfnt);

        reg.add_font_base(0, 400, &tpft).unwrap();

        let data = reg.font_data(0, 400).unwrap();
        assert_eq!(data, &sfnt[..]);
        assert_eq!(reg.font_version(0, 400), 0);
    }

    #[test]
    fn add_font_base_different_weights() {
        let mut reg = FontRegistry::new();
        let sfnt_400 = vec![0u8; 20];
        let sfnt_700 = vec![1u8; 20];
        reg.add_font_base(0, 400, &make_base_tpft(8, &sfnt_400))
            .unwrap();
        reg.add_font_base(0, 700, &make_base_tpft(8, &sfnt_700))
            .unwrap();

        assert_eq!(reg.font_data(0, 400).unwrap(), &sfnt_400[..]);
        assert_eq!(reg.font_data(0, 700).unwrap(), &sfnt_700[..]);
    }

    #[test]
    fn add_font_chunk_patches_data() {
        let mut reg = FontRegistry::new();
        let sfnt = vec![0u8; 20];
        let tpft = make_base_tpft(8, &sfnt);
        reg.add_font_base(0, 400, &tpft).unwrap();

        // Patch 3 bytes at offset 4 (relative to split_offset)
        let chunk = make_chunk_tpft(&[(4, &[0xAA, 0xBB, 0xCC])]);
        reg.add_font_chunk(0, 400, &chunk).unwrap();

        let data = reg.font_data(0, 400).unwrap();
        // split_offset(8) + chunk_offset(4) = byte 12..15
        assert_eq!(&data[12..15], &[0xAA, 0xBB, 0xCC]);
        assert_eq!(reg.font_version(0, 400), 1);
    }

    #[test]
    fn add_font_chunk_increments_version() {
        let mut reg = FontRegistry::new();
        let sfnt = vec![0u8; 20];
        reg.add_font_base(0, 400, &make_base_tpft(8, &sfnt))
            .unwrap();

        reg.add_font_chunk(0, 400, &make_chunk_tpft(&[(0, &[1])]))
            .unwrap();
        assert_eq!(reg.font_version(0, 400), 1);

        reg.add_font_chunk(0, 400, &make_chunk_tpft(&[(1, &[2])]))
            .unwrap();
        assert_eq!(reg.font_version(0, 400), 2);
    }

    #[test]
    fn add_font_chunk_without_base_errors() {
        let mut reg = FontRegistry::new();
        let chunk = make_chunk_tpft(&[(0, &[1])]);
        assert!(reg.add_font_chunk(5, 400, &chunk).is_err());
    }
}
