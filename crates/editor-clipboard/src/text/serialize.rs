use crate::slice::Slice;
use editor_model::{Fragment, PlainNode, Schema};

pub fn to_text(slice: &Slice) -> String {
    let mut out = String::new();
    let mut context = TextContext::default();
    for fragment in &slice.content {
        walk(fragment, &mut out, &mut context);
    }
    out
}

#[derive(Default)]
struct TextContext {
    seen_textblock: bool,
}

fn walk(fragment: &Fragment, out: &mut String, context: &mut TextContext) {
    match &fragment.node {
        PlainNode::Text(t) => out.push_str(&t.text),
        PlainNode::HardBreak(_) => out.push('\n'),
        PlainNode::Tab(_) => out.push('\t'),
        PlainNode::Table(_) => walk_table(fragment, out, context),
        _ => {
            if is_textblock_node(&fragment.node) {
                separate_textblock(out, context);
            }
            for child in &fragment.children {
                walk(child, out, context);
            }
        }
    }
}

fn is_textblock_node(n: &PlainNode) -> bool {
    Schema::node_spec(n.as_type()).is_textblock()
}

fn separate_textblock(out: &mut String, context: &mut TextContext) {
    if context.seen_textblock && !out.ends_with("\n\n") {
        if out.ends_with('\n') {
            out.push('\n');
        } else {
            out.push_str("\n\n");
        }
    }
    context.seen_textblock = true;
}

// TSV-style emission: tabs between cells, newlines between rows. Cell content
// is flattened to inline text so multi-block cells don't shred the row layout.
fn walk_table(table: &Fragment, out: &mut String, context: &mut TextContext) {
    separate_textblock(out, context);
    let mut first_row = true;
    for row in &table.children {
        if !matches!(row.node, PlainNode::TableRow(_)) {
            continue;
        }
        if !first_row {
            out.push('\n');
        }
        first_row = false;
        let mut first_cell = true;
        for cell in &row.children {
            if !matches!(cell.node, PlainNode::TableCell(_)) {
                continue;
            }
            if !first_cell {
                out.push('\t');
            }
            first_cell = false;
            collect_cell_text(cell, out);
        }
    }
}

fn collect_cell_text(node: &Fragment, out: &mut String) {
    match &node.node {
        PlainNode::Text(t) => out.push_str(&t.text),
        PlainNode::HardBreak(_) => out.push(' '),
        PlainNode::Tab(_) => out.push(' '),
        _ => {
            for c in &node.children {
                collect_cell_text(c, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::Slice;
    use crate::test_doc::DocBuilder;
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_model::{AtomLeaf, NodeType};
    use editor_state::{Position, Selection};

    #[test]
    fn to_text_single_paragraph() {
        let (s, _p1) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 0) -> (p1, 11)
        };
        let slice = Slice::extract(&s).unwrap();
        assert_eq!(to_text(&slice), "Hello World");
    }

    #[test]
    fn to_text_multi_paragraph_with_hardbreak() {
        let mut b = DocBuilder::new();
        let root = Dot::ROOT;
        let p1 = b.block(NodeType::Paragraph, &[root]);
        b.text("first");
        b.atom(AtomLeaf::HardBreak, &[]);
        b.text("line");
        let p2 = b.block(NodeType::Paragraph, &[root]);
        b.text("second");
        let s = b.finish(Some(Selection::new(
            Position::new(p1, 0),
            Position::new(p2, 6),
        )));
        let slice = Slice::extract(&s).unwrap();
        assert_eq!(slice.to_text(), "first\nline\n\nsecond");
    }

    #[test]
    fn to_text_preserves_empty_paragraph_separator() {
        let slice = Slice::from_text("\n\n");
        assert_eq!(slice.to_text(), "\n\n");
    }

    #[test]
    fn to_text_emits_tab_for_tab_node() {
        let mut b = DocBuilder::new();
        let root = Dot::ROOT;
        let para = b.block(NodeType::Paragraph, &[root]);
        b.text("a");
        b.atom(AtomLeaf::Tab, &[]);
        b.text("b");
        let s2 = b.finish(Some(Selection::new(
            Position::new(para, 0),
            Position::new(para, 3),
        )));
        let slice = Slice::extract(&s2).unwrap();
        assert_eq!(slice.to_text(), "a\tb");
    }

    #[test]
    fn to_text_cell_rect_is_tsv() {
        let (s, _, c00, _, _, _, c11) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = cell_rect_sel(&s, c00, c11);
        let s = editor_state::State {
            selection: Some(sel),
            ..s
        };
        let slice = Slice::extract(&s).unwrap();
        assert_eq!(slice.to_text(), "a\tb\nc\td");
    }

    #[test]
    fn cell_internal_tab_flattens_to_space_not_column() {
        let mut b = DocBuilder::new();
        let root = Dot::ROOT;
        let table = b.block(NodeType::Table, &[root]);
        let row = b.block(NodeType::TableRow, &[root, table]);
        let c00 = b.block(NodeType::TableCell, &[root, table, row]);
        let _cp0 = b.block(NodeType::Paragraph, &[root, table, row, c00]);
        b.text("x");
        b.atom(AtomLeaf::Tab, &[]);
        b.text("y");
        let c11 = b.block(NodeType::TableCell, &[root, table, row]);
        let _cp1 = b.block(NodeType::Paragraph, &[root, table, row, c11]);
        b.text("z");
        let s = b.finish(None);
        let sel = cell_rect_sel(&s, c00, c11);
        let s = editor_state::State {
            selection: Some(sel),
            ..s
        };
        let slice = Slice::extract(&s).unwrap();
        assert_eq!(slice.to_text(), "x y\tz");
    }

    fn cell_rect_sel(
        state: &editor_state::State,
        anchor_cell: Dot,
        head_cell: Dot,
    ) -> editor_state::Selection {
        use editor_state::{Position, Selection};
        let view = state.view();
        let a = view.node(anchor_cell).unwrap();
        let h = view.node(head_cell).unwrap();
        let a_row = a.parent().unwrap().id();
        let h_row = h.parent().unwrap().id();
        let a_col = a.index().unwrap();
        let h_col = h.index().unwrap();
        let lo = a_col.min(h_col);
        let hi = a_col.max(h_col);
        Selection::new(Position::new(a_row, lo), Position::new(h_row, hi + 1))
    }
}
