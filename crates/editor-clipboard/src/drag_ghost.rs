use crate::{ClipboardAsset, Slice};
use editor_macros::ffi;
use editor_model::{NodeType, PlainNode, Schema};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DragGhost {
    pub text: String,
    pub kind: NodeType,
    pub blocks: Vec<DragGhostBlock>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DragGhostBlock {
    pub kind: NodeType,
    pub count: u32,
}

impl Slice {
    /// Summarize the transferred slice, including open wrappers but never counting
    /// them as whole blocks. Descendants of a counted container only supply text.
    pub fn drag_ghost(&self, assets: &[ClipboardAsset]) -> DragGhost {
        let mut ghost = DragGhost {
            text: String::new(),
            kind: NodeType::Paragraph,
            blocks: vec![],
        };
        let mut stack = Vec::new();
        for (index, fragment) in self.content.iter().enumerate().rev() {
            stack.push((
                fragment,
                if index == 0 { self.open_start } else { 0 },
                if index + 1 == self.content.len() {
                    self.open_end
                } else {
                    0
                },
                false,
            ));
        }
        while let Some((fragment, open_start, open_end, counted_parent)) = stack.pop() {
            let kind = fragment.node.as_type();
            let count = !counted_parent && open_start == 0 && open_end == 0 && is_ghost_block(kind);
            if count {
                if let Some(group) = ghost.blocks.iter_mut().find(|group| group.kind == kind) {
                    group.count += 1;
                } else {
                    ghost.blocks.push(DragGhostBlock { kind, count: 1 });
                }
            }
            match &fragment.node {
                PlainNode::Text(node) => append_text(&mut ghost.text, &node.text),
                PlainNode::HardBreak(_) | PlainNode::Tab(_) => append_text(&mut ghost.text, " "),
                _ => {
                    if Schema::node_spec(kind).is_textblock() || is_ghost_block(kind) {
                        append_text(&mut ghost.text, " ");
                    }
                }
            }
            for (index, child) in fragment.children.iter().enumerate().rev() {
                stack.push((
                    child,
                    if index == 0 {
                        open_start.saturating_sub(1)
                    } else {
                        0
                    },
                    if index + 1 == fragment.children.len() {
                        open_end.saturating_sub(1)
                    } else {
                        0
                    },
                    counted_parent || count,
                ));
            }
        }

        // A single complete block can identify itself on the excerpt row.
        // Assets never supply ordinary excerpt text; only a lone file uses its name.
        if self.content.len() == 1 && self.open_start == 0 && self.open_end == 0 {
            let node = &self.content[0].node;
            if let PlainNode::File(file) = node
                && let Some(asset) = assets
                    .iter()
                    .find(|asset| Some(&asset.id) == file.id.as_ref())
            {
                ghost.text.clear();
                append_text(&mut ghost.text, &asset.label);
            }
            if is_ghost_block(node.as_type()) && !ghost.text.trim().is_empty() {
                ghost.kind = node.as_type();
                ghost.blocks.clear();
            }
        }
        ghost.text = excerpt(ghost.text);
        ghost
    }
}

fn is_ghost_block(kind: NodeType) -> bool {
    matches!(
        kind,
        NodeType::Image
            | NodeType::File
            | NodeType::Embed
            | NodeType::Table
            | NodeType::Callout
            | NodeType::Fold
            | NodeType::HorizontalRule
            | NodeType::PageBreak
            | NodeType::Archived
            | NodeType::Unknown
    )
}

// Normalize as text is collected, without keeping another full copy of the selection.
// Do not insert a separator between adjacent format runs.
fn append_text(out: &mut String, text: &str) {
    for part in text.split_inclusive(char::is_whitespace) {
        let word = part.trim_end_matches(char::is_whitespace);
        out.push_str(word);
        if word.len() < part.len() && !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
    }
}

fn excerpt(mut text: String) -> String {
    // Leave room for full-width glyphs in the web capsule. The frontend displays
    // this finished excerpt verbatim, including any ellipses in the original text.
    const HEAD: usize = 8;
    const TAIL: usize = 6;
    const LIMIT: usize = HEAD + 1 + TAIL;

    text.truncate(text.trim_end().len());
    // Segment only the ends, after joining format runs so split combining
    // characters still form one grapheme.
    if text.graphemes(true).take(LIMIT + 1).count() <= LIMIT {
        text
    } else {
        let head_end = text.grapheme_indices(true).nth(HEAD).unwrap().0;
        let tail_start = text.grapheme_indices(true).rev().nth(TAIL - 1).unwrap().0;
        format!(
            "{}…{}",
            text[..head_end].trim_end(),
            text[tail_start..].trim_start()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

    #[test]
    fn excerpt_uses_only_selected_text_in_document_order() {
        let (state, ..) = state! {
            doc { root {
                p1: paragraph { text("outside first") }
                paragraph { text("middle") }
                p2: paragraph { text("last outside") }
            } }
            selection: (p2, 4) -> (p1, 8)
        };
        let ghost = Slice::extract(&state).unwrap().drag_ghost(&[]);
        assert_eq!(ghost.text, "first mi…e last");
        assert!(ghost.blocks.is_empty());
    }

    #[test]
    fn groups_preserve_first_occurrence_and_exclude_asset_export_text() {
        let (state, ..) = state! {
            doc { r: root {
                image {} file {} image {} embed {} file {}
                table { table_row { table_cell { paragraph { text("cell") } } } }
            } }
            selection: (r, 0) -> (r, 6)
        };
        let ghost = Slice::extract(&state).unwrap().drag_ghost(&[]);
        assert_eq!(ghost.text, "cell");
        assert_eq!(
            ghost.blocks,
            vec![
                DragGhostBlock {
                    kind: NodeType::Image,
                    count: 2
                },
                DragGhostBlock {
                    kind: NodeType::File,
                    count: 2
                },
                DragGhostBlock {
                    kind: NodeType::Embed,
                    count: 1
                },
                DragGhostBlock {
                    kind: NodeType::Table,
                    count: 1
                },
            ]
        );
    }

    #[test]
    fn whole_container_represents_its_nested_contents_once() {
        let (state, ..) = state! {
            doc { r: root { fold {
                fold_title { text("title") }
                fold_content { callout { paragraph { text("inside") } } paragraph { text("hidden") } image {} }
            } } }
            selection: (r, 0) -> (r, 1)
        };
        let ghost = Slice::extract(&state).unwrap().drag_ghost(&[]);
        assert_eq!(ghost.text, "title in…hidden");
        assert_eq!(ghost.kind, NodeType::Fold);
        assert!(ghost.blocks.is_empty());
    }

    #[test]
    fn partial_container_and_cell_text_do_not_claim_the_container() {
        let (state, ..) = state! {
            doc { root {
                callout { p: paragraph { text("unselected chosen") } }
                table { table_row { table_cell { q: paragraph { text("inside outside") } } } }
            } }
            selection: (p, 11) -> (q, 6)
        };
        let ghost = Slice::extract(&state).unwrap().drag_ghost(&[]);
        assert_eq!(ghost.text, "chosen inside");
        assert!(ghost.blocks.is_empty(), "{ghost:?}");
        assert_eq!(ghost.kind, NodeType::Paragraph);
    }

    #[test]
    fn lone_file_uses_name_without_duplicating_its_group() {
        use editor_model::{Fragment, PlainFileNode};
        let slice = Slice::new(
            vec![Fragment::leaf(PlainNode::File(PlainFileNode {
                id: Some("asset".into()),
            }))],
            0,
            0,
        );
        let ghost = slice.drag_ghost(&[ClipboardAsset {
            id: "asset".into(),
            label: "설정 자료.pdf".into(),
            url: "https://example.com/file".into(),
        }]);
        assert_eq!(ghost.text, "설정 자료.pdf");
        assert_eq!(ghost.kind, NodeType::File);
        assert!(ghost.blocks.is_empty());
    }

    #[test]
    fn selected_cells_describe_the_transferred_table_fragment() {
        let (state, ..) = state! {
            doc { root { table { row: table_row {
                table_cell { paragraph { text("outside") } }
                table_cell { paragraph { text("chosen") } }
                table_cell { paragraph { text("outside") } }
            } } } }
            selection: (row, 1) -> (row, 2)
        };
        let ghost = Slice::extract(&state).unwrap().drag_ghost(&[]);
        assert_eq!(ghost.text, "chosen");
        assert_eq!(ghost.kind, NodeType::Table);
        assert!(ghost.blocks.is_empty());
    }

    #[test]
    fn a_complete_callout_uses_its_icon_but_partial_text_does_not() {
        let (state, ..) = state! {
            doc { r: root { callout { paragraph { text("notice") } } } }
            selection: (r, 0) -> (r, 1)
        };
        let ghost = Slice::extract(&state).unwrap().drag_ghost(&[]);
        assert_eq!(ghost.text, "notice");
        assert_eq!(ghost.kind, NodeType::Callout);
        assert!(ghost.blocks.is_empty());
    }

    #[test]
    fn textless_blocks_have_no_excerpt_and_no_kind_limit() {
        use editor_model::Fragment;
        let kinds = [
            NodeType::Image,
            NodeType::File,
            NodeType::Embed,
            NodeType::Table,
            NodeType::Callout,
            NodeType::Fold,
            NodeType::HorizontalRule,
            NodeType::PageBreak,
        ];
        let slice = Slice::new(
            kinds
                .iter()
                .map(|kind| Fragment::leaf(kind.into_node().to_plain()))
                .collect(),
            0,
            0,
        );
        let ghost = slice.drag_ghost(&[]);
        assert!(ghost.text.is_empty());
        assert_eq!(
            ghost
                .blocks
                .iter()
                .map(|block| block.kind)
                .collect::<Vec<_>>(),
            kinds
        );
    }

    #[test]
    fn excerpt_normalizes_whitespace_across_format_runs() {
        let mut text = String::new();
        for run in [" \n", "오늘은\t ", "날씨가", "  좋았", "다 \n"] {
            append_text(&mut text, run);
        }
        assert_eq!(excerpt(text), "오늘은 날씨가 좋았다");

        let mut text = String::new();
        for run in [" \n\t ", "e", "\u{301}", "x", " "] {
            append_text(&mut text, run);
        }
        assert_eq!(excerpt(text), "e\u{301}x");

        let mut text = String::new();
        append_text(&mut text, " \n\t ");
        assert!(excerpt(text).is_empty());
    }

    #[test]
    fn excerpt_keeps_graphemes_and_original_ellipses_on_both_ends() {
        for cluster in ["👩🏽‍💻", "한", "e\u{301}", "🇰🇷"] {
            assert_eq!(excerpt(cluster.repeat(15)), cluster.repeat(15));
            assert_eq!(
                excerpt(cluster.repeat(16)),
                format!("{}…{}", cluster.repeat(8), cluster.repeat(6))
            );
        }
        assert_eq!(
            excerpt(format!("{}끝…", "👩🏽‍💻".repeat(16))),
            format!("{}…{}끝…", "👩🏽‍💻".repeat(8), "👩🏽‍💻".repeat(4))
        );
    }
}
