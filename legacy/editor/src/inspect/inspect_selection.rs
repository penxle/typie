use crate::state::{Position, Selection};

pub fn inspect_selection(selection: &Selection) -> String {
    let mut result = String::from("Selection:\n");

    result.push_str(&format!(
        "  Anchor: {}\n",
        format_position(&selection.anchor)
    ));
    result.push_str(&format!("  Head: {}\n", format_position(&selection.head)));

    result
}

fn format_position(position: &Position) -> String {
    format!(
        "Position node_id={} offset={} affinity={:?}",
        position.node_id, position.offset, position.affinity
    )
}
