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
        let pending_styles = Self::compute_initial_pending_styles(&doc, &selection);
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

    fn compute_initial_pending_styles(doc: &Doc, selection: &Selection) -> Vec<Style> {
        let node_id = selection.head.node_id;
        if let Some(node) = doc.node(node_id) {
            let mut attrs = Vec::new();
            for ancestor in node.ancestors() {
                if let Some(cascade) = ancestor.cascade_attrs() {
                    for attr in cascade {
                        if !attrs.iter().any(|a: &Attr| a.key() == attr.key()) {
                            attrs.push(attr);
                        }
                    }
                }
            }
            let styles = Attr::extract_styles(&attrs);
            if !styles.is_empty() {
                return styles;
            }
        }
        DefaultAttrs::default().to_styles()
    }
}
