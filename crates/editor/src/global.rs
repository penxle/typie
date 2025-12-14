use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

thread_local! {
    pub static GLOBALS: RefCell<Globals> = RefCell::new(Globals::new());
}

pub struct Globals {
    pub parley_layout_context: RefCell<parley::LayoutContext<String>>,
    pub parley_font_context: RefCell<parley::FontContext>,
    pub available_fonts: RefCell<HashMap<String, Vec<u16>>>,
}

impl Globals {
    pub fn new() -> Self {
        Self {
            parley_layout_context: RefCell::new(parley::LayoutContext::new()),
            parley_font_context: RefCell::new(parley::FontContext::new()),
            available_fonts: RefCell::new(HashMap::new()),
        }
    }
}

#[allow(unused)]
pub fn register_font_family(name: &str, weight: u16, data: &[u8]) -> fontique::FamilyId {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();

        let mut fcx = globals.parley_font_context.borrow_mut();

        let families = fcx.collection.register_fonts(
            fontique::Blob::new(Arc::new(data.to_vec())),
            Some(fontique::FontInfoOverride {
                family_name: Some(name),
                weight: Some(fontique::FontWeight::new(weight as f32)),
                ..Default::default()
            }),
        );

        families.into_iter().next().unwrap().0
    })
}

#[allow(unused)]
pub fn register_fallback_font_family(name: &str, weight: u16, data: &[u8]) -> fontique::FamilyId {
    let family = register_font_family(name, weight, data);

    GLOBALS.with(|globals| {
        let globals = globals.borrow();

        let mut fcx = globals.parley_font_context.borrow_mut();

        for script in icu_properties::props::Script::ALL_VALUES {
            let fallbacks = fcx
                .collection
                .fallback_families(fontique::Script::from(*script))
                .collect::<Vec<_>>();

            fcx.collection.set_fallbacks(
                fontique::Script::from(*script),
                std::iter::once(family).chain(fallbacks),
            );
        }
    });

    family
}

pub fn set_available_fonts(fonts: HashMap<String, Vec<u16>>) {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        let mut available = globals.available_fonts.borrow_mut();
        *available = fonts;
    });
}

#[allow(unused)]
pub fn get_available_fonts() -> HashMap<String, Vec<u16>> {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        globals.available_fonts.borrow().clone()
    })
}

pub fn get_available_font_weights(family_name: &str) -> Vec<u16> {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        if let Some(weights) = globals.available_fonts.borrow().get(family_name) {
            return weights.clone();
        }
        vec![]
    })
}

#[allow(unused)]
pub fn get_loaded_font_weights(family_name: &str) -> Vec<u16> {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();

        let mut fcx = globals.parley_font_context.borrow_mut();

        if let Some(family) = fcx.collection.family_by_name(family_name) {
            let mut weights: Vec<u16> = family
                .fonts()
                .into_iter()
                .map(|font| font.weight().value() as u16)
                .collect();

            weights.sort_unstable();
            weights.dedup();

            if !weights.is_empty() {
                return weights;
            }
        }

        vec![]
    })
}
