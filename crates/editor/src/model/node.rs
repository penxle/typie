use crate::layout::{Layout, LayoutContext, LayoutNode};
use crate::model::html::{DomSpec, NodeHtmlCodec};
use crate::model::*;
use crate::types::BoxConstraints;
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Root,
    Paragraph,
    Blockquote,
    Callout,
    Text,
    Image,
    File,
    Embed,
    HardBreak,
    HorizontalRule,
    PageBreak,
    BulletList,
    OrderedList,
    ListItem,
    Fold,
    FoldTitle,
    FoldContent,
    Table,
    TableRow,
    TableCell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Node {
    Root(RootNode),
    Paragraph(ParagraphNode),
    Blockquote(BlockquoteNode),
    Callout(CalloutNode),
    Image(ImageNode),
    File(FileNode),
    Embed(EmbedNode),
    Text(TextNode),
    HardBreak(HardBreakNode),
    HorizontalRule(HorizontalRuleNode),
    PageBreak(PageBreakNode),
    BulletList(BulletListNode),
    OrderedList(OrderedListNode),
    ListItem(ListItemNode),
    Fold(FoldNode),
    FoldTitle(FoldTitleNode),
    FoldContent(FoldContentNode),
    Table(TableNode),
    TableRow(TableRowNode),
    TableCell(TableCellNode),
}

impl Node {
    pub fn as_type(&self) -> NodeType {
        match self {
            Node::Root(_) => NodeType::Root,
            Node::Paragraph(_) => NodeType::Paragraph,
            Node::Blockquote(_) => NodeType::Blockquote,
            Node::Callout(_) => NodeType::Callout,
            Node::Image(_) => NodeType::Image,
            Node::File(_) => NodeType::File,
            Node::Embed(_) => NodeType::Embed,
            Node::Text(_) => NodeType::Text,
            Node::HardBreak(_) => NodeType::HardBreak,
            Node::HorizontalRule(_) => NodeType::HorizontalRule,
            Node::PageBreak(_) => NodeType::PageBreak,
            Node::BulletList(_) => NodeType::BulletList,
            Node::OrderedList(_) => NodeType::OrderedList,
            Node::ListItem(_) => NodeType::ListItem,
            Node::Fold(_) => NodeType::Fold,
            Node::FoldTitle(_) => NodeType::FoldTitle,
            Node::FoldContent(_) => NodeType::FoldContent,
            Node::Table(_) => NodeType::Table,
            Node::TableRow(_) => NodeType::TableRow,
            Node::TableCell(_) => NodeType::TableCell,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Node::Text(t) => t.text.char_len(),
            _ => 1,
        }
    }

    pub fn plan_consecutive_text_merges<'a>(
        nodes: impl Iterator<Item = (NodeId, &'a Node)>,
    ) -> Vec<(NodeId, Vec<NodeId>, Vec<(String, Vec<Mark>)>)> {
        let mut plans = Vec::new();
        let mut current_merge: Option<(NodeId, Vec<NodeId>, Vec<(String, Vec<Mark>)>)> = None;

        for (id, node) in nodes {
            if let Node::Text(text_node) = node {
                let segments = text_node.text.get_rich_text_segments();
                if let Some(ref mut merge) = current_merge {
                    merge.1.push(id);
                    merge.2.extend(segments);
                } else {
                    current_merge = Some((id, Vec::new(), segments));
                }
            } else {
                if let Some(merge) = current_merge.take() {
                    if !merge.1.is_empty() {
                        plans.push(merge);
                    }
                }
            }
        }

        if let Some(merge) = current_merge.take() {
            if !merge.1.is_empty() {
                plans.push(merge);
            }
        }

        plans
    }
}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Node::Root(n) => n.hash(state),
            Node::Paragraph(n) => n.hash(state),
            Node::Blockquote(n) => n.hash(state),
            Node::Callout(n) => n.hash(state),
            Node::Image(n) => n.hash(state),
            Node::File(n) => n.hash(state),
            Node::Embed(n) => n.hash(state),
            Node::Text(n) => n.hash(state),
            Node::HardBreak(n) => n.hash(state),
            Node::HorizontalRule(n) => n.hash(state),
            Node::PageBreak(n) => n.hash(state),
            Node::BulletList(n) => n.hash(state),
            Node::OrderedList(n) => n.hash(state),
            Node::ListItem(n) => n.hash(state),
            Node::Fold(n) => n.hash(state),
            Node::FoldTitle(n) => n.hash(state),
            Node::FoldContent(n) => n.hash(state),
            Node::Table(n) => n.hash(state),
            Node::TableRow(n) => n.hash(state),
            Node::TableCell(n) => n.hash(state),
        }
    }
}

impl Layout for Node {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        match self {
            Node::Root(node) => node.layout(ctx, constraints),
            Node::Paragraph(node) => node.layout(ctx, constraints),
            Node::Blockquote(node) => node.layout(ctx, constraints),
            Node::Callout(node) => node.layout(ctx, constraints),
            Node::Image(node) => node.layout(ctx, constraints),
            Node::File(node) => node.layout(ctx, constraints),
            Node::Embed(node) => node.layout(ctx, constraints),
            Node::HorizontalRule(node) => node.layout(ctx, constraints),
            Node::PageBreak(node) => node.layout(ctx, constraints),
            Node::BulletList(node) => node.layout(ctx, constraints),
            Node::OrderedList(node) => node.layout(ctx, constraints),
            Node::ListItem(node) => node.layout(ctx, constraints),
            Node::Fold(node) => node.layout(ctx, constraints),
            Node::FoldTitle(node) => node.layout(ctx, constraints),
            Node::FoldContent(node) => node.layout(ctx, constraints),
            Node::Table(node) => node.layout(ctx, constraints),
            Node::TableRow(node) => node.layout(ctx, constraints),
            Node::TableCell(node) => node.layout(ctx, constraints),
            _ => panic!("Unsupported node type"),
        }
    }
}

impl NodeHtmlCodec for Node {
    fn to_dom(&self) -> Option<DomSpec> {
        match self {
            Node::Root(node) => node.to_dom(),
            Node::Paragraph(node) => node.to_dom(),
            Node::Blockquote(node) => node.to_dom(),
            Node::Callout(node) => node.to_dom(),
            Node::Image(node) => node.to_dom(),
            Node::File(node) => node.to_dom(),
            Node::Embed(node) => node.to_dom(),
            Node::Text(node) => node.to_dom(),
            Node::HardBreak(node) => node.to_dom(),
            Node::HorizontalRule(node) => node.to_dom(),
            Node::PageBreak(node) => node.to_dom(),
            Node::BulletList(node) => node.to_dom(),
            Node::OrderedList(node) => node.to_dom(),
            Node::ListItem(node) => node.to_dom(),
            Node::Fold(node) => node.to_dom(),
            Node::FoldTitle(node) => node.to_dom(),
            Node::FoldContent(node) => node.to_dom(),
            Node::Table(node) => node.to_dom(),
            Node::TableRow(node) => node.to_dom(),
            Node::TableCell(node) => node.to_dom(),
        }
    }
}
