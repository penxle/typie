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
    pub pending_marks: Option<Vec<Mark>>,
    pub frontiers: Frontiers,
    pub pending_loro_commit: bool,
    pub read_only: bool,
}

impl State {
    pub fn new(doc: Rc<Doc>, selection: Selection) -> Self {
        let frontiers = doc.frontiers();
        Self {
            doc,
            selection,
            preedit: None,
            preferred_x: None,
            pending_marks: None,
            frontiers,
            pending_loro_commit: false,
            read_only: false,
        }
    }
}
