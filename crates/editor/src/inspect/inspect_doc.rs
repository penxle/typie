use super::utils::truncate_str;
use crate::model::{Doc, Node, NodeId, NodeRef};
use std::collections::BTreeMap;

pub fn inspect_document_tree(doc: &Doc) -> String {
    let Some(root) = doc.node(NodeId::ROOT) else {
        return String::from("Root node not found\n");
    };

    let mut result = String::from("Document Tree:\n");
    print_node(root, "", true, &mut result);
    result
}

fn print_node(node: NodeRef, prefix: &str, is_last: bool, output: &mut String) {
    let connector = if is_last { "└── " } else { "├── " };

    let node_info = format_node_info(&node);
    output.push_str(&format!("{}{}{}\n", prefix, connector, node_info));

    let children: Vec<_> = node.children().collect();
    let child_count = children.len();

    let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

    for (i, child) in children.into_iter().enumerate() {
        let is_last_child = i == child_count - 1;
        print_node(child, &child_prefix, is_last_child, output);
    }
}

fn format_node_info(node: &NodeRef) -> String {
    let id = node.node_id();
    let node_data = node.node();

    match node_data {
        Node::Root(_) => {
            format!("Root {}", id)
        }
        Node::Paragraph(_) => {
            format!("Paragraph {}", id)
        }
        Node::Blockquote(blockquote_node) => {
            format!("Blockquote {} variant={:?}", id, blockquote_node.variant)
        }
        Node::Text(text_node) => {
            let display_text = truncate_str(&text_node.text.as_str(), 50);
            let mut info = format!("Text {} \"{}\"", id, display_text);

            let segments = text_node.text.get_rich_text_segments();
            let mut mark_ranges: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
            let mut offset = 0;

            for (segment_text, marks) in segments.iter() {
                let len = segment_text.chars().count();
                let start = offset;
                let end = offset + len;

                for mark in marks {
                    let label = format!("{:?}", mark);
                    let ranges = mark_ranges.entry(label).or_default();
                    if let Some((last_start, last_end)) = ranges.last_mut() {
                        if *last_end == start && *last_start <= start {
                            *last_end = end;
                            continue;
                        }
                    }
                    ranges.push((start, end));
                }

                offset = end;
            }

            if !mark_ranges.is_empty() {
                let marks_str: Vec<String> = mark_ranges
                    .into_iter()
                    .map(|(label, ranges)| {
                        let ranges_str = ranges
                            .iter()
                            .map(|(start, end)| format!("{}-{}", start, end))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{label} ({ranges_str})")
                    })
                    .collect();
                info.push_str(&format!(" ({})", marks_str.join(", ")));
            }

            info
        }
        Node::Image(image_node) => {
            let id_display = image_node
                .id
                .as_deref()
                .map(|s| truncate_str(s, 30))
                .unwrap_or_else(|| "(placeholder)".to_string());
            format!(
                "Image {} imageId=\"{}\" proportion={}",
                id, id_display, image_node.proportion
            )
        }
        Node::File(file_node) => {
            let id_display = file_node
                .id
                .as_deref()
                .map(|s| truncate_str(s, 30))
                .unwrap_or_else(|| "(placeholder)".to_string());
            format!("File {} fileId=\"{}\"", id, id_display)
        }
        Node::HardBreak(_) => {
            format!("HardBreak {}", id)
        }
        Node::HorizontalRule(_) => {
            format!("HorizontalRule {}", id)
        }
        Node::PageBreak(_) => {
            format!("PageBreak {}", id)
        }
        Node::BulletList(_) => {
            format!("BulletList {}", id)
        }
        Node::OrderedList(_) => {
            format!("OrderedList {}", id)
        }
        Node::ListItem(_) => {
            format!("ListItem {}", id)
        }
        Node::Fold(_) => {
            format!("Fold {}", id)
        }
        Node::FoldTitle(_) => {
            format!("FoldTitle {}", id)
        }
        Node::FoldContent(_) => {
            format!("FoldContent {}", id)
        }
        Node::Callout(callout_node) => {
            format!("Callout {} variant={:?}", id, callout_node.variant)
        }
        Node::Table(table_node) => {
            format!("Table {} border_style={:?}", id, table_node.border_style)
        }
        Node::TableRow(_) => {
            format!("TableRow {}", id)
        }
        Node::TableCell(cell_node) => {
            format!("TableCell {} col_width={:?}", id, cell_node.col_width)
        }
    }
}
