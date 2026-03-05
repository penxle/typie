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
        assert!(
            !spec.name.is_empty(),
            "Node {:?} must define NodeSpec.name",
            node_type
        );

        if let Some(item_type) = spec.promote_item_type_on_delete {
            assert!(
                spec.content.repeated_single_type() == Some(item_type),
                "Node {:?} has promote_item_type_on_delete={:?}, but content is not repeated single type {:?}",
                node_type,
                item_type,
                item_type
            );
            assert!(
                !spec.inline,
                "Node {:?} cannot set promote_item_type_on_delete when inline=true",
                node_type
            );
        }

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
                name: "root",
                content: content_expr!([((Paragraph | Image | File | Embed | Archived | Blockquote | Callout | BulletList | OrderedList | HorizontalRule | Fold | Table)*), (Paragraph)]),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Blockquote,
            NodeSpec {
                name: "blockquote",
                content: content_expr!((Paragraph | BulletList | OrderedList)+),
                block_selection_boundary_mode: Some(BlockSelectionBoundaryMode::FrontOnly),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Paragraph,
            NodeSpec {
                name: "paragraph",
                content: content_expr!([((Text | HardBreak)*), (PageBreak?)]),
                styles: Some(&[
                    StyleType::BackgroundColor,
                    StyleType::Bold,
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
                name: "text",
                inline: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Image,
            NodeSpec {
                name: "image",
                selectable: true,
                external: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::File,
            NodeSpec {
                name: "file",
                selectable: true,
                external: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Embed,
            NodeSpec {
                name: "embed",
                selectable: true,
                external: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Archived,
            NodeSpec {
                name: "archived",
                selectable: true,
                external: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::HardBreak,
            NodeSpec {
                name: "hard_break",
                inline: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::PageBreak,
            NodeSpec {
                name: "page_break",
                inline: true,
                grandparent_must_be: Some(NodeType::Root),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::HorizontalRule,
            NodeSpec {
                name: "horizontal_rule",
                selectable: true,
                block_selection_boundary_mode: Some(BlockSelectionBoundaryMode::Both),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::BulletList,
            NodeSpec {
                name: "bullet_list",
                content: content_expr!(ListItem+),
                promote_item_type_on_delete: Some(NodeType::ListItem),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::OrderedList,
            NodeSpec {
                name: "ordered_list",
                content: content_expr!(ListItem+),
                promote_item_type_on_delete: Some(NodeType::ListItem),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::ListItem,
            NodeSpec {
                name: "list_item",
                content: content_expr!([(Paragraph), ((BulletList | OrderedList) *)]),
                structural: true,
                block_selection_boundary_mode: Some(BlockSelectionBoundaryMode::FrontOnly),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Fold,
            NodeSpec {
                name: "fold",
                content: content_expr!([(FoldTitle), (FoldContent)]),
                isolating: true,
                block_selection_boundary_mode: Some(BlockSelectionBoundaryMode::FrontOrBack),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::FoldTitle,
            NodeSpec {
                name: "fold_title",
                content: content_expr!(Text*),
                isolating: true,
                structural: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::FoldContent,
            NodeSpec {
                name: "fold_content",
                content: content_expr!((Paragraph | Image | File | Embed | Archived | Blockquote | Callout | BulletList | OrderedList | HorizontalRule | Fold | Table)+),
                isolating: true,
                structural: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Callout,
            NodeSpec {
                name: "callout",
                content: content_expr!((Paragraph | BulletList | OrderedList)+),
                block_selection_boundary_mode: Some(BlockSelectionBoundaryMode::FrontOnly),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::Table,
            NodeSpec {
                name: "table",
                content: content_expr!(TableRow+),
                isolating: true,
                block_selection_boundary_mode: Some(BlockSelectionBoundaryMode::FrontOrBack),
                forbidden_descendants: Some(&[NodeType::Table]),
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::TableRow,
            NodeSpec {
                name: "table_row",
                content: content_expr!(TableCell+),
                structural: true,
                ..Default::default()
            },
        );

        schema.add_node(
            NodeType::TableCell,
            NodeSpec {
                name: "table_cell",
                content: content_expr!((Paragraph | Image | File | Embed | Archived | Blockquote | Callout | BulletList | OrderedList | HorizontalRule | Fold)+),
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
            StyleType::Bold,
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
