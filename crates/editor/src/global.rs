use crate::font::decode_tpft;
use crate::runtime::text_replacement::{
    CompiledPattern, RawTextReplacementRule, TextReplacementRule,
};
use std::cell::{RefCell, UnsafeCell};
use std::collections::HashMap;
use std::sync::Arc;

thread_local! {
    pub static GLOBALS: RefCell<Globals> = RefCell::new(Globals::new());
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

pub struct Font {
    pub data: Arc<SharedFontData>,
    pub split_offset: usize,
}

pub struct Globals {
    pub parley_layout_context: RefCell<parley::LayoutContext<String>>,
    pub parley_font_context: RefCell<parley::FontContext>,
    pub text_replacement_rules: RefCell<Vec<TextReplacementRule>>,
    pub fonts: RefCell<HashMap<(String, u16), Font>>,
    pub auto_surround_enabled: RefCell<bool>,
    pub available_fonts: RefCell<HashMap<String, Vec<u16>>>,
    pub font_versions: RefCell<HashMap<usize, u64>>,
}

impl Globals {
    pub fn new() -> Self {
        Self {
            parley_layout_context: RefCell::new(parley::LayoutContext::new()),
            parley_font_context: RefCell::new(parley::FontContext::new()),
            fonts: RefCell::new(HashMap::new()),
            text_replacement_rules: RefCell::new(Vec::new()),
            auto_surround_enabled: RefCell::new(true),
            available_fonts: RefCell::new(HashMap::new()),
            font_versions: RefCell::new(HashMap::new()),
        }
    }
}

pub(crate) fn font_version(ptr: *const u8) -> u64 {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        globals
            .font_versions
            .borrow()
            .get(&(ptr as usize))
            .copied()
            .unwrap_or(0)
    })
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

        if family_id.is_some() && split_offset > 0 {
            globals.fonts.borrow_mut().insert(
                (family.to_string(), weight),
                Font {
                    data: shared,
                    split_offset,
                },
            );
        }
    });
}

pub fn add_font_chunk(family: &str, weight: u16, data: &[u8]) {
    let chunk_data = decode_tpft(data);
    let num_entries = u32::from_be_bytes(chunk_data[0..4].try_into().unwrap()) as usize;

    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        let fonts = globals.fonts.borrow();

        let key = (family.to_string(), weight);
        let font = match fonts.get(&key) {
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

        *globals
            .font_versions
            .borrow_mut()
            .entry(sfnt.as_ptr() as usize)
            .or_insert(0) += 1;
    });
}

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

pub fn set_available_fonts(fonts: HashMap<String, Vec<u16>>) {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        *globals.available_fonts.borrow_mut() = fonts;
    });
}

pub fn get_available_fonts() -> HashMap<String, Vec<u16>> {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        globals.available_fonts.borrow().clone()
    })
}

pub fn set_text_replacement_rules(raw_rules: Vec<RawTextReplacementRule>) {
    let compiled: Vec<TextReplacementRule> = raw_rules
        .into_iter()
        .filter(|r| !r.match_pattern.is_empty())
        .filter(|r| !r.substitute.is_empty())
        .filter(|r| r.match_pattern != r.substitute)
        .filter_map(|r| {
            let pattern = if r.regex {
                let anchored = format!("(?:{})$", r.match_pattern);
                match fancy_regex::Regex::new(&anchored) {
                    Ok(re) => CompiledPattern::Regex(re),
                    Err(_) => return None,
                }
            } else {
                CompiledPattern::Plain(r.match_pattern)
            };
            Some(TextReplacementRule {
                id: r.id,
                pattern,
                substitute: r.substitute,
            })
        })
        .collect();

    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        *globals.text_replacement_rules.borrow_mut() = compiled;
    });
}

pub fn with_text_replacement_rules<F, R>(f: F) -> R
where
    F: FnOnce(&[TextReplacementRule]) -> R,
{
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        let rules = globals.text_replacement_rules.borrow();
        f(&rules)
    })
}

pub fn clear_text_replacement_rules() {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        globals.text_replacement_rules.borrow_mut().clear();
    });
}

pub fn set_auto_surround_enabled(enabled: bool) {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        *globals.auto_surround_enabled.borrow_mut() = enabled;
    });
}

pub fn is_auto_surround_enabled() -> bool {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        *globals.auto_surround_enabled.borrow()
    })
}
