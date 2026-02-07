use std::cell::{RefCell, UnsafeCell};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::Arc;

use crate::global::GLOBALS;

thread_local! {
    static FONT_VERSIONS: RefCell<HashMap<usize, u64>> = RefCell::new(HashMap::new());
}

pub(crate) fn font_version(ptr: *const u8) -> u64 {
    FONT_VERSIONS.with(|v| v.borrow().get(&(ptr as usize)).copied().unwrap_or(0))
}

fn bump_font_version(ptr: *const u8) {
    FONT_VERSIONS.with(|v| {
        *v.borrow_mut().entry(ptr as usize).or_insert(0) += 1;
    });
}

pub(crate) struct SharedFontData(UnsafeCell<Vec<u8>>);

unsafe impl Send for SharedFontData {}
unsafe impl Sync for SharedFontData {}

impl AsRef<[u8]> for SharedFontData {
    fn as_ref(&self) -> &[u8] {
        unsafe { &*self.0.get() }
    }
}

impl SharedFontData {
    pub(crate) fn new(data: Vec<u8>) -> Self {
        Self(UnsafeCell::new(data))
    }

    unsafe fn as_mut_slice(&self) -> &mut [u8] {
        unsafe { (*self.0.get()).as_mut_slice() }
    }
}

pub struct LazyFont {
    pub family_id: fontique::FamilyId,
    pub data: Arc<SharedFontData>,
    pub split_offset: usize,
}

const TPFT_MAGIC: &[u8; 4] = b"TPFT";
const TPFT_VERSION: u16 = 1;
const TPFT_HEADER_SIZE: usize = 6;

fn decode_tpft(data: &[u8]) -> Vec<u8> {
    assert_eq!(&data[0..4], TPFT_MAGIC, "invalid TPFT magic");
    let version = u16::from_be_bytes(data[4..6].try_into().unwrap());
    assert_eq!(version, TPFT_VERSION, "unsupported TPFT version {version}");
    let mut decoder = ruzstd::decoding::StreamingDecoder::new(&data[TPFT_HEADER_SIZE..])
        .expect("zstd init failed");
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).expect("zstd decode failed");
    buf
}

pub(crate) fn register_font(
    fcx: &mut parley::FontContext,
    family: &str,
    weight: u16,
    font_data: Vec<u8>,
) -> Option<fontique::FamilyId> {
    fcx.collection
        .register_fonts(
            fontique::Blob::new(Arc::new(font_data)),
            Some(fontique::FontInfoOverride {
                family_name: Some(family),
                weight: Some(fontique::FontWeight::new(weight as f32)),
                ..Default::default()
            }),
        )
        .into_iter()
        .next()
        .map(|(id, _)| id)
}

#[allow(unused)]
pub fn add_font_base(family: &str, weight: u16, data: &[u8]) {
    let decompressed = decode_tpft(data);
    let split_offset = u32::from_be_bytes(decompressed[0..4].try_into().unwrap()) as usize;
    let sfnt = decompressed[4..].to_vec();
    let shared = Arc::new(SharedFontData::new(sfnt));

    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        let mut fcx = globals.parley_font_context.borrow_mut();

        let family_id = fcx
            .collection
            .register_fonts(
                fontique::Blob::new(shared.clone()),
                Some(fontique::FontInfoOverride {
                    family_name: Some(family),
                    weight: Some(fontique::FontWeight::new(weight as f32)),
                    ..Default::default()
                }),
            )
            .into_iter()
            .next()
            .map(|(id, _)| id);

        if let Some(family_id) = family_id {
            if split_offset > 0 {
                globals.lazy_fonts.borrow_mut().insert(
                    (family.to_string(), weight),
                    LazyFont {
                        family_id,
                        data: shared,
                        split_offset,
                    },
                );
            }
        }
    });
}

#[allow(unused)]
pub fn add_font_chunk(family: &str, weight: u16, data: &[u8]) {
    let chunk_data = decode_tpft(data);
    let num_entries = u32::from_be_bytes(chunk_data[0..4].try_into().unwrap()) as usize;

    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        let lazy_fonts = globals.lazy_fonts.borrow();

        let key = (family.to_string(), weight);
        let font = match lazy_fonts.get(&key) {
            Some(f) => f,
            None => return,
        };

        let sfnt = unsafe { font.data.as_mut_slice() };
        let mut pos = 4;
        for _ in 0..num_entries {
            let offset = u32::from_be_bytes(chunk_data[pos..pos + 4].try_into().unwrap()) as usize;
            let len = u32::from_be_bytes(chunk_data[pos + 4..pos + 8].try_into().unwrap()) as usize;
            let src = &chunk_data[pos + 8..pos + 8 + len];

            let dst = font.split_offset + offset;
            sfnt[dst..dst + len].copy_from_slice(src);

            pos += 8 + len;
        }

        bump_font_version(sfnt.as_ptr());
    });
}

#[allow(unused)]
pub fn set_fallback_fonts(names: &[&str]) {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        let mut fcx = globals.parley_font_context.borrow_mut();

        let families: Vec<fontique::FamilyId> = names
            .iter()
            .filter_map(|name| fcx.collection.family_by_name(name).map(|f| f.id()))
            .collect();

        for script in icu_properties::props::Script::ALL_VALUES {
            fcx.collection
                .set_fallbacks(fontique::Script::from(*script), families.iter().copied());
        }
    });
}

#[allow(unused)]
pub fn get_available_fonts() -> HashMap<String, Vec<u16>> {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        let mut fcx = globals.parley_font_context.borrow_mut();
        let fallbacks: HashSet<fontique::FamilyId> = fcx
            .collection
            .fallback_families(fontique::Script::from(icu_properties::props::Script::Latin))
            .collect();
        let lazy_fonts = globals.lazy_fonts.borrow();
        let mut result: HashMap<String, Vec<u16>> = HashMap::new();
        for ((family, weight), font) in lazy_fonts.iter() {
            if !fallbacks.contains(&font.family_id) {
                result.entry(family.clone()).or_default().push(*weight);
            }
        }
        for weights in result.values_mut() {
            weights.sort();
        }
        result
    })
}
