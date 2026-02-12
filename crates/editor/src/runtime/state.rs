use crate::model::*;
use crate::state::{Preedit, Selection};
use loro::Frontiers;
use std::rc::Rc;

#[derive(Clone)]
pub struct State {
    pub doc: Rc<Doc>,
    pub selection: Selection,
    pub preedit: Option<Preedit>,
    pub preferred_x: Option<f32>,
    pub pending_styles: Vec<Style>,
    pub frontiers: Frontiers,
    pub pending_loro_commit: bool,
    pub read_only: bool,
}

impl State {
    pub fn new(doc: Rc<Doc>, selection: Selection) -> Self {
        let frontiers = doc.frontiers();
        let pending_styles = doc.default_styles().to_styles();
        Self {
            doc,
            selection,
            preedit: None,
            preferred_x: None,
            pending_styles,
            frontiers,
            pending_loro_commit: false,
            read_only: false,
        }
    }
}
