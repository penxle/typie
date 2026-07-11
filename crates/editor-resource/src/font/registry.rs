use hashbrown::{HashMap, HashSet};
use smallvec::SmallVec;
use std::sync::Arc;

use super::config::{FontFamily, FontFamilySource};
use super::data::FontData;
use super::manifest::FontManifest;
use crate::error::ResourceError;

pub(super) struct FontEntry {
    pub(super) data: Arc<FontData>,
    pub(super) split_offset: usize,
}

pub struct FontRegistry {
    families: HashMap<String, SmallVec<[u16; 9]>>,
    family_names: Vec<String>,
    family_index: HashMap<String, u16>,
    family_source: HashMap<u16, FontFamilySource>,
    pub(super) font_entries: HashMap<(u16, u16), FontEntry>,
    pub(super) font_versions: HashMap<(u16, u16), u64>,
    pub(super) loaded_chunks: HashMap<(u16, u16), Vec<bool>>,
    manifests: HashMap<(u16, u16), FontManifest>,
    font_hashes: HashMap<(u16, u16), String>,
    placeholder_family_id: Option<u16>,
    font_generation: u64,
}

impl FontRegistry {
    pub fn new() -> Self {
        Self {
            families: HashMap::default(),
            family_names: Vec::new(),
            family_index: HashMap::default(),
            family_source: HashMap::default(),
            font_entries: HashMap::default(),
            font_versions: HashMap::default(),
            loaded_chunks: HashMap::default(),
            manifests: HashMap::default(),
            font_hashes: HashMap::default(),
            placeholder_family_id: None,
            font_generation: 0,
        }
    }

    pub fn font_generation(&self) -> u64 {
        self.font_generation
    }

    pub fn set_fonts(&mut self, families: Vec<FontFamily>) {
        self.font_generation += 1;

        // Phase 1: intern names and build the target (family_id, weight) -> hash map.
        let mut new_hashes: HashMap<(u16, u16), String> = HashMap::default();
        for family in &families {
            if family.name == super::placeholder::PLACEHOLDER_FAMILY_NAME {
                continue;
            }
            let family_id = self.intern(&family.name);
            for w in &family.weights {
                new_hashes.insert((family_id, w.value), w.hash.clone());
            }
        }

        // Phase 2: compute which keys survive. Placeholder weights always stay;
        // non-placeholder weights stay only if their hash is unchanged.
        let placeholder_id = self.placeholder_family_id;
        let surviving: HashSet<(u16, u16)> = self
            .font_hashes
            .iter()
            .filter(|(key, old_hash)| new_hashes.get(*key) == Some(*old_hash))
            .map(|(&key, _)| key)
            .chain(
                self.font_entries
                    .keys()
                    .filter(|key| Some(key.0) == placeholder_id)
                    .copied(),
            )
            .collect();

        // Phase 3: rebuild maps. Retain surviving byte-level state; drop the rest.
        self.font_entries.retain(|key, _| surviving.contains(key));
        self.font_versions.retain(|key, _| surviving.contains(key));
        self.loaded_chunks.retain(|key, _| surviving.contains(key));
        self.font_hashes.retain(|key, _| surviving.contains(key));

        self.families.clear();
        self.family_source.clear();
        self.manifests.retain(|key, _| surviving.contains(key));

        for family in families {
            if family.name == super::placeholder::PLACEHOLDER_FAMILY_NAME {
                continue;
            }

            let family_id = self.intern(&family.name);
            self.family_source.insert(family_id, family.source);

            let mut weights: Vec<u16> = Vec::with_capacity(family.weights.len());
            for w in &family.weights {
                weights.push(w.value);
                let key = (family_id, w.value);
                if !surviving.contains(&key) {
                    self.loaded_chunks.insert(key, Vec::new());
                    self.font_versions.insert(key, 0);
                }
                self.font_hashes.insert(key, w.hash.clone());
            }
            weights.sort_unstable();
            weights.dedup();
            self.families
                .insert(family.name, SmallVec::from_vec(weights));
        }
    }

    pub fn has_family(&self, family: &str) -> bool {
        self.families.contains_key(family)
    }

