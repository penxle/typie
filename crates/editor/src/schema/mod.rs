#[macro_use]
mod content_macro;

mod content;
mod spec;

pub use content::{ContentExpr, RepairAction};
pub use spec::*;

use crate::model::{AnnotationType, NodeType, StyleType};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct Schema {
    nodes: FxHashMap<NodeType, NodeSpec>,
    styles: FxHashMap<StyleType, StyleSpec>,
    annotations: FxHashMap<AnnotationType, AnnotationSpec>,
}

impl Schema {
    pub fn new() -> Self {
        Self {
            nodes: FxHashMap::default(),
            styles: FxHashMap::default(),
            annotations: FxHashMap::default(),
        }
    }

    pub fn add_node(&mut self, node_type: NodeType, spec: NodeSpec) {
        self.nodes.insert(node_type, spec);
    }

    pub fn add_style(&mut self, style_type: StyleType, spec: StyleSpec) {
        self.styles.insert(style_type, spec);
    }

    pub fn add_annotation(&mut self, annotation_type: AnnotationType, spec: AnnotationSpec) {
        self.annotations.insert(annotation_type, spec);
    }

    pub fn node_spec(&self, node_type: NodeType) -> &NodeSpec {
        self.nodes
            .get(&node_type)
            .unwrap_or_else(|| panic!("Unknown node type: {:?}", node_type))
    }

    pub fn annotation_spec(&self, annotation_type: AnnotationType) -> &AnnotationSpec {
        self.annotations
            .get(&annotation_type)
            .unwrap_or_else(|| panic!("Unknown annotation type: {:?}", annotation_type))
    }
}

impl Default for Schema {
    fn default() -> Self {
        let mut schema = Schema::new();

        schema.add_node(
            NodeType::Root,
            NodeSpec {
                content: content_expr!([((Paragraph | Image | File | Embed | Archived | Blockquote | Callout | BulletList | OrderedList | HorizontalRule | Fold | Table)*), (Paragraph)]),
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
                styles: Some(&[
                    StyleType::BackgroundColor,
                    StyleType::FontFamily,
                    StyleType::FontSize,
                    StyleType::FontWeight,
                    StyleType::Italic,
                    StyleType::LetterSpacing,
                    StyleType::Strikethrough,
                    StyleType::TextColor,
                    StyleType::Underline,
                ]),
                annotations: Some(&[AnnotationType::Link, AnnotationType::Ruby]),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Text,
            NodeSpec {
                inline: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Image,
            NodeSpec {
                selectable: true,
                external: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::File,
            NodeSpec {
                selectable: true,
                external: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Embed,
            NodeSpec {
                selectable: true,
                external: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Archived,
            NodeSpec {
                selectable: true,
                external: true,
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
                isolating: true,
                structural: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::FoldContent,
            NodeSpec {
                content: content_expr!((Paragraph | Image | File | Embed | Archived | Blockquote | Callout | BulletList | OrderedList | HorizontalRule | Fold)+),
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
                content: content_expr!((Paragraph | Image | File | Embed | Archived | BulletList | OrderedList)+),
                isolating: true,
                structural: true,
                ..Default::default()
            },
        );

        schema.add_style(
            StyleType::BackgroundColor,
            StyleSpec {
                expand: Expand::After,
            },
        );
        schema.add_style(
            StyleType::FontFamily,
            StyleSpec {
                expand: Expand::After,
            },
        );
        schema.add_style(
            StyleType::FontSize,
            StyleSpec {
                expand: Expand::After,
            },
        );
        schema.add_style(
            StyleType::FontWeight,
            StyleSpec {
                expand: Expand::After,
            },
        );
        schema.add_style(
            StyleType::Italic,
            StyleSpec {
                expand: Expand::After,
            },
        );
        schema.add_style(
            StyleType::LetterSpacing,
            StyleSpec {
                expand: Expand::After,
            },
        );
        schema.add_style(
            StyleType::Strikethrough,
            StyleSpec {
                expand: Expand::After,
            },
        );
        schema.add_style(
            StyleType::TextColor,
            StyleSpec {
                expand: Expand::After,
            },
        );
        schema.add_style(
            StyleType::Underline,
            StyleSpec {
                expand: Expand::After,
            },
        );

        schema.add_annotation(AnnotationType::Link, AnnotationSpec { overlap: false });
        schema.add_annotation(AnnotationType::Ruby, AnnotationSpec { overlap: false });

        schema
    }
}
