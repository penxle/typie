use editor_model::{Doc, NodeId, Schema};

use super::run::ProseRun;
use super::view::ProseText;
use crate::{DocFlatExt, FlatSegment};

pub(super) fn run(doc: &Doc) -> ProseText {
    let mut state = EmitState::default();
    for (flat_offset, segment) in doc.flat_segments() {
        state.handle(doc, flat_offset, segment);
    }
    state.finish()
}

#[derive(Default)]
struct EmitState {
    text: String,
    runs: Vec<ProseRun>,
    plain_len: usize,
    pending_boundary: bool,
    block_emitted_stack: Vec<bool>,
    last_text_end_flat: usize,
}

impl EmitState {
    fn handle(&mut self, doc: &Doc, flat_offset: usize, segment: FlatSegment) {
        match segment {
            FlatSegment::Text { text, .. } => self.emit_text(flat_offset, &text),
            FlatSegment::Break { .. } => self.emit_break(flat_offset),
            FlatSegment::Atom { .. } => {}
            FlatSegment::Open { node_id } => self.handle_open(doc, node_id),
            FlatSegment::Close { node_id } => self.handle_close(doc, node_id),
        }
    }

    fn handle_open(&mut self, doc: &Doc, node_id: NodeId) {
        let Some(entry) = doc.get_entry(node_id) else {
            return;
        };
        let kind = entry.node.as_type();
        if Schema::node_spec(kind).is_textblock() {
            self.block_emitted_stack.push(false);
        }
    }

    fn handle_close(&mut self, doc: &Doc, node_id: NodeId) {
        let Some(entry) = doc.get_entry(node_id) else {
            return;
        };
        let kind = entry.node.as_type();
        if Schema::node_spec(kind).is_textblock() {
            if let Some(emitted) = self.block_emitted_stack.pop() {
                if emitted {
                    self.pending_boundary = true;
                }
            }
        }
    }

    fn emit_text(&mut self, flat_offset: usize, text: &str) {
        let n = text.chars().count();
        if n == 0 {
            return;
        }
        self.flush_pending_boundary(flat_offset);
        let p = self.plain_len;
        self.runs.push(ProseRun {
            plain_range: p..p + n,
            flat_start: flat_offset,
        });
        self.text.push_str(text);
        self.plain_len += n;
        self.last_text_end_flat = flat_offset + n;
        self.mark_block_emitted();
    }

    fn emit_break(&mut self, flat_offset: usize) {
        self.flush_pending_boundary(flat_offset);
        let p = self.plain_len;
        self.runs.push(ProseRun {
            plain_range: p..p + 1,
            flat_start: flat_offset,
        });
        self.text.push('\n');
        self.plain_len += 1;
        self.last_text_end_flat = flat_offset + 1;
        self.mark_block_emitted();
    }

    fn flush_pending_boundary(&mut self, current_flat: usize) {
        if !self.pending_boundary {
            return;
        }
        let p = self.plain_len;
        self.runs.push(ProseRun {
            plain_range: p..p + 1,
            flat_start: self.last_text_end_flat,
        });
        debug_assert!(current_flat >= 1);
        self.runs.push(ProseRun {
            plain_range: (p + 1)..(p + 2),
            flat_start: current_flat - 1,
        });
        self.text.push_str("\n\n");
        self.plain_len += 2;
        self.pending_boundary = false;
    }

    fn mark_block_emitted(&mut self) {
        if let Some(top) = self.block_emitted_stack.last_mut() {
            *top = true;
        }
    }

    fn finish(self) -> ProseText {
        ProseText::from_parts(self.text, self.runs, self.plain_len)
    }
}

#[cfg(test)]
mod tests {
    use crate::DocProseExt;
    use editor_macros::doc;

    #[test]
    fn empty_doc_produces_empty_prose() {
        let (doc, ..) = doc! { root {} };
        let prose = doc.prose();
        assert_eq!(prose.text(), "");
        assert!(prose.runs.is_empty());
    }

    #[test]
    fn single_paragraph_text() {
        let (doc, _p, t1) = doc! { root { _p: paragraph { t1: text("hello") } } };
        let prose = doc.prose();
        assert_eq!(prose.text(), "hello");
        assert_eq!(prose.runs.len(), 1);
        let r = &prose.runs[0];
        assert_eq!(r.plain_range, 0..5);
        assert_eq!(r.flat_start, 1);
        let _ = t1;
    }

    #[test]
    fn paragraph_with_multiple_text_nodes() {
        let (doc, ..) = doc! { root { paragraph { text("hel") text("lo") } } };
        let prose = doc.prose();
        assert_eq!(prose.text(), "hello");
        assert_eq!(prose.runs.len(), 2);
        assert_eq!(prose.runs[0].plain_range, 0..3);
        assert_eq!(prose.runs[1].plain_range, 3..5);
        assert_eq!(
            prose.runs[0].plain_range.end,
            prose.runs[1].plain_range.start
        );
    }

    #[test]
    fn paragraph_with_multibyte_chars() {
        let (doc, ..) = doc! { root { paragraph { text("한글") } } };
        let prose = doc.prose();
        assert_eq!(prose.text(), "한글");
        assert_eq!(prose.runs.len(), 1);
        assert_eq!(prose.runs[0].plain_range, 0..2);
    }

    #[test]
    fn empty_text_node_emits_nothing() {
        let (doc, ..) = doc! { root { paragraph { text("") } } };
        let prose = doc.prose();
        assert_eq!(prose.text(), "");
        assert!(prose.runs.is_empty(), "no run for an empty Text node");
    }

