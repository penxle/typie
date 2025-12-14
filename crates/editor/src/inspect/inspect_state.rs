use crate::inspect::inspect_doc::inspect_document_tree;
use crate::inspect::inspect_selection::inspect_selection;
use crate::model::Doc;
use crate::state::Selection;

pub fn inspect_state(doc: &Doc, selection: &Selection) -> String {
    let mut result = String::new();

    result.push_str(&inspect_document_tree(doc));
    result.push_str("\n");
    result.push_str(&inspect_selection(selection));

    result
}