    pub fn has_family_ci(&self, name: &str) -> Option<&str> {
        self.families
            .keys()
            .find(|k| k.eq_ignore_ascii_case(name))
            .map(|s| s.as_str())
    }

    pub fn weights(&self, family: &str) -> Option<&[u16]> {
        self.families.get(family).map(|w| w.as_slice())
    }

    pub fn has_weight(&self, family: &str, weight: u16) -> bool {
        self.families
            .get(family)
            .is_some_and(|w| w.contains(&weight))
    }

    pub fn nearest_weight(&self, family: &str, target: u16) -> Option<u16> {
        let weights = self.families.get(family)?;
        super::weight::match_weight(weights, target)
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

    pub fn family_name(&self, id: u16) -> &str {
        &self.family_names[id as usize]
    }

    pub fn family_name_opt(&self, id: u16) -> Option<&str> {
        self.family_names.get(id as usize).map(|s| s.as_str())
    }

    pub fn family_source(&self, family_id: u16) -> Option<FontFamilySource> {
        self.family_source.get(&family_id).copied()
    }

    pub fn fallback_family_ids(&self) -> impl Iterator<Item = u16> + '_ {
        let mut ids: Vec<u16> = self
            .family_source
            .iter()
            .filter_map(|(&id, &src)| (src == FontFamilySource::Fallback).then_some(id))
            .collect();
        ids.sort();
        ids.into_iter()
    }

    pub fn manifest(&self, family_id: u16, weight: u16) -> Option<&FontManifest> {
        self.manifests.get(&(family_id, weight))
    }

    pub fn has_manifest(&self, family_id: u16, weight: u16) -> bool {
        self.manifests.contains_key(&(family_id, weight))
    }

    pub fn set_manifest(&mut self, family_id: u16, weight: u16, manifest: FontManifest) {
        let key = (family_id, weight);
        let unchanged = self.manifests.get(&key) == Some(&manifest);
        if !unchanged {
            self.loaded_chunks
                .insert(key, vec![false; manifest.chunk_count as usize]);
        }
        self.manifests.insert(key, manifest);
    }

    pub fn chunk_id_for_codepoint(&self, family_id: u16, weight: u16, cp: u32) -> Option<u16> {
        self.manifests.get(&(family_id, weight))?.chunk_id(cp)
    }

    pub fn is_base_loaded(&self, family_id: u16, weight: u16) -> bool {
        self.font_entries.contains_key(&(family_id, weight))
    }

    pub fn is_chunk_loaded(&self, family_id: u16, weight: u16, chunk_id: u16) -> bool {
        self.loaded_chunks
            .get(&(family_id, weight))
            .and_then(|bv| bv.get(chunk_id as usize).copied())
            .unwrap_or(false)
    }

    /// Insert already-decompressed, already-parsed base font bytes. Callers
    /// (`Resource::insert_font_base`) are responsible for the zstd decompression
    /// and TTF split-offset parsing — both moved out of the registry so they can
    /// run outside the `Resource` mutex.
    pub fn insert_base(
        &mut self,
        family_id: u16,
        weight: u16,
        data: Arc<FontData>,
        split_offset: usize,
    ) {
        let key = (family_id, weight);
        self.font_entries
            .insert(key, FontEntry { data, split_offset });
        self.font_versions.insert(key, 0);
        if let Some(manifest) = self.manifests.get(&key) {
            self.loaded_chunks
                .insert(key, vec![false; manifest.chunk_count as usize]);
        } else {
            self.loaded_chunks.insert(key, Vec::new());
        }

        self.font_generation += 1;
    }

