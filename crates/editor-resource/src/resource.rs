use hashbrown::HashMap;
use parley::{FontContext, LayoutContext};
use skrifa::Tag;
use skrifa::raw::FontRef;
use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

use icu_properties::CodePointMapData;
use icu_properties::props::GeneralCategory;

use crate::brush::TextBrush;
use crate::error::ResourceError;
use crate::font::{FontData, FontFamily, FontManifest, FontRegistry};
use crate::segmentation::{IcuResources, TextSegmenters};
use crate::text_replacement::{PreparedTextReplacementRules, TextReplacementRule};
use crate::theme::Theme;
use crate::theme_data::ThemeVariant;
use crate::zstd::decompress_zstd_capped;

const PLACEHOLDER_TTF: &[u8] = include_bytes!("../assets/placeholder.ttf");
const BASE_MAX_BYTES: usize = 64 * 1024 * 1024;

/// Output of [`prepare_font_base`] — the expensive TTF parsing done up front
/// so `ResourceSource::insert_font_base` only needs a short, infallible apply
/// step.
pub struct PreparedFontBase {
    font_data: Arc<FontData>,
    base_hash: u64,
    split_offset: usize,
}

/// Decompress and parse a base font ahead of taking the `ResourceSource` lock.
pub fn prepare_font_base(data: &[u8]) -> Result<PreparedFontBase, ResourceError> {
    let raw_ttf = decompress_zstd_capped(data, BASE_MAX_BYTES)?;

    let font = FontRef::new(&raw_ttf)
        .map_err(|e| ResourceError::InvalidFont(format!("failed to parse TTF: {e:?}")))?;

    let glyf_tag = Tag::new(b"glyf");
    let cbdt_tag = Tag::new(b"CBDT");
    let record = font
        .table_directory()
        .table_records()
        .iter()
        .find(|r| r.tag() == cbdt_tag)
        .or_else(|| {
            font.table_directory()
                .table_records()
                .iter()
                .find(|r| r.tag() == glyf_tag)
        })
        .ok_or_else(|| ResourceError::InvalidFont("glyf/CBDT table missing".into()))?;

    let split_offset = record.offset() as usize;
    let base_hash = rapidhash::v3::rapidhash_v3(&raw_ttf);
    let font_data = Arc::new(FontData::new(raw_ttf));

    Ok(PreparedFontBase {
        font_data,
        base_hash,
        split_offset,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceRevision(u64);

impl ResourceRevision {
    pub const INITIAL: Self = Self(0);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    fn next(self) -> Self {
        Self(self.0.checked_add(1).expect("resource revision overflow"))
    }
}

pub struct ResourceSnapshot {
    revision: ResourceRevision,
    theme: Arc<Theme>,
    fonts: Arc<FontRegistry>,
    segmenters: Arc<TextSegmenters>,
    general_category: Arc<CodePointMapData<GeneralCategory>>,
    text_replacement_rules: Arc<[TextReplacementRule]>,
    auto_surround_enabled: bool,
}

impl ResourceSnapshot {
    fn new(icu: IcuResources) -> Self {
        let mut fonts = FontRegistry::new();
        fonts.register_placeholder(PLACEHOLDER_TTF);
        Self {
            revision: ResourceRevision::INITIAL,
            theme: Arc::new(Theme::new(ThemeVariant::LightWhite)),
            fonts: Arc::new(fonts),
            segmenters: icu.segmenters,
            general_category: icu.general_category,
            text_replacement_rules: Arc::from([]),
            auto_surround_enabled: true,
        }
    }

    pub fn revision(&self) -> ResourceRevision {
        self.revision
    }

    pub fn theme(&self) -> &Arc<Theme> {
        &self.theme
    }

    pub fn fonts(&self) -> &Arc<FontRegistry> {
        &self.fonts
    }

    pub fn segmenters(&self) -> &Arc<TextSegmenters> {
        &self.segmenters
    }

    pub fn general_category(&self) -> &Arc<CodePointMapData<GeneralCategory>> {
        &self.general_category
    }

    pub fn text_replacement_rules(&self) -> &Arc<[TextReplacementRule]> {
        &self.text_replacement_rules
    }

    pub fn auto_surround_enabled(&self) -> bool {
        self.auto_surround_enabled
    }

    fn with_revision_from(&self) -> Self {
        Self {
            revision: self.revision.next(),
            theme: Arc::clone(&self.theme),
            fonts: Arc::clone(&self.fonts),
            segmenters: Arc::clone(&self.segmenters),
            general_category: Arc::clone(&self.general_category),
            text_replacement_rules: Arc::clone(&self.text_replacement_rules),
            auto_surround_enabled: self.auto_surround_enabled,
        }
    }
}

pub struct ResourceSource {
    current: Arc<ResourceSnapshot>,
}

impl ResourceSource {
    pub fn new(icu: IcuResources) -> Self {
        Self {
            current: Arc::new(ResourceSnapshot::new(icu)),
        }
    }

    pub fn snapshot(&self) -> Arc<ResourceSnapshot> {
        Arc::clone(&self.current)
    }

    pub fn revision(&self) -> ResourceRevision {
        self.current.revision()
    }

    fn commit(&mut self, snapshot: ResourceSnapshot) -> Arc<ResourceSnapshot> {
        let snapshot = Arc::new(snapshot);
        self.current = Arc::clone(&snapshot);
        snapshot
    }

    pub fn set_theme_variant(&mut self, variant: ThemeVariant) -> Option<Arc<ResourceSnapshot>> {
        if self.current.theme.variant() == variant {
            return None;
        }
        let mut next = self.current.with_revision_from();
        next.theme = Arc::new(Theme::new(variant));
        Some(self.commit(next))
    }

    pub fn set_text_replacement_rules(
        &mut self,
        prepared: PreparedTextReplacementRules,
    ) -> Option<Arc<ResourceSnapshot>> {
        if self.current.text_replacement_rules.as_ref() == prepared.rules.as_ref() {
            return None;
        }
        let mut next = self.current.with_revision_from();
        next.text_replacement_rules = prepared.rules;
        Some(self.commit(next))
    }

    pub fn set_auto_surround_enabled(&mut self, enabled: bool) -> Option<Arc<ResourceSnapshot>> {
        if self.current.auto_surround_enabled == enabled {
            return None;
        }
        let mut next = self.current.with_revision_from();
        next.auto_surround_enabled = enabled;
        Some(self.commit(next))
    }

    pub fn set_fonts(&mut self, prepared: PreparedFonts) -> Option<Arc<ResourceSnapshot>> {
        let mut fonts = self.current.fonts.as_ref().clone();
        if !fonts.set_fonts(prepared.families) {
            return None;
        }
        let mut next = self.current.with_revision_from();
        next.fonts = Arc::new(fonts);
        Some(self.commit(next))
    }

    pub fn insert_font_base(
        &mut self,
        family: &str,
        weight: u16,
        prepared: PreparedFontBase,
    ) -> Option<Arc<ResourceSnapshot>> {
        if self.current.fonts.font_base_matches(
            family,
            weight,
            prepared.base_hash,
            prepared.split_offset,
        ) {
            return None;
        }
        let mut fonts = self.current.fonts.as_ref().clone();
        let family_id = fonts.intern(family);
        fonts.insert_base(
            family_id,
            weight,
            prepared.font_data,
            prepared.base_hash,
            prepared.split_offset,
        );
        let mut next = self.current.with_revision_from();
        next.fonts = Arc::new(fonts);
        Some(self.commit(next))
    }

    pub fn add_font_chunk(
        &mut self,
        family: &str,
        weight: u16,
        chunk_id: u16,
        prepared: PreparedFontChunk,
    ) -> Result<Option<Arc<ResourceSnapshot>>, ResourceError> {
        let mut fonts = self.current.fonts.as_ref().clone();
        let family_id = fonts
            .intern_id(family)
            .ok_or_else(|| ResourceError::UnknownFont(family.into()))?;
        if !fonts.add_font_chunk(family_id, weight, chunk_id, &prepared)? {
            return Ok(None);
        }
        let mut next = self.current.with_revision_from();
        next.fonts = Arc::new(fonts);
        Ok(Some(self.commit(next)))
    }

    pub fn add_font_manifest(
        &mut self,
        family: &str,
        weight: u16,
        manifest: FontManifest,
    ) -> Option<Arc<ResourceSnapshot>> {
        let mut fonts = self.current.fonts.as_ref().clone();
        let family_id = fonts.intern(family);
        if !fonts.set_manifest(family_id, weight, manifest) {
            return None;
        }
        let mut next = self.current.with_revision_from();
        next.fonts = Arc::new(fonts);
        Some(self.commit(next))
    }
}

pub struct PreparedFonts {
    families: Vec<FontFamily>,
}

pub fn prepare_fonts(families: Vec<FontFamily>) -> PreparedFonts {
    struct CanonicalFamily {
        source: crate::font::FontFamilySource,
        weights: BTreeMap<u16, String>,
    }

    let mut order = Vec::new();
    let mut canonical = HashMap::<String, CanonicalFamily>::new();
    for family in families {
        if family.name == crate::font::PLACEHOLDER_FAMILY_NAME {
            continue;
        }
        let entry = canonical.entry(family.name.clone()).or_insert_with(|| {
            order.push(family.name.clone());
            CanonicalFamily {
                source: family.source,
                weights: BTreeMap::new(),
            }
        });
        entry.source = family.source;
        for weight in family.weights {
            entry.weights.insert(weight.value, weight.hash);
        }
    }

    let families = order
        .into_iter()
        .map(|name| {
            let family = canonical
                .remove(&name)
                .expect("ordered canonical family exists");
            FontFamily {
                name,
                source: family.source,
                weights: family
                    .weights
                    .into_iter()
                    .map(|(value, hash)| crate::font::FontWeight { value, hash })
                    .collect(),
            }
        })
        .collect();
    PreparedFonts { families }
}

pub struct PreparedFontChunk {
    pub(crate) payload: Arc<[u8]>,
    pub(crate) patches: Box<[PreparedFontPatch]>,
}

pub(crate) struct PreparedFontPatch {
    pub(crate) offset: usize,
    pub(crate) bytes: Range<usize>,
}

pub fn prepare_font_chunk(payload: Vec<u8>) -> Result<PreparedFontChunk, ResourceError> {
    if payload.len() < 4 {
        return Err(ResourceError::InvalidFont("chunk data too short".into()));
    }
    let num_entries = u32::from_be_bytes(payload[0..4].try_into().unwrap()) as usize;
    if num_entries > payload.len().saturating_sub(4) / 8 {
        return Err(ResourceError::InvalidFont(
            "chunk entry count implausible".into(),
        ));
    }
    let mut patches = Vec::with_capacity(num_entries);
    let mut destination_ranges = Vec::with_capacity(num_entries);
    let mut pos = 4usize;
    for _ in 0..num_entries {
        let header_end = pos
            .checked_add(8)
            .filter(|&end| end <= payload.len())
            .ok_or_else(|| ResourceError::InvalidFont("chunk entry header truncated".into()))?;
        let len = u32::from_be_bytes(payload[pos + 4..header_end].try_into().unwrap()) as usize;
        let body_end = header_end
            .checked_add(len)
            .filter(|&end| end <= payload.len())
            .ok_or_else(|| ResourceError::InvalidFont("chunk entry body truncated".into()))?;
        let offset = u32::from_be_bytes(payload[pos..pos + 4].try_into().unwrap()) as usize;
        let destination_end = offset
            .checked_add(len)
            .ok_or_else(|| ResourceError::InvalidFont("chunk entry range overflow".into()))?;
        patches.push(PreparedFontPatch {
            offset,
            bytes: header_end..body_end,
        });
        if offset != destination_end {
            destination_ranges.push(offset..destination_end);
        }
        pos = body_end;
    }
    if pos != payload.len() {
        return Err(ResourceError::InvalidFont("chunk trailing bytes".into()));
    }
    destination_ranges.sort_unstable_by_key(|range| range.start);
    if destination_ranges
        .windows(2)
        .any(|ranges| ranges[1].start < ranges[0].end)
    {
        return Err(ResourceError::InvalidFont("chunk entries overlap".into()));
    }
    Ok(PreparedFontChunk {
        payload: payload.into(),
        patches: patches.into_boxed_slice(),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ResourceApplyError {
    #[error("stale resource snapshot: current {current:?}, incoming {incoming:?}")]
    Stale {
        current: ResourceRevision,
        incoming: ResourceRevision,
    },
    #[error("different resource snapshot for revision {0:?}")]
    ConflictingRevision(ResourceRevision),
}

pub struct Resource {
    snapshot: Arc<ResourceSnapshot>,
    pub font_registry: FontRegistry,
    pub font_context: FontContext,
    pub layout_context: LayoutContext<TextBrush>,
}

impl Resource {
    pub fn from_snapshot(snapshot: Arc<ResourceSnapshot>) -> Self {
        let mut font_context = FontContext::new();
        snapshot.fonts.register_with_font_context(&mut font_context);
        Self {
            font_registry: snapshot.fonts.as_ref().clone(),
            snapshot: Arc::clone(&snapshot),
            font_context,
            layout_context: LayoutContext::new(),
        }
    }

    pub fn snapshot(&self) -> &Arc<ResourceSnapshot> {
        &self.snapshot
    }

    pub fn theme(&self) -> &Theme {
        self.snapshot.theme().as_ref()
    }

    pub fn segmenters(&self) -> &Arc<TextSegmenters> {
        self.snapshot.segmenters()
    }

    pub fn general_category(&self) -> &Arc<CodePointMapData<GeneralCategory>> {
        self.snapshot.general_category()
    }

    pub fn text_replacement_rules(&self) -> &Arc<[TextReplacementRule]> {
        self.snapshot.text_replacement_rules()
    }

    pub fn auto_surround_enabled(&self) -> bool {
        self.snapshot.auto_surround_enabled()
    }

    pub fn apply_update(
        &mut self,
        snapshot: Arc<ResourceSnapshot>,
    ) -> Result<bool, ResourceApplyError> {
        if Arc::ptr_eq(&self.snapshot, &snapshot) {
            return Ok(false);
        }
        let current = self.snapshot.revision();
        let incoming = snapshot.revision();
        if incoming < current {
            return Err(ResourceApplyError::Stale { current, incoming });
        }
        if incoming == current {
            return Err(ResourceApplyError::ConflictingRevision(incoming));
        }

        let fonts_changed = !Arc::ptr_eq(&self.snapshot.fonts, &snapshot.fonts);
        if fonts_changed {
            self.font_registry = snapshot.fonts.as_ref().clone();
            self.font_context = FontContext::new();
            self.font_registry
                .register_with_font_context(&mut self.font_context);
            self.layout_context = LayoutContext::new();
        }
        self.snapshot = snapshot;
        Ok(true)
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl ResourceSource {
    pub fn new_test() -> Self {
        let segmenters = Arc::new(TextSegmenters::new_test());
        let general_category =
            Arc::new(CodePointMapData::<GeneralCategory>::new().static_to_owned());
        Self::new(IcuResources {
            segmenters,
            general_category,
        })
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Resource {
    pub fn new_test() -> Self {
        Self::from_snapshot(ResourceSource::new_test().snapshot())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_replacement::{RawTextReplacementRule, prepare_text_replacement_rules};

    fn raw_rule(id: &str, pattern: &str, substitute: &str) -> RawTextReplacementRule {
        RawTextReplacementRule {
            id: id.into(),
            match_pattern: pattern.into(),
            substitute: substitute.into(),
            regex: false,
        }
    }

    #[test]
    fn resource_snapshot_theme_update_structurally_shares_unchanged_fields() {
        let mut source = ResourceSource::new_test();
        let before = source.snapshot();

        let committed = source
            .set_theme_variant(ThemeVariant::DarkBlack)
            .expect("theme changed");

        assert_eq!(committed.revision(), ResourceRevision::new(1));
        assert!(Arc::ptr_eq(before.fonts(), committed.fonts()));
        assert!(Arc::ptr_eq(before.segmenters(), committed.segmenters()));
        assert!(Arc::ptr_eq(
            before.general_category(),
            committed.general_category()
        ));
        assert!(Arc::ptr_eq(
            before.text_replacement_rules(),
            committed.text_replacement_rules()
        ));
    }

    #[test]
    fn resource_snapshot_text_replacement_update_shares_theme_and_fonts() {
        let mut source = ResourceSource::new_test();
        let before = source.snapshot();
        let prepared = prepare_text_replacement_rules(vec![raw_rule("dash", "--", "—")]);

        let after = source
            .set_text_replacement_rules(prepared)
            .expect("rules changed");

        assert!(Arc::ptr_eq(before.theme(), after.theme()));
        assert!(Arc::ptr_eq(before.fonts(), after.fonts()));
    }

    #[test]
    fn resource_snapshot_local_resources_have_distinct_mutable_contexts() {
        let source = ResourceSource::new_test();
        let snapshot = source.snapshot();
        let first = Resource::from_snapshot(Arc::clone(&snapshot));
        let second = Resource::from_snapshot(snapshot);

        assert!(!std::ptr::eq(&first.font_context, &second.font_context));
        assert!(!std::ptr::eq(&first.layout_context, &second.layout_context));
        assert!(!std::ptr::eq(&first.font_registry, &second.font_registry));
    }

    #[test]
    fn resource_update_noop_keeps_snapshot_and_revision() {
        let mut source = ResourceSource::new_test();
        let before = source.snapshot();

        assert!(source.set_theme_variant(ThemeVariant::LightWhite).is_none());
        assert!(Arc::ptr_eq(&before, &source.snapshot()));
        assert_eq!(source.revision(), ResourceRevision::INITIAL);
    }

    #[test]
    fn resource_update_preparation_failure_does_not_change_source() {
        let source = ResourceSource::new_test();
        let before = source.snapshot();

        assert!(prepare_font_base(b"not a compressed font").is_err());
        assert!(Arc::ptr_eq(&before, &source.snapshot()));
        assert_eq!(source.revision(), ResourceRevision::INITIAL);
    }

    #[test]
    fn resource_update_invalid_font_chunk_does_not_change_source() {
        let mut source = ResourceSource::new_test();
        let before = source.snapshot();
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.extend_from_slice(&u32::MAX.to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.push(0xAA);
        let prepared = prepare_font_chunk(payload).expect("chunk structure is valid");

        assert!(
            source
                .add_font_chunk(
                    crate::font::PLACEHOLDER_FAMILY_NAME,
                    crate::font::PLACEHOLDER_WEIGHT,
                    0,
                    prepared,
                )
                .is_err()
        );
        assert!(Arc::ptr_eq(&before, &source.snapshot()));
        assert_eq!(source.revision(), ResourceRevision::INITIAL);
    }

    #[test]
    fn resource_update_prepared_changes_apply_to_latest_snapshot() {
        let mut source = ResourceSource::new_test();
        let prepared_rules = prepare_text_replacement_rules(vec![raw_rule("ellipsis", "...", "…")]);

        source
            .set_theme_variant(ThemeVariant::DarkBlack)
            .expect("theme changed");
        let committed = source
            .set_text_replacement_rules(prepared_rules)
            .expect("rules changed");

        assert_eq!(committed.theme().variant(), ThemeVariant::DarkBlack);
        assert_eq!(committed.text_replacement_rules().len(), 1);
        assert_eq!(committed.revision(), ResourceRevision::new(2));
    }

    #[test]
    fn resource_update_equal_font_chunk_bytes_still_commit_first_load_metadata() {
        let mut source = ResourceSource::new_test();
        source
            .add_font_manifest(
                crate::font::PLACEHOLDER_FAMILY_NAME,
                crate::font::PLACEHOLDER_WEIGHT,
                FontManifest::from_coverages(&[vec![0, 0]]),
            )
            .expect("manifest changed");
        let before = source.snapshot();
        let family_id = before
            .fonts()
            .placeholder_family_id()
            .expect("placeholder exists");
        let existing_byte = before
            .fonts()
            .font_data(family_id, crate::font::PLACEHOLDER_WEIGHT)
            .expect("placeholder bytes")[0];
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.extend_from_slice(&0u32.to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.push(existing_byte);
        let prepared = prepare_font_chunk(payload).unwrap();

        let committed = source
            .add_font_chunk(
                crate::font::PLACEHOLDER_FAMILY_NAME,
                crate::font::PLACEHOLDER_WEIGHT,
                0,
                prepared,
            )
            .unwrap()
            .expect("first load changes chunk metadata even when bytes match");

        assert_eq!(committed.revision(), ResourceRevision::new(2));
        assert!(
            committed
                .fonts()
                .is_chunk_loaded(family_id, crate::font::PLACEHOLDER_WEIGHT, 0)
        );
        assert_eq!(
            committed
                .fonts()
                .font_version(family_id, crate::font::PLACEHOLDER_WEIGHT),
            1
        );
    }

    #[test]
    fn resource_update_equal_font_chunk_without_loaded_slot_is_noop() {
        let mut source = ResourceSource::new_test();
        let before = source.snapshot();
        let family_id = before
            .fonts()
            .placeholder_family_id()
            .expect("placeholder exists");
        let existing_byte = before
            .fonts()
            .font_data(family_id, crate::font::PLACEHOLDER_WEIGHT)
            .expect("placeholder bytes")[0];
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.extend_from_slice(&0u32.to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.push(existing_byte);

        let committed = source
            .add_font_chunk(
                crate::font::PLACEHOLDER_FAMILY_NAME,
                crate::font::PLACEHOLDER_WEIGHT,
                0,
                prepare_font_chunk(payload).unwrap(),
            )
            .unwrap();

        assert!(committed.is_none());
        assert!(Arc::ptr_eq(&before, &source.snapshot()));
        assert_eq!(
            source
                .snapshot()
                .fonts()
                .font_version(family_id, crate::font::PLACEHOLDER_WEIGHT),
            0
        );
    }

    #[test]
    fn resource_update_reinserting_same_font_base_after_chunk_is_noop() {
        let compressed = crate::zstd::compress_zstd(PLACEHOLDER_TTF);
        let prepared = prepare_font_base(&compressed).expect("base font is valid");
        let split_offset = prepared.split_offset;
        let mut source = ResourceSource::new_test();
        source
            .add_font_manifest("test", 400, FontManifest::from_coverages(&[vec![0, 0]]))
            .expect("manifest changed");
        let base_snapshot = source
            .insert_font_base("test", 400, prepared)
            .expect("base font changed");
        let family_id = base_snapshot
            .fonts()
            .intern_id("test")
            .expect("font family exists");
        let patched_byte = base_snapshot
            .fonts()
            .font_data(family_id, 400)
            .expect("base font bytes")[split_offset]
            ^ 0xFF;
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.extend_from_slice(&0u32.to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.push(patched_byte);
        source
            .add_font_chunk(
                "test",
                400,
                0,
                prepare_font_chunk(payload).expect("font chunk is valid"),
            )
            .expect("font chunk applies")
            .expect("font chunk changed");
        let before = source.snapshot();
        let before_revision = before.revision();

        let result = source.insert_font_base(
            "test",
            400,
            prepare_font_base(&compressed).expect("same base font is valid"),
        );
        let after = source.snapshot();

        assert!(result.is_none());
        assert!(Arc::ptr_eq(&before, &after));
        assert_eq!(after.revision(), before_revision);
        assert!(after.fonts().is_chunk_loaded(family_id, 400, 0));
        assert_eq!(after.fonts().font_version(family_id, 400), 1);
        assert_eq!(
            after
                .fonts()
                .font_data(family_id, 400)
                .expect("patched font bytes")[split_offset],
            patched_byte
        );
    }

    #[test]
    fn resource_update_different_font_base_remains_replacement() {
        let mut source = ResourceSource::new_test();
        source
            .insert_font_base(
                "test",
                400,
                prepare_font_base(&crate::zstd::compress_zstd(PLACEHOLDER_TTF))
                    .expect("base font is valid"),
            )
            .expect("base font changed");
        let before = source.snapshot();
        let mut different_ttf = PLACEHOLDER_TTF.to_vec();
        different_ttf.push(0);

        let replacement = source
            .insert_font_base(
                "test",
                400,
                prepare_font_base(&crate::zstd::compress_zstd(&different_ttf))
                    .expect("different base font is valid"),
            )
            .expect("different base font replaced the current font");
        let after = source.snapshot();
        let family_id = after.fonts().intern_id("test").expect("font family exists");

        assert!(!Arc::ptr_eq(&before, &after));
        assert!(Arc::ptr_eq(&replacement, &after));
        assert_eq!(
            after.revision(),
            ResourceRevision::new(before.revision().get() + 1)
        );
        assert_eq!(
            after.fonts().font_data(family_id, 400),
            Some(different_ttf.as_slice())
        );
    }

    #[test]
    fn resource_update_apply_snapshot_is_monotonic_and_identity_aware() {
        let mut source = ResourceSource::new_test();
        let initial = source.snapshot();
        let current = source
            .set_theme_variant(ThemeVariant::DarkBlack)
            .expect("theme changed");
        let mut resource = Resource::from_snapshot(Arc::clone(&current));

        assert_eq!(
            resource.apply_update(Arc::clone(&current)),
            Ok(false),
            "same snapshot identity is a no-op"
        );
        assert_eq!(
            resource.apply_update(initial),
            Err(ResourceApplyError::Stale {
                current: ResourceRevision::new(1),
                incoming: ResourceRevision::INITIAL,
            })
        );
        assert_eq!(resource.snapshot().revision(), ResourceRevision::new(1));
    }

    #[test]
    fn resource_update_rejects_same_revision_different_snapshot() {
        let first = ResourceSource::new_test().snapshot();
        let second = ResourceSource::new_test().snapshot();
        assert!(!Arc::ptr_eq(&first, &second));
        let mut resource = Resource::from_snapshot(first);

        assert_eq!(
            resource.apply_update(second),
            Err(ResourceApplyError::ConflictingRevision(
                ResourceRevision::INITIAL
            ))
        );
    }

    #[test]
    fn resource_update_duplicate_font_input_is_canonical_and_repeated_input_is_noop() {
        use crate::font::{FontFamilySource, FontWeight};

        let duplicated = || {
            prepare_fonts(vec![
                FontFamily {
                    name: "Example".into(),
                    source: FontFamilySource::Default,
                    weights: vec![
                        FontWeight {
                            value: 300,
                            hash: "only-in-first".into(),
                        },
                        FontWeight {
                            value: 400,
                            hash: "old".into(),
                        },
                    ],
                },
                FontFamily {
                    name: "Example".into(),
                    source: FontFamilySource::User,
                    weights: vec![
                        FontWeight {
                            value: 400,
                            hash: "new".into(),
                        },
                        FontWeight {
                            value: 700,
                            hash: "bold".into(),
                        },
                    ],
                },
            ])
        };
        let mut source = ResourceSource::new_test();

        let first = source.set_fonts(duplicated()).expect("fonts changed");
        let family_id = first
            .fonts()
            .intern_id("Example")
            .expect("canonical family exists");
        assert_eq!(
            first.fonts().family_source(family_id),
            Some(FontFamilySource::User)
        );
        assert_eq!(first.fonts().weights("Example"), Some(&[300, 400, 700][..]));
        let current = source.snapshot();

        assert!(source.set_fonts(duplicated()).is_none());
        assert!(Arc::ptr_eq(&current, &source.snapshot()));
    }

    #[test]
    fn resource_source_accepts_empty_font_list() {
        let mut source = ResourceSource::new_test();
        assert!(source.set_fonts(prepare_fonts(vec![])).is_none());
    }

    #[test]
    fn new_initializes_with_light_white_theme() {
        let resource = Resource::new_test();
        assert_eq!(resource.theme().variant(), ThemeVariant::LightWhite);
    }

    #[test]
    fn resource_applies_source_theme_update() {
        let mut source = ResourceSource::new_test();
        let mut resource = Resource::from_snapshot(source.snapshot());
        let snapshot = source
            .set_theme_variant(ThemeVariant::DarkBlack)
            .expect("theme changed");

        assert_eq!(resource.apply_update(snapshot), Ok(true));
        assert_eq!(resource.theme().variant(), ThemeVariant::DarkBlack);
    }

    #[test]
    fn placeholder_registered_on_new() {
        use crate::font::{PLACEHOLDER_FAMILY_NAME, PLACEHOLDER_WEIGHT};

        let mut resource = Resource::new_test();

        let id = resource
            .font_registry
            .placeholder_family_id()
            .expect("placeholder family id must be set on Resource::new");
        assert_eq!(
            resource.font_registry.family_name_opt(id),
            Some(PLACEHOLDER_FAMILY_NAME)
        );
        let bytes = resource
            .font_registry
            .font_data(id, PLACEHOLDER_WEIGHT)
            .expect("placeholder bytes present");
        assert!(!bytes.is_empty());

        let family = resource
            .font_context
            .collection
            .family_by_name(PLACEHOLDER_FAMILY_NAME);
        assert!(
            family.is_some(),
            "placeholder must be registered with fontique"
        );
    }

    #[test]
    fn font_bytes_are_shared_between_snapshot_and_local_resources() {
        let compressed = crate::zstd::compress_zstd(PLACEHOLDER_TTF);
        let prepared = prepare_font_base(&compressed).unwrap();
        let mut source = ResourceSource::new_test();
        let snapshot = source
            .insert_font_base("test", 400, prepared)
            .expect("font changed");
        let family_id = snapshot
            .fonts()
            .intern_id("test")
            .expect("font family exists");
        let snapshot_font_data = snapshot
            .fonts()
            .font_data_arc(family_id, 400)
            .expect("snapshot font data exists");
        let first = Resource::from_snapshot(Arc::clone(&snapshot));
        let second = Resource::from_snapshot(snapshot);
        let first_font_data = first
            .font_registry
            .font_data_arc(family_id, 400)
            .expect("first local font data exists");
        let second_font_data = second
            .font_registry
            .font_data_arc(family_id, 400)
            .expect("second local font data exists");

        assert!(Arc::ptr_eq(&snapshot_font_data, &first_font_data));
        assert!(Arc::ptr_eq(&snapshot_font_data, &second_font_data));
    }
}
