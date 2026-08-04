use editor_model::{ChildView, DocView, NodeType, NodeView};

/// Whole-document plain text with the same rules as the clipboard text
/// serializer, so "what you copy" and "what gets counted" never diverge.
pub fn doc_plain_text(view: &DocView<'_>) -> String {
    let mut out = String::new();
    let mut seen_textblock = false;
    if let Some(root) = view.root() {
        walk(&root, &mut out, &mut seen_textblock);
    }
    out
}

fn separate_textblock(out: &mut String, seen_textblock: &mut bool) {
    if *seen_textblock {
        out.push('\n');
    }
    *seen_textblock = true;
}

fn walk(node: &NodeView<'_>, out: &mut String, seen_textblock: &mut bool) {
    if node.node_type() == NodeType::Table {
        separate_textblock(out, seen_textblock);
        walk_table(node, out);
        return;
    }
    if node.spec().is_textblock() {
        separate_textblock(out, seen_textblock);
    }
    for child in node.children() {
        match child {
            ChildView::Block(b) => walk(&b, out, seen_textblock),
            ChildView::Leaf(l) => {
                if let Some(c) = l.as_char() {
                    out.push(c);
                } else {
                    match l.node_type() {
                        NodeType::HardBreak => out.push('\n'),
                        NodeType::Tab => out.push('\t'),
                        _ => {}
                    }
                }
            }
        }
    }
}

fn walk_table(table: &NodeView<'_>, out: &mut String) {
    let mut first_row = true;
    for row in table.children() {
        let ChildView::Block(row) = row else { continue };
        if row.node_type() != NodeType::TableRow {
            continue;
        }
        if !first_row {
            out.push('\n');
        }
        first_row = false;
        let mut first_cell = true;
        for cell in row.children() {
            let ChildView::Block(cell) = cell else {
                continue;
            };
            if cell.node_type() != NodeType::TableCell {
                continue;
            }
            if !first_cell {
                out.push('\t');
            }
            first_cell = false;
            collect_cell_text(&cell, out);
        }
    }
}

fn collect_cell_text(node: &NodeView<'_>, out: &mut String) {
    for child in node.children() {
        match child {
            ChildView::Block(b) => collect_cell_text(&b, out),
            ChildView::Leaf(l) => {
                if let Some(c) = l.as_char() {
                    out.push(c);
                } else if matches!(l.node_type(), NodeType::HardBreak | NodeType::Tab) {
                    out.push(' ');
                }
            }
        }
    }
}
