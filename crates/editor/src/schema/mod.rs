#[macro_use]
mod content_macro;

mod content;
mod spec;

pub use content::{ContentExpr, RepairAction};
pub use spec::*;

use crate::model::{MarkType, NodeType};
use crate::schema::spec::MarkSpec;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct Schema {
    nodes: FxHashMap<NodeType, NodeSpec>,
    marks: FxHashMap<MarkType, MarkSpec>,
}

impl Schema {
    pub fn new() -> Self {
        Self {
            nodes: FxHashMap::default(),
            marks: FxHashMap::default(),
        }
    }

    #[allow(dead_code)]
    pub fn nodes(&self) -> &FxHashMap<NodeType, NodeSpec> {
        &self.nodes
    }

    pub fn marks(&self) -> &FxHashMap<MarkType, MarkSpec> {
        &self.marks
    }

    pub fn add_node(&mut self, node_type: NodeType, spec: NodeSpec) {
        self.nodes.insert(node_type, spec);
    }

    pub fn add_mark(&mut self, mark_type: MarkType, spec: MarkSpec) {
        self.marks.insert(mark_type, spec);
    }

    pub fn node_spec(&self, node_type: NodeType) -> &NodeSpec {
        self.nodes
            .get(&node_type)
            .unwrap_or_else(|| panic!("Unknown node type: {:?}", node_type))
    }

    #[allow(dead_code)]
    pub fn mark_spec(&self, mark_type: MarkType) -> &MarkSpec {
        self.marks
            .get(&mark_type)
            .unwrap_or_else(|| panic!("Unknown mark type: {:?}", mark_type))
    }
}

impl Default for Schema {
    fn default() -> Self {
        let mut schema = Schema::new();

        schema.add_node(
            NodeType::Root,
            NodeSpec {
                content: content_expr!([((Paragraph | Image | File | Embed | Blockquote | Callout | BulletList | OrderedList | HorizontalRule | Fold | Table)*), (Paragraph)]),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Blockquote,
            NodeSpec {
                content: content_expr!((Paragraph | BulletList | OrderedList)+),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Paragraph,
            NodeSpec {
                content: content_expr!([((Text | HardBreak)*), (PageBreak?)]),
                marks: Some(&[
                    MarkType::BackgroundColor,
                    MarkType::FontFamily,
                    MarkType::FontSize,
                    MarkType::FontWeight,
                    MarkType::Italic,
                    MarkType::LetterSpacing,
                    MarkType::Link,
                    MarkType::Ruby,
                    MarkType::Strikethrough,
                    MarkType::TextColor,
                    MarkType::Underline,
                ]),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Text,
            NodeSpec {
                marks: Some(&[]),
                inline: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Image,
            NodeSpec {
                selectable: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::File,
            NodeSpec {
                selectable: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Embed,
            NodeSpec {
                selectable: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::HardBreak,
            NodeSpec {
                inline: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::PageBreak,
            NodeSpec {
                inline: true,
                grandparent_must_be: Some(NodeType::Root),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::HorizontalRule,
            NodeSpec {
                selectable: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::BulletList,
            NodeSpec {
                content: content_expr!(ListItem+),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::OrderedList,
            NodeSpec {
                content: content_expr!(ListItem+),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::ListItem,
            NodeSpec {
                content: content_expr!([(Paragraph), ((BulletList | OrderedList) *)]),
                structural: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Fold,
            NodeSpec {
                content: content_expr!([(FoldTitle), (FoldContent)]),
                isolating: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::FoldTitle,
            NodeSpec {
                content: content_expr!(Text*),
                marks: Some(&[]),
                isolating: true,
                structural: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::FoldContent,
            NodeSpec {
                content: content_expr!((Paragraph | Image | File | Embed | Blockquote | Callout | BulletList | OrderedList | HorizontalRule | Fold)+),
                isolating: true,
                structural: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Callout,
            NodeSpec {
                content: content_expr!((Paragraph | BulletList | OrderedList)+),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Table,
            NodeSpec {
                content: content_expr!(TableRow+),
                isolating: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::TableRow,
            NodeSpec {
                content: content_expr!(TableCell+),
                structural: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::TableCell,
            NodeSpec {
                content: content_expr!((Paragraph | Image | File | Embed | BulletList | OrderedList)+),
                isolating: true,
                structural: true,
                ..Default::default()
            },
        );

        schema.add_mark(
            MarkType::BackgroundColor,
            MarkSpec {
                ..Default::default()
            },
        );
        schema.add_mark(
            MarkType::FontFamily,
            MarkSpec {
                ..Default::default()
            },
        );
        schema.add_mark(
            MarkType::FontSize,
            MarkSpec {
                ..Default::default()
            },
        );
        schema.add_mark(
            MarkType::FontWeight,
            MarkSpec {
                ..Default::default()
            },
        );
        schema.add_mark(
            MarkType::Italic,
            MarkSpec {
                ..Default::default()
            },
        );
        schema.add_mark(
            MarkType::LetterSpacing,
            MarkSpec {
                ..Default::default()
            },
        );
        schema.add_mark(
            MarkType::Strikethrough,
            MarkSpec {
                ..Default::default()
            },
        );
        schema.add_mark(
            MarkType::Underline,
            MarkSpec {
                ..Default::default()
            },
        );
        schema.add_mark(
            MarkType::TextColor,
            MarkSpec {
                ..Default::default()
            },
        );
        schema.add_mark(
            MarkType::Link,
            MarkSpec {
                expand: Expand::None,
                persist: false,
            },
        );
        schema.add_mark(
            MarkType::Ruby,
            MarkSpec {
                expand: Expand::None,
                persist: false,
            },
        );

        schema
    }
}