    #[test]
    fn paragraph_with_hard_break() {
        let (doc, ..) = doc! {
            root { paragraph { text("a") hard_break {} text("b") } }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\nb");
        assert_eq!(prose.runs.len(), 3);
        assert_eq!(prose.runs[0].plain_range, 0..1);
        assert_eq!(prose.runs[1].plain_range, 1..2);
        assert_eq!(prose.runs[2].plain_range, 2..3);
        assert_eq!(
            prose.runs[0].plain_range.end,
            prose.runs[1].plain_range.start
        );
        assert_eq!(
            prose.runs[1].plain_range.end,
            prose.runs[2].plain_range.start
        );
    }

    #[test]
    fn two_paragraphs_emit_double_newline() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                paragraph { text("b") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\n\nb");
        assert_eq!(prose.runs.len(), 4);
        assert_eq!(prose.runs[0].plain_range, 0..1);
        assert_eq!(prose.runs[0].flat_start, 1);
        assert_eq!(prose.runs[1].plain_range, 1..2);
        assert_eq!(prose.runs[1].flat_start, 2);
        assert_eq!(prose.runs[2].plain_range, 2..3);
        assert_eq!(prose.runs[2].flat_start, 3);
        assert_eq!(prose.runs[3].plain_range, 3..4);
        assert_eq!(prose.runs[3].flat_start, 4);
    }

    #[test]
    fn leading_empty_paragraph_collapses() {
        let (doc, ..) = doc! {
            root {
                paragraph {}
                paragraph { text("a") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a");
    }

    #[test]
    fn trailing_empty_paragraph_collapses() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                paragraph {}
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a");
    }

    #[test]
    fn middle_empty_paragraph_collapses() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                paragraph {}
                paragraph { text("b") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\n\nb");
    }

    #[test]
    fn paragraph_with_only_empty_text_collapses() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("") }
                paragraph { text("a") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a");
    }

    #[test]
    fn hard_break_then_block_boundary() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") hard_break {} text("b") }
                paragraph { text("c") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\nb\n\nc");
    }

    #[test]
    fn hard_break_only_paragraph_then_text_produces_triple_newline() {
        let (doc, ..) = doc! {
            root {
                paragraph { hard_break {} }
                paragraph { text("a") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "\n\n\na");
    }

    #[test]
    fn block_boundary_then_hard_break_produces_triple_newline() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                paragraph { hard_break {} text("b") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\n\n\nb");
    }

    #[test]
    fn all_empty_paragraphs_doc_is_empty() {
        let (doc, ..) = doc! {
            root {
                paragraph {}
                paragraph {}
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "");
        assert!(prose.runs.is_empty());
    }

    #[test]
    fn atom_between_paragraphs_still_emits_double_newline() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                horizontal_rule {}
                paragraph { text("b") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\n\nb");
    }

    #[test]
    fn archived_atom_between_paragraphs() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                archived {}
                paragraph { text("b") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\n\nb");
    }

    #[test]
    fn archived_only_doc_is_empty() {
        let (doc, ..) = doc! { root { archived {} } };
        let prose = doc.prose();
        assert_eq!(prose.text(), "");
        assert!(prose.runs.is_empty());
    }

    #[test]
    fn image_only_doc_is_empty() {
        let (doc, ..) = doc! { root { image {} } };
        let prose = doc.prose();
        assert_eq!(prose.text(), "");
        assert!(prose.runs.is_empty());
    }

    #[test]
    fn blockquote_is_transparent() {
        let (doc, ..) = doc! { root { blockquote { paragraph { text("hi") } } } };
        let prose = doc.prose();
        assert_eq!(prose.text(), "hi");
        assert_eq!(prose.runs.len(), 1);
    }

    #[test]
    fn fold_title_is_a_text_block() {
        let (doc, ..) = doc! {
            root {
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { text("body") } }
                }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "title\n\nbody");
    }

    #[test]
    fn fold_then_paragraph_boundary() {
        let (doc, ..) = doc! {
            root {
                fold {
                    fold_title { text("t") }
                    fold_content { paragraph { text("c") } }
                }
                paragraph { text("p") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "t\n\nc\n\np");
    }

    #[test]
    fn page_break_emits_newline_like_hard_break() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") page_break {} }
                paragraph { text("b") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\n\n\nb");
    }

    #[test]
    fn bullet_list_items_separated_by_double_newline() {
        let (doc, ..) = doc! {
            root {
                bullet_list {
                    list_item { paragraph { text("a") } }
                    list_item { paragraph { text("b") } }
                }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\n\nb");
    }

    #[test]
    fn ordered_list_items_separated_by_double_newline() {
        let (doc, ..) = doc! {
            root {
                ordered_list {
                    list_item { paragraph { text("first") } }
                    list_item { paragraph { text("second") } }
                }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "first\n\nsecond");
    }

    #[test]
    fn nested_list_collapses_to_sequence() {
        let (doc, ..) = doc! {
            root {
                bullet_list {
                    list_item {
                        paragraph { text("a") }
                        bullet_list {
                            list_item { paragraph { text("a.1") } }
                        }
                    }
                }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\n\na.1");
    }

    #[test]
    fn table_cells_appear_as_paragraph_sequence() {
        let (doc, ..) = doc! {
            root {
                table {
                    table_row {
                        table_cell { paragraph { text("a") } }
                        table_cell { paragraph { text("b") } }
                    }
                    table_row {
                        table_cell { paragraph { text("c") } }
                        table_cell { paragraph { text("d") } }
                    }
                }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\n\nb\n\nc\n\nd");
    }

    #[test]
    fn callout_then_paragraph_boundary() {
        let (doc, ..) = doc! {
            root {
                callout { paragraph { text("warn") } }
                paragraph { text("after") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "warn\n\nafter");
    }
}