    /// Validates every chunk entry's bounds up front (pass 1) before mutating
    /// any SFNT bytes (pass 2), so a corrupt payload can't panic or leave a
    /// partially-patched font.
    pub fn add_font_chunk(
        &mut self,
        family_id: u16,
        weight: u16,
        chunk_id: u16,
        payload: &[u8],
    ) -> Result<(), ResourceError> {
        if payload.len() < 4 {
            return Err(ResourceError::InvalidFont("chunk data too short".into()));
        }

        let key = (family_id, weight);
        if let Some(bv) = self.loaded_chunks.get(&key)
            && !bv.is_empty()
            && (chunk_id as usize) >= bv.len()
        {
            return Err(ResourceError::InvalidFont("chunk id out of range".into()));
        }

        let entry = self
            .font_entries
            .get(&key)
            .ok_or_else(|| ResourceError::InvalidFont("no base font registered".into()))?;

        let num_entries = u32::from_be_bytes(payload[0..4].try_into().unwrap()) as usize;
        if num_entries > payload.len().saturating_sub(4) / 8 {
            return Err(ResourceError::InvalidFont(
                "chunk entry count implausible".into(),
            ));
        }
        let sfnt_len = entry.data.as_ref().as_ref().len();

        // pass 1: validate every entry's bounds against the SFNT length without touching it.
        let mut spans: Vec<(usize, usize, usize)> = Vec::with_capacity(num_entries);
        let mut pos = 4usize;
        for _ in 0..num_entries {
            let header_end = pos
                .checked_add(8)
                .filter(|&e| e <= payload.len())
                .ok_or_else(|| ResourceError::InvalidFont("chunk entry header truncated".into()))?;
            let offset = u32::from_be_bytes(payload[pos..pos + 4].try_into().unwrap()) as usize;
            let len = u32::from_be_bytes(payload[pos + 4..header_end].try_into().unwrap()) as usize;
            let src_start = header_end;
            let src_end = src_start
                .checked_add(len)
                .filter(|&e| e <= payload.len())
                .ok_or_else(|| ResourceError::InvalidFont("chunk entry body truncated".into()))?;
            let dst = entry
                .split_offset
                .checked_add(offset)
                .ok_or_else(|| ResourceError::InvalidFont("chunk entry out of bounds".into()))?;
            dst.checked_add(len)
                .filter(|&e| e <= sfnt_len)
                .ok_or_else(|| ResourceError::InvalidFont("chunk entry out of bounds".into()))?;
            spans.push((dst, src_start, len));
            pos = src_end;
        }
        if pos != payload.len() {
            return Err(ResourceError::InvalidFont("chunk trailing bytes".into()));
        }

        // pass 2: apply only after every entry has been validated.
        // Safety: &mut self guarantees no concurrent readers of FontData.
        let sfnt = unsafe { &mut *entry.data.as_mut_ptr() };
        for (dst, src_start, len) in spans {
            sfnt[dst..dst + len].copy_from_slice(&payload[src_start..src_start + len]);
        }

        *self.font_versions.entry(key).or_insert(0) += 1;

        if let Some(bv) = self.loaded_chunks.get_mut(&key)
            && (chunk_id as usize) < bv.len()
        {
            bv[chunk_id as usize] = true;
        }

        self.font_generation += 1;

        Ok(())
    }

    pub fn font_version(&self, family_id: u16, weight: u16) -> u64 {
        self.font_versions
            .get(&(family_id, weight))
            .copied()
            .unwrap_or(0)
    }

    pub fn font_data(&self, family_id: u16, weight: u16) -> Option<&[u8]> {
        self.font_entries
            .get(&(family_id, weight))
            .map(|e| e.data.as_ref().as_ref())
    }

    pub fn register_placeholder(&mut self, data: &[u8]) {
        let id = self.intern(super::placeholder::PLACEHOLDER_FAMILY_NAME);
        let buffer: Vec<u8> = data.to_vec();
        self.font_entries.insert(
            (id, super::placeholder::PLACEHOLDER_WEIGHT),
            FontEntry {
                data: Arc::new(FontData::new(buffer)),
                split_offset: 0,
            },
        );
        self.font_versions
            .insert((id, super::placeholder::PLACEHOLDER_WEIGHT), 0);
        self.placeholder_family_id = Some(id);
        self.font_generation += 1;
    }

