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
pub fn add_font(name: &str, weight: u16, data: &[u8]) -> Option<fontique::FamilyId> {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        let mut fcx = globals.parley_font_context.borrow_mut();

        if let Some(family) = fcx.collection.family_by_name(name) {
            let has_weight = family
                .fonts()
                .iter()
                .any(|font| font.weight().value() as u16 == weight);
            if has_weight {
                return None;
            }
        }

        let families = fcx.collection.register_fonts(
            fontique::Blob::new(Arc::new(data.to_vec())),
            Some(fontique::FontInfoOverride {
                family_name: Some(name),
                weight: Some(fontique::FontWeight::new(weight as f32)),
                ..Default::default()
            }),
        );

        families.into_iter().next().map(|(id, _)| id)
    })
}

#[allow(unused)]
pub fn register_fallback_font(name: &str) {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        let mut fcx = globals.parley_font_context.borrow_mut();

        let Some(family) = fcx.collection.family_by_name(name).map(|f| f.id()) else {
            return;
        };

        let already_registered = fcx
            .collection
            .fallback_families(fontique::Script::from(icu_properties::props::Script::Latin))
            .any(|id| id == family);
        if already_registered {
            return;
        }

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