    pub fn placeholder_family_id(&self) -> Option<u16> {
        self.placeholder_family_id
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl FontRegistry {
    pub fn force_loaded_for_test(&mut self, family_id: u16, weight: u16, chunk_count: u16) {
        let key = (family_id, weight);
        self.font_entries.insert(
            key,
            FontEntry {
                data: Arc::new(FontData::new(vec![0u8; 16])),
                split_offset: 0,
            },
        );
        self.font_versions.insert(key, 0);
        self.loaded_chunks
            .insert(key, vec![true; chunk_count as usize]);
        self.font_generation += 1;
    }
}

impl Default for FontRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::config::FontWeight;

    fn make_chunk_data(entries: &[(u32, &[u8])]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for &(offset, data) in entries {
            payload.extend_from_slice(&offset.to_be_bytes());
            payload.extend_from_slice(&(data.len() as u32).to_be_bytes());
            payload.extend_from_slice(data);
        }
        payload
    }

    /// Bypass skrifa TTF parsing by injecting a `FontEntry` directly.
    /// Tests target `add_font_chunk` behaviour only; `add_font_base`'s TTF
    /// parsing is covered by integration runs with real fonts.
    fn inject_base(
        reg: &mut FontRegistry,
        family_id: u16,
        weight: u16,
        buffer: Vec<u8>,
        split_offset: usize,
    ) {
        let key = (family_id, weight);
        reg.font_entries.insert(
            key,
            FontEntry {
                data: Arc::new(FontData::new(buffer)),
                split_offset,
            },
        );
        reg.font_versions.insert(key, 0);
        if let Some(manifest) = reg.manifests.get(&key) {
            reg.loaded_chunks
                .insert(key, vec![false; manifest.chunk_count as usize]);
        } else {
            reg.loaded_chunks.insert(key, Vec::new());
        }
    }

    fn make_registry_with_family(name: &str, weights: &[u16]) -> FontRegistry {
        let mut reg = FontRegistry::new();
        let families = vec![FontFamily {
            name: name.into(),
            source: FontFamilySource::Default,
            weights: weights
                .iter()
                .map(|&w| FontWeight {
                    value: w,
                    hash: format!("h{w}"),
                })
                .collect(),
        }];
        reg.set_fonts(families);
        let fid = reg.intern_id(name).unwrap();
        for &w in weights {
            reg.set_manifest(fid, w, FontManifest::from_coverages(&[vec![0x41, 0x41]]));
        }
        reg
    }

    #[test]
    fn has_family() {
        let reg = make_registry_with_family("Pretendard", &[400, 700]);
        assert!(reg.has_family("Pretendard"));
        assert!(!reg.has_family("Unknown"));
    }

    #[test]
    fn has_family_ci_returns_registered_name() {
        let reg = make_registry_with_family("Pretendard", &[400]);
        assert_eq!(reg.has_family_ci("Pretendard"), Some("Pretendard"));
        assert_eq!(reg.has_family_ci("pretendard"), Some("Pretendard"));
        assert_eq!(reg.has_family_ci("PRETENDARD"), Some("Pretendard"));
        assert_eq!(reg.has_family_ci("PreTenDard"), Some("Pretendard"));
        assert_eq!(reg.has_family_ci("Unknown"), None);
    }

    #[test]
    fn weights() {
        let reg = make_registry_with_family("Pretendard", &[700, 400, 100]);
        assert_eq!(reg.weights("Pretendard"), Some(&[100, 400, 700][..]));
        assert_eq!(reg.weights("Unknown"), None);
    }

    #[test]
    fn has_weight() {
        let reg = make_registry_with_family("Pretendard", &[400, 700]);
        assert!(reg.has_weight("Pretendard", 400));
        assert!(!reg.has_weight("Pretendard", 200));
    }

    #[test]
    fn nearest_weight_between() {
        let reg = make_registry_with_family("Pretendard", &[400, 700]);
        assert_eq!(reg.nearest_weight("Pretendard", 600), Some(700));
    }

    #[test]
    fn intern_and_resolve() {
        let mut reg = FontRegistry::new();
        let id = reg.intern("Arial");
        assert_eq!(reg.family_name(id), "Arial");
        assert_eq!(reg.intern("Arial"), id);
    }

    #[test]
    fn set_fonts_records_source() {
        let mut reg = FontRegistry::new();
        reg.set_fonts(vec![FontFamily {
            name: "F".into(),
            source: FontFamilySource::Fallback,
            weights: vec![FontWeight {
                value: 400,
                hash: "h".into(),
            }],
        }]);
        let fid = reg.intern_id("F").unwrap();
        assert_eq!(reg.family_source(fid), Some(FontFamilySource::Fallback));
    }

    #[test]
    fn font_version_initial_is_zero() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);
        assert_eq!(reg.font_version(fid, 400), 0);
    }

    #[test]
    fn add_font_chunk_patches_data() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);

        let chunk = make_chunk_data(&[(4, &[0xAA, 0xBB, 0xCC])]);
        reg.add_font_chunk(fid, 400, 0, &chunk).unwrap();

        let data = reg.font_data(fid, 400).unwrap();
        // split_offset(8) + chunk_offset(4) = byte 12..15
        assert_eq!(&data[12..15], &[0xAA, 0xBB, 0xCC]);
        assert_eq!(reg.font_version(fid, 400), 1);
    }

    #[test]
    fn add_font_chunk_increments_version() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);

        reg.add_font_chunk(fid, 400, 0, &make_chunk_data(&[(0, &[1])]))
            .unwrap();
        assert_eq!(reg.font_version(fid, 400), 1);

        reg.add_font_chunk(fid, 400, 0, &make_chunk_data(&[(1, &[2])]))
            .unwrap();
        assert_eq!(reg.font_version(fid, 400), 2);
    }

    #[test]
    fn add_font_chunk_without_base_errors() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        let chunk = make_chunk_data(&[(0, &[1])]);
        assert!(matches!(
            reg.add_font_chunk(fid, 400, 0, &chunk),
            Err(ResourceError::InvalidFont(_))
        ));
    }

    #[test]
    fn add_font_chunk_marks_loaded_chunk() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);

        assert!(!reg.is_chunk_loaded(fid, 400, 0));
        reg.add_font_chunk(fid, 400, 0, &make_chunk_data(&[(0, &[1])]))
            .unwrap();
        assert!(reg.is_chunk_loaded(fid, 400, 0));
    }

    #[test]
    fn is_chunk_loaded_returns_false_before_load() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);
        assert!(!reg.is_chunk_loaded(fid, 400, 0));
    }

    #[test]
    fn is_base_loaded_tracks_font_entry() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        assert!(!reg.is_base_loaded(fid, 400));
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);
        assert!(reg.is_base_loaded(fid, 400));
    }

    use super::super::placeholder::{PLACEHOLDER_FAMILY_NAME, PLACEHOLDER_WEIGHT};

    #[test]
    fn placeholder_unset_initially() {
        let reg = FontRegistry::new();
        assert!(reg.placeholder_family_id().is_none());
    }

    #[test]
    fn register_placeholder_sets_id_and_stores_bytes() {
        let mut reg = FontRegistry::new();
        let bytes = vec![0u8; 16]; // dummy payload; real registration validated in later tasks
        reg.register_placeholder(&bytes);

        let id = reg.placeholder_family_id().expect("placeholder id set");
        assert_eq!(reg.family_name_opt(id), Some(PLACEHOLDER_FAMILY_NAME));
        assert_eq!(
            reg.font_data(id, PLACEHOLDER_WEIGHT).map(|s| s.len()),
            Some(16)
        );
    }

    #[test]
    fn set_fonts_preserves_placeholder_entry() {
        let mut reg = FontRegistry::new();
        reg.register_placeholder(&[0u8; 8]);
        let placeholder_id = reg.placeholder_family_id().unwrap();

        reg.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "h".into(),
            }],
        }]);

        assert!(reg.font_data(placeholder_id, PLACEHOLDER_WEIGHT).is_some());
        assert_eq!(reg.placeholder_family_id(), Some(placeholder_id));
    }

    #[test]
    fn set_fonts_rejects_reserved_name() {
        let mut reg = FontRegistry::new();
        reg.register_placeholder(&[0u8; 8]);

        reg.set_fonts(vec![FontFamily {
            name: PLACEHOLDER_FAMILY_NAME.into(),
            source: FontFamilySource::User,
            weights: vec![FontWeight {
                value: 400,
                hash: "h".into(),
            }],
        }]);

        assert!(!reg.has_family(PLACEHOLDER_FAMILY_NAME));
    }

    #[test]
    fn set_fonts_preserves_bytes_when_hash_unchanged() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);
        reg.add_font_chunk(fid, 400, 0, &make_chunk_data(&[(0, &[1])]))
            .unwrap();

        // Re-register with the SAME hash ("h400"); manifest set by make_registry_with_family.
        reg.set_fonts(vec![FontFamily {
            name: "T".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "h400".into(),
            }],
        }]);

        assert!(
            reg.is_base_loaded(fid, 400),
            "base bytes must survive set_fonts when hash is unchanged"
        );
        assert!(
            reg.is_chunk_loaded(fid, 400, 0),
            "chunk bitmap must survive set_fonts when hash is unchanged"
        );
    }

    #[test]
    fn set_fonts_drops_bytes_when_hash_changes() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);
        reg.add_font_chunk(fid, 400, 0, &make_chunk_data(&[(0, &[1])]))
            .unwrap();

        // Re-register with a DIFFERENT hash.
        reg.set_fonts(vec![FontFamily {
            name: "T".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "h400-v2".into(),
            }],
        }]);

        assert!(
            !reg.is_base_loaded(fid, 400),
            "base bytes must be dropped when hash changes"
        );
        assert!(
            !reg.is_chunk_loaded(fid, 400, 0),
            "chunk bitmap must be dropped when hash changes"
        );
    }

    #[test]
    fn set_fonts_drops_bytes_when_weight_removed() {
        let mut reg = FontRegistry::new();
        reg.set_fonts(vec![FontFamily {
            name: "T".into(),
            source: FontFamilySource::Default,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h400".into(),
                },
                FontWeight {
                    value: 700,
                    hash: "h700".into(),
                },
            ],
        }]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);
        inject_base(&mut reg, fid, 700, vec![0u8; 20], 8);

        // Re-register without weight 700.
        reg.set_fonts(vec![FontFamily {
            name: "T".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "h400".into(),
            }],
        }]);

        assert!(
            reg.is_base_loaded(fid, 400),
            "retained weight keeps its bytes"
        );
        assert!(
            !reg.is_base_loaded(fid, 700),
            "removed weight drops its bytes"
        );
    }

    #[test]
    fn set_fonts_leaves_manifest_absent_until_set_manifest() {
        let mut reg = FontRegistry::new();
        reg.set_fonts(vec![FontFamily {
            name: "T".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "h".into(),
            }],
        }]);
        let fid = reg.intern_id("T").unwrap();
        assert!(!reg.has_manifest(fid, 400));
        assert_eq!(reg.chunk_id_for_codepoint(fid, 400, 0x41), None);

        reg.set_manifest(fid, 400, FontManifest::from_coverages(&[vec![0x41, 0x41]]));
        assert!(reg.has_manifest(fid, 400));
        assert_eq!(reg.chunk_id_for_codepoint(fid, 400, 0x41), Some(0));
    }

    #[test]
    fn set_manifest_sizes_loaded_chunks() {
        let mut reg = FontRegistry::new();
        reg.set_fonts(vec![FontFamily {
            name: "T".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "h".into(),
            }],
        }]);
        let fid = reg.intern_id("T").unwrap();
        reg.set_manifest(
            fid,
            400,
            FontManifest::from_coverages(&[vec![0x41, 0x41], vec![0x42, 0x42]]),
        );
        assert!(!reg.is_chunk_loaded(fid, 400, 0));
        assert!(!reg.is_chunk_loaded(fid, 400, 1));
    }

    #[test]
    fn set_fonts_preserves_manifest_when_hash_unchanged() {
        let mut reg = FontRegistry::new();
        let family = |hash: &str| FontFamily {
            name: "T".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: hash.into(),
            }],
        };
        reg.set_fonts(vec![family("h1")]);
        let fid = reg.intern_id("T").unwrap();
        reg.set_manifest(fid, 400, FontManifest::from_coverages(&[vec![0x41, 0x41]]));

        reg.set_fonts(vec![family("h1")]);
        assert!(reg.has_manifest(fid, 400), "same hash keeps manifest");

        reg.set_fonts(vec![family("h2")]);
        assert!(!reg.has_manifest(fid, 400), "hash change drops manifest");
    }

    #[test]
    fn set_manifest_keeps_bitmap_only_for_equal_manifest() {
        let mut reg = FontRegistry::new();
        reg.set_fonts(vec![FontFamily {
            name: "T".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "h".into(),
            }],
        }]);
        let fid = reg.intern_id("T").unwrap();
        let manifest = FontManifest::from_coverages(&[vec![0x41, 0x41]]);
        reg.set_manifest(fid, 400, manifest.clone());
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);
        reg.add_font_chunk(fid, 400, 0, &make_chunk_data(&[(0, &[1])]))
            .unwrap();
        assert!(reg.is_chunk_loaded(fid, 400, 0));

        reg.set_manifest(fid, 400, manifest);
        assert!(
            reg.is_chunk_loaded(fid, 400, 0),
            "동등 manifest 재주입은 bitmap 유지"
        );

        // 같은 chunk 수, 다른 매핑 → bitmap 리셋
        reg.set_manifest(fid, 400, FontManifest::from_coverages(&[vec![0x42, 0x42]]));
        assert!(!reg.is_chunk_loaded(fid, 400, 0), "다른 매핑은 bitmap 리셋");
    }

    #[test]
    fn add_font_chunk_rejects_corrupt_payload_without_panic() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        reg.set_manifest(fid, 400, FontManifest::from_coverages(&[vec![0x41, 0x41]]));
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);

        // 엔트리 수만 선언하고 본문이 없는 payload
        let truncated = 1u32.to_be_bytes();
        assert!(reg.add_font_chunk(fid, 400, 0, &truncated).is_err());

        // 목적지 범위를 벗어나는 offset
        let oob = make_chunk_data(&[(1000, &[0xAA])]);
        assert!(reg.add_font_chunk(fid, 400, 0, &oob).is_err());
    }

    #[test]
    fn add_font_chunk_second_entry_failure_leaves_first_entry_untouched() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);

        let chunk = make_chunk_data(&[(0, &[0xAA, 0xBB, 0xCC]), (1000, &[0xFF])]);
        assert!(reg.add_font_chunk(fid, 400, 0, &chunk).is_err());

        let data = reg.font_data(fid, 400).unwrap();
        assert_eq!(
            &data[8..11],
            &[0, 0, 0],
            "first entry must stay unmodified when a later entry is invalid"
        );
    }

    #[test]
    fn add_font_chunk_rejects_trailing_bytes() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);

        let mut raw = Vec::new();
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.push(0xAA);
        raw.push(0x00); // trailing byte past the declared entry

        assert!(reg.add_font_chunk(fid, 400, 0, &raw).is_err());
    }

    #[test]
    fn add_font_chunk_rejects_chunk_id_out_of_range() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);

        let chunk = make_chunk_data(&[(0, &[1])]);
        assert!(matches!(
            reg.add_font_chunk(fid, 400, 5, &chunk),
            Err(ResourceError::InvalidFont(_))
        ));
    }

    #[test]
    fn add_font_chunk_rejects_implausible_num_entries_without_large_allocation() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);

        let bomb = u32::MAX.to_be_bytes();
        assert!(matches!(
            reg.add_font_chunk(fid, 400, 0, &bomb),
            Err(ResourceError::InvalidFont(_))
        ));
    }

    #[test]
    fn set_manifest_does_not_bump_generation() {
        let mut reg = FontRegistry::new();
        reg.set_fonts(vec![FontFamily {
            name: "T".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "h".into(),
            }],
        }]);
        let fid = reg.intern_id("T").unwrap();
        let before = reg.font_generation();

        reg.set_manifest(fid, 400, FontManifest::from_coverages(&[vec![0x41, 0x41]]));

        assert_eq!(reg.font_generation(), before);
    }

    #[test]
    fn insert_base_bumps_generation() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        let before = reg.font_generation();

        reg.insert_base(fid, 400, Arc::new(FontData::new(vec![0u8; 20])), 8);

        assert!(reg.font_generation() > before);
    }

    #[test]
    fn add_font_chunk_bumps_generation() {
        let mut reg = make_registry_with_family("T", &[400]);
        let fid = reg.intern_id("T").unwrap();
        inject_base(&mut reg, fid, 400, vec![0u8; 20], 8);
        let before = reg.font_generation();

        reg.add_font_chunk(fid, 400, 0, &make_chunk_data(&[(0, &[1])]))
            .unwrap();

        assert!(reg.font_generation() > before);
    }
}
