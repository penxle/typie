use editor_crdt::{Changeset, Dot, ListOp, Op};
use editor_model::{
    AliasOp, AliasRun, Alignment, AtomLeaf, EditOp, LayoutMode, Modifier, ModifierAttrOp,
    ModifierType, NodeAttr, NodeAttrOp, NodeType, SeqClass, SeqItem, SpanOp, alias_op_is_valid,
    classify,
};

use crate::bundle::{
    BundleChangeset, BundleRecord, RecordPayload, decode_bundle_with_ctx, encode_bundle,
};
use crate::ctx::EncCtx;
use crate::durable::Durable;
use crate::envelope::unwrap_one;
use crate::error::{CodecResult, Corruption, EncodeInvariant};
use crate::framing::{UnknownTail, read_open_variant};
use crate::types::*;

pub struct ReencodableChangesets(Vec<Changeset<EditOp>>);

impl ReencodableChangesets {
    pub fn from_local_ops(css: Vec<Changeset<EditOp>>) -> Self {
        Self(css)
    }

    /// Repackages an extract that already passed the `into_reencodable()` gate; the caller vouches that `css` is lossless-decoded, not freshly authored.
    pub fn from_verified(css: Vec<Changeset<EditOp>>) -> Self {
        Self(css)
    }

    pub fn as_slice(&self) -> &[Changeset<EditOp>] {
        &self.0
    }

    pub fn retain(&mut self, f: impl FnMut(&Changeset<EditOp>) -> bool) {
        self.0.retain(f);
    }

    pub fn concat(parts: Vec<Self>) -> Self {
        Self(parts.into_iter().flat_map(|p| p.0).collect())
    }
}

pub struct Decoded {
    changesets: Vec<Changeset<EditOp>>,
    lossless: bool,
}

impl Decoded {
    pub fn into_graph_input(self) -> Vec<Changeset<EditOp>> {
        self.changesets
    }

    pub fn into_reencodable(self) -> CodecResult<ReencodableChangesets> {
        if self.lossless {
            Ok(ReencodableChangesets(self.changesets))
        } else {
            Err(crate::error::Fenced::LossyForReencode.into())
        }
    }
}

fn no_tail() -> UnknownTail {
    UnknownTail(Vec::new())
}

fn to_durable_node_type(nt: NodeType) -> DurableNodeType {
    match nt {
        NodeType::Unknown => unreachable!("sealed by to_durable_item"),
        NodeType::Root => DurableNodeType::Root,
        NodeType::Paragraph => DurableNodeType::Paragraph,
        NodeType::Blockquote => DurableNodeType::Blockquote,
        NodeType::Callout => DurableNodeType::Callout,
        NodeType::Text => DurableNodeType::Text,
        NodeType::BulletList => DurableNodeType::BulletList,
        NodeType::OrderedList => DurableNodeType::OrderedList,
        NodeType::ListItem => DurableNodeType::ListItem,
        NodeType::Fold => DurableNodeType::Fold,
        NodeType::FoldTitle => DurableNodeType::FoldTitle,
        NodeType::FoldContent => DurableNodeType::FoldContent,
        NodeType::Table => DurableNodeType::Table,
        NodeType::TableRow => DurableNodeType::TableRow,
        NodeType::TableCell => DurableNodeType::TableCell,
        NodeType::Image => DurableNodeType::Image,
        NodeType::File => DurableNodeType::File,
        NodeType::Embed => DurableNodeType::Embed,
        NodeType::Archived => DurableNodeType::Archived,
        NodeType::HardBreak => DurableNodeType::HardBreak,
        NodeType::HorizontalRule => DurableNodeType::HorizontalRule,
        NodeType::PageBreak => DurableNodeType::PageBreak,
        NodeType::Tab => DurableNodeType::Tab,
    }
}

fn to_durable_layout(m: &LayoutMode) -> DurableLayoutMode {
    match m {
        LayoutMode::Paginated {
            page_width,
            page_height,
            page_margin_top,
            page_margin_bottom,
            page_margin_left,
            page_margin_right,
        } => DurableLayoutMode::Paginated {
            page_width: *page_width,
            page_height: *page_height,
            page_margin_top: *page_margin_top,
            page_margin_bottom: *page_margin_bottom,
            page_margin_left: *page_margin_left,
            page_margin_right: *page_margin_right,
            tail: no_tail(),
        },
        LayoutMode::Continuous { max_width } => DurableLayoutMode::Continuous {
            max_width: *max_width,
            tail: no_tail(),
        },
    }
}

fn to_durable_attr(attr: &NodeAttr) -> DurableAttr {
    use editor_model::{
        ArchivedNodeAttr, BlockquoteNodeAttr, CalloutNodeAttr, EmbedNodeAttr, FileNodeAttr,
        HorizontalRuleNodeAttr, ImageNodeAttr, RootNodeAttr, TableCellNodeAttr, TableNodeAttr,
    };
    match attr {
        NodeAttr::Root { attr } => match attr {
            RootNodeAttr::LayoutMode(m) => DurableAttr::RootLayoutMode(to_durable_layout(m)),
        },
        NodeAttr::Blockquote { attr } => match attr {
            BlockquoteNodeAttr::Variant(v) => DurableAttr::BlockquoteVariant(match v {
                editor_model::BlockquoteVariant::LeftLine => DurableBlockquoteVariant::LeftLine,
                editor_model::BlockquoteVariant::LeftQuote => DurableBlockquoteVariant::LeftQuote,
                editor_model::BlockquoteVariant::MessageSent => {
                    DurableBlockquoteVariant::MessageSent
                }
                editor_model::BlockquoteVariant::MessageReceived => {
                    DurableBlockquoteVariant::MessageReceived
                }
            }),
        },
        NodeAttr::Callout { attr } => match attr {
            CalloutNodeAttr::Variant(v) => DurableAttr::CalloutVariant(match v {
                editor_model::CalloutVariant::Info => DurableCalloutVariant::Info,
                editor_model::CalloutVariant::Success => DurableCalloutVariant::Success,
                editor_model::CalloutVariant::Warning => DurableCalloutVariant::Warning,
                editor_model::CalloutVariant::Danger => DurableCalloutVariant::Danger,
            }),
        },
        NodeAttr::Table { attr } => match attr {
            TableNodeAttr::BorderStyle(s) => DurableAttr::TableBorderStyle(match s {
                editor_model::TableBorderStyle::Solid => DurableTableBorderStyle::Solid,
                editor_model::TableBorderStyle::Dashed => DurableTableBorderStyle::Dashed,
                editor_model::TableBorderStyle::Dotted => DurableTableBorderStyle::Dotted,
                editor_model::TableBorderStyle::None => DurableTableBorderStyle::None,
            }),
            TableNodeAttr::Proportion(p) => DurableAttr::TableProportion(*p),
        },
        NodeAttr::TableCell { attr } => match attr {
            TableCellNodeAttr::ColWidth(w) => DurableAttr::TableCellColWidth(*w),
            TableCellNodeAttr::BackgroundColor(c) => {
                DurableAttr::TableCellBackgroundColor(c.clone())
            }
        },
        NodeAttr::Image { attr } => match attr {
            ImageNodeAttr::Id(id) => DurableAttr::ImageId(id.clone()),
            ImageNodeAttr::Proportion(p) => DurableAttr::ImageProportion(*p),
        },
        NodeAttr::File { attr } => match attr {
            FileNodeAttr::Id(id) => DurableAttr::FileId(id.clone()),
        },
        NodeAttr::Embed { attr } => match attr {
            EmbedNodeAttr::Id(id) => DurableAttr::EmbedId(id.clone()),
        },
        NodeAttr::Archived { attr } => match attr {
            ArchivedNodeAttr::Id(id) => DurableAttr::ArchivedId(id.clone()),
        },
        NodeAttr::HorizontalRule { attr } => match attr {
            HorizontalRuleNodeAttr::Variant(v) => DurableAttr::HorizontalRuleVariant(match v {
                editor_model::HorizontalRuleVariant::Line => DurableHorizontalRuleVariant::Line,
                editor_model::HorizontalRuleVariant::DashedLine => {
                    DurableHorizontalRuleVariant::DashedLine
                }
                editor_model::HorizontalRuleVariant::CircleLine => {
                    DurableHorizontalRuleVariant::CircleLine
                }
                editor_model::HorizontalRuleVariant::DiamondLine => {
                    DurableHorizontalRuleVariant::DiamondLine
                }
                editor_model::HorizontalRuleVariant::Circle => DurableHorizontalRuleVariant::Circle,
                editor_model::HorizontalRuleVariant::Diamond => {
                    DurableHorizontalRuleVariant::Diamond
                }
                editor_model::HorizontalRuleVariant::ThreeCircles => {
                    DurableHorizontalRuleVariant::ThreeCircles
                }
                editor_model::HorizontalRuleVariant::ThreeDiamonds => {
                    DurableHorizontalRuleVariant::ThreeDiamonds
                }
                editor_model::HorizontalRuleVariant::Zigzag => DurableHorizontalRuleVariant::Zigzag,
            }),
        },
        NodeAttr::Unknown { tag, bytes } => DurableAttr::Unknown(crate::framing::UnknownPayload {
            tag: *tag,
            bytes: bytes.clone(),
        }),
        NodeAttr::Paragraph { attr } => match *attr {},
        NodeAttr::Text { attr } => match *attr {},
        NodeAttr::BulletList { attr } => match *attr {},
        NodeAttr::OrderedList { attr } => match *attr {},
        NodeAttr::ListItem { attr } => match *attr {},
        NodeAttr::Fold { attr } => match *attr {},
        NodeAttr::FoldTitle { attr } => match *attr {},
        NodeAttr::FoldContent { attr } => match *attr {},
        NodeAttr::TableRow { attr } => match *attr {},
        NodeAttr::HardBreak { attr } => match *attr {},
        NodeAttr::PageBreak { attr } => match *attr {},
        NodeAttr::Tab { attr } => match *attr {},
    }
}

fn to_durable_modifier(m: &Modifier) -> DurableModifier {
    match m {
        Modifier::Bold => DurableModifier::Bold,
        Modifier::Italic => DurableModifier::Italic,
        Modifier::Underline => DurableModifier::Underline,
        Modifier::Strikethrough => DurableModifier::Strikethrough,
        Modifier::FontSize { value } => DurableModifier::FontSize(*value),
        Modifier::FontFamily { value } => DurableModifier::FontFamily(value.clone()),
        Modifier::FontWeight { value } => DurableModifier::FontWeight(*value),
        Modifier::TextColor { value } => DurableModifier::TextColor(value.clone()),
        Modifier::BackgroundColor { value } => DurableModifier::BackgroundColor(value.clone()),
        Modifier::LetterSpacing { value } => DurableModifier::LetterSpacing(*value),
        Modifier::Link { href } => DurableModifier::Link(href.clone()),
        Modifier::Ruby { text } => DurableModifier::Ruby(text.clone()),
        Modifier::LineHeight { value } => DurableModifier::LineHeight(*value),
        Modifier::BlockGap { value } => DurableModifier::BlockGap(*value),
        Modifier::ParagraphIndent { value } => DurableModifier::ParagraphIndent(*value),
        Modifier::Alignment { value } => DurableModifier::Alignment(match value {
            Alignment::Left => DurableAlignment::Left,
            Alignment::Center => DurableAlignment::Center,
            Alignment::Right => DurableAlignment::Right,
            Alignment::Justify => DurableAlignment::Justify,
        }),
    }
}

fn to_durable_kind(k: &ModifierType) -> DurableModifierKind {
    match k {
        ModifierType::Bold => DurableModifierKind::Bold,
        ModifierType::Italic => DurableModifierKind::Italic,
        ModifierType::Underline => DurableModifierKind::Underline,
        ModifierType::Strikethrough => DurableModifierKind::Strikethrough,
        ModifierType::FontSize => DurableModifierKind::FontSize,
        ModifierType::FontFamily => DurableModifierKind::FontFamily,
        ModifierType::FontWeight => DurableModifierKind::FontWeight,
        ModifierType::TextColor => DurableModifierKind::TextColor,
        ModifierType::BackgroundColor => DurableModifierKind::BackgroundColor,
        ModifierType::LetterSpacing => DurableModifierKind::LetterSpacing,
        ModifierType::Link => DurableModifierKind::Link,
        ModifierType::Ruby => DurableModifierKind::Ruby,
        ModifierType::LineHeight => DurableModifierKind::LineHeight,
        ModifierType::BlockGap => DurableModifierKind::BlockGap,
        ModifierType::ParagraphIndent => DurableModifierKind::ParagraphIndent,
        ModifierType::Alignment => DurableModifierKind::Alignment,
    }
}

fn to_durable_alias_run(run: &AliasRun) -> DurableAliasRun {
    DurableAliasRun {
        old_start: run.old_start,
        len: u64::from(run.len),
        new_start: run.new_start,
    }
}

fn to_durable_anchor(a: &editor_model::Anchor) -> DurableAnchor {
    DurableAnchor {
        id: a.id,
        bias: match a.bias {
            editor_model::Bias::Before => DurableBias::Before,
            editor_model::Bias::After => DurableBias::After,
        },
    }
}

fn atom_init(leaf: &AtomLeaf) -> Vec<DurableAttr> {
    leaf.clone()
        .into_node()
        .to_plain()
        .to_attrs()
        .iter()
        .map(to_durable_attr)
        .collect()
}

fn to_durable_item(item: &SeqItem) -> CodecResult<DurableItem> {
    if matches!(
        item,
        SeqItem::Block {
            node_type: NodeType::Unknown,
            ..
        } | SeqItem::BlockAtom {
            leaf: AtomLeaf::Unknown(_),
            ..
        } | SeqItem::Atom(AtomLeaf::Unknown(_))
    ) {
        return Err(EncodeInvariant::UnknownPayloadEncode.into());
    }
    Ok(match item {
        SeqItem::Char(c) => DurableItem::Char(*c),
        SeqItem::Atom(leaf) => DurableItem::Atom {
            node_type: to_durable_node_type(leaf.node_type()),
            init: atom_init(leaf),
            tail: no_tail(),
        },
        SeqItem::Block {
            node_type,
            parents,
            attrs,
        } => DurableItem::Block {
            node_type: to_durable_node_type(*node_type),
            parents: parents.clone(),
            init: attrs.iter().map(to_durable_attr).collect(),
            tail: no_tail(),
        },
        SeqItem::BlockAtom { leaf, parents } => DurableItem::BlockAtom {
            node_type: to_durable_node_type(leaf.node_type()),
            parents: parents.clone(),
            init: atom_init(leaf),
            tail: no_tail(),
        },
        SeqItem::Unknown { .. } => return Err(EncodeInvariant::UnknownPayloadEncode.into()),
    })
}

pub fn to_durable_op(op: &EditOp) -> CodecResult<DurableOp> {
    Ok(match op {
        EditOp::Seq(ListOp::Ins { pos, item }) => DurableOp::SeqIns {
            pos: *pos as u64,
            item: to_durable_item(item)?,
        },
        EditOp::Seq(ListOp::Del { pos, len }) => DurableOp::SeqDel {
            pos: *pos as u64,
            len: *len as u64,
        },
        EditOp::Seq(ListOp::Undel { del }) => DurableOp::SeqUndel { del: *del },
        EditOp::Span(SpanOp::AddSpan {
            start,
            end,
            modifier,
        }) => DurableOp::AddSpan {
            start: to_durable_anchor(start),
            end: to_durable_anchor(end),
            modifier: to_durable_modifier(modifier),
            tail: no_tail(),
        },
        EditOp::Span(SpanOp::RemoveSpan {
            start,
            end,
            modifier_type,
        }) => DurableOp::RemoveSpan {
            start: to_durable_anchor(start),
            end: to_durable_anchor(end),
            kind: to_durable_kind(modifier_type),
            tail: no_tail(),
        },
        EditOp::BlockModifier(ModifierAttrOp::SetModifier { target, modifier }) => {
            DurableOp::SetBlockModifier {
                target: *target,
                modifier: to_durable_modifier(modifier),
                tail: no_tail(),
            }
        }
        EditOp::BlockModifier(ModifierAttrOp::ClearModifier { target, key }) => {
            DurableOp::ClearBlockModifier {
                target: *target,
                kind: to_durable_kind(key),
                tail: no_tail(),
            }
        }
        EditOp::NodeCarry(ModifierAttrOp::SetModifier { target, modifier }) => {
            DurableOp::SetNodeCarry {
                target: *target,
                modifier: to_durable_modifier(modifier),
                tail: no_tail(),
            }
        }
        EditOp::NodeCarry(ModifierAttrOp::ClearModifier { target, key }) => {
            DurableOp::ClearNodeCarry {
                target: *target,
                kind: to_durable_kind(key),
                tail: no_tail(),
            }
        }
        EditOp::NodeAttr(NodeAttrOp { target, attr }) => DurableOp::SetNodeAttr {
            target: *target,
            attr: to_durable_attr(attr),
            tail: no_tail(),
        },
        EditOp::Alias(op) => {
            if !alias_op_is_valid(op) {
                return Err(EncodeInvariant::InvalidAliasOp.into());
            }
            DurableOp::AliasDots {
                pairs: op.pairs.iter().map(to_durable_alias_run).collect(),
                tail: no_tail(),
            }
        }
        EditOp::Unknown { .. } => return Err(EncodeInvariant::UnknownPayloadEncode.into()),
    })
}

pub fn encode_changesets(css: ReencodableChangesets) -> CodecResult<Vec<u8>> {
    let css = css.0;
    let mut bundles = Vec::with_capacity(css.len());
    for cs in &css {
        let mut records = Vec::with_capacity(cs.ops.len());
        for op in &cs.ops {
            records.push(BundleRecord {
                id: op.id,
                parents: op.parents.clone(),
                payload: RecordPayload::Known(to_durable_op(&op.payload)?),
                record_tail: Vec::new(),
            });
        }
        bundles.push(BundleChangeset { records });
    }
    encode_bundle(&bundles)
}

pub fn changesets_contain_unknown(css: &[Changeset<EditOp>]) -> bool {
    css.iter().any(|cs| {
        cs.ops.iter().any(|op| match &op.payload {
            EditOp::Unknown { .. } => true,
            EditOp::Seq(ListOp::Ins { item, .. }) => matches!(
                item,
                SeqItem::Unknown { .. }
                    | SeqItem::Block {
                        node_type: NodeType::Unknown,
                        ..
                    }
                    | SeqItem::BlockAtom {
                        leaf: AtomLeaf::Unknown(_),
                        ..
                    }
                    | SeqItem::Atom(AtomLeaf::Unknown(_))
            ),
            EditOp::Seq(_)
            | EditOp::Span(_)
            | EditOp::BlockModifier(_)
            | EditOp::NodeCarry(_)
            | EditOp::NodeAttr(_)
            | EditOp::Alias(_) => false,
        })
    })
}

struct Unrepresentable;

fn from_durable_node_type(nt: &DurableNodeType) -> Result<NodeType, Unrepresentable> {
    Ok(match nt {
        DurableNodeType::Root => NodeType::Root,
        DurableNodeType::Paragraph => NodeType::Paragraph,
        DurableNodeType::Blockquote => NodeType::Blockquote,
        DurableNodeType::Callout => NodeType::Callout,
        DurableNodeType::Text => NodeType::Text,
        DurableNodeType::BulletList => NodeType::BulletList,
        DurableNodeType::OrderedList => NodeType::OrderedList,
        DurableNodeType::ListItem => NodeType::ListItem,
        DurableNodeType::Fold => NodeType::Fold,
        DurableNodeType::FoldTitle => NodeType::FoldTitle,
        DurableNodeType::FoldContent => NodeType::FoldContent,
        DurableNodeType::Table => NodeType::Table,
        DurableNodeType::TableRow => NodeType::TableRow,
        DurableNodeType::TableCell => NodeType::TableCell,
        DurableNodeType::Image => NodeType::Image,
        DurableNodeType::File => NodeType::File,
        DurableNodeType::Embed => NodeType::Embed,
        DurableNodeType::Archived => NodeType::Archived,
        DurableNodeType::HardBreak => NodeType::HardBreak,
        DurableNodeType::HorizontalRule => NodeType::HorizontalRule,
        DurableNodeType::PageBreak => NodeType::PageBreak,
        DurableNodeType::Tab => NodeType::Tab,
        DurableNodeType::Unknown(_) => return Err(Unrepresentable),
    })
}

fn from_durable_layout(m: &DurableLayoutMode) -> Result<LayoutMode, Unrepresentable> {
    Ok(match m {
        DurableLayoutMode::Paginated {
            page_width,
            page_height,
            page_margin_top,
            page_margin_bottom,
            page_margin_left,
            page_margin_right,
            tail,
        } => {
            if !tail.0.is_empty() {
                return Err(Unrepresentable);
            }
            LayoutMode::Paginated {
                page_width: *page_width,
                page_height: *page_height,
                page_margin_top: *page_margin_top,
                page_margin_bottom: *page_margin_bottom,
                page_margin_left: *page_margin_left,
                page_margin_right: *page_margin_right,
            }
        }
        DurableLayoutMode::Continuous { max_width, tail } => {
            if !tail.0.is_empty() {
                return Err(Unrepresentable);
            }
            LayoutMode::Continuous {
                max_width: *max_width,
            }
        }
        DurableLayoutMode::Unknown(_) => return Err(Unrepresentable),
    })
}

fn from_durable_blockquote_variant(
    v: &DurableBlockquoteVariant,
) -> Result<editor_model::BlockquoteVariant, Unrepresentable> {
    Ok(match v {
        DurableBlockquoteVariant::LeftLine => editor_model::BlockquoteVariant::LeftLine,
        DurableBlockquoteVariant::LeftQuote => editor_model::BlockquoteVariant::LeftQuote,
        DurableBlockquoteVariant::MessageSent => editor_model::BlockquoteVariant::MessageSent,
        DurableBlockquoteVariant::MessageReceived => {
            editor_model::BlockquoteVariant::MessageReceived
        }
        DurableBlockquoteVariant::Unknown(_) => return Err(Unrepresentable),
    })
}

fn from_durable_callout_variant(
    v: &DurableCalloutVariant,
) -> Result<editor_model::CalloutVariant, Unrepresentable> {
    Ok(match v {
        DurableCalloutVariant::Info => editor_model::CalloutVariant::Info,
        DurableCalloutVariant::Success => editor_model::CalloutVariant::Success,
        DurableCalloutVariant::Warning => editor_model::CalloutVariant::Warning,
        DurableCalloutVariant::Danger => editor_model::CalloutVariant::Danger,
        DurableCalloutVariant::Unknown(_) => return Err(Unrepresentable),
    })
}

fn from_durable_table_border_style(
    v: &DurableTableBorderStyle,
) -> Result<editor_model::TableBorderStyle, Unrepresentable> {
    Ok(match v {
        DurableTableBorderStyle::Solid => editor_model::TableBorderStyle::Solid,
        DurableTableBorderStyle::Dashed => editor_model::TableBorderStyle::Dashed,
        DurableTableBorderStyle::Dotted => editor_model::TableBorderStyle::Dotted,
        DurableTableBorderStyle::None => editor_model::TableBorderStyle::None,
        DurableTableBorderStyle::Unknown(_) => return Err(Unrepresentable),
    })
}

fn from_durable_horizontal_rule_variant(
    v: &DurableHorizontalRuleVariant,
) -> Result<editor_model::HorizontalRuleVariant, Unrepresentable> {
    Ok(match v {
        DurableHorizontalRuleVariant::Line => editor_model::HorizontalRuleVariant::Line,
        DurableHorizontalRuleVariant::DashedLine => editor_model::HorizontalRuleVariant::DashedLine,
        DurableHorizontalRuleVariant::CircleLine => editor_model::HorizontalRuleVariant::CircleLine,
        DurableHorizontalRuleVariant::DiamondLine => {
            editor_model::HorizontalRuleVariant::DiamondLine
        }
        DurableHorizontalRuleVariant::Circle => editor_model::HorizontalRuleVariant::Circle,
        DurableHorizontalRuleVariant::Diamond => editor_model::HorizontalRuleVariant::Diamond,
        DurableHorizontalRuleVariant::ThreeCircles => {
            editor_model::HorizontalRuleVariant::ThreeCircles
        }
        DurableHorizontalRuleVariant::ThreeDiamonds => {
            editor_model::HorizontalRuleVariant::ThreeDiamonds
        }
        DurableHorizontalRuleVariant::Zigzag => editor_model::HorizontalRuleVariant::Zigzag,
        DurableHorizontalRuleVariant::Unknown(_) => return Err(Unrepresentable),
    })
}

fn from_durable_modifier(m: &DurableModifier) -> Result<Modifier, Unrepresentable> {
    Ok(match m {
        DurableModifier::Bold => Modifier::Bold,
        DurableModifier::Italic => Modifier::Italic,
        DurableModifier::Underline => Modifier::Underline,
        DurableModifier::Strikethrough => Modifier::Strikethrough,
        DurableModifier::FontSize(v) => Modifier::FontSize { value: *v },
        DurableModifier::FontFamily(v) => Modifier::FontFamily { value: v.clone() },
        DurableModifier::FontWeight(v) => Modifier::FontWeight { value: *v },
        DurableModifier::TextColor(v) => Modifier::TextColor { value: v.clone() },
        DurableModifier::BackgroundColor(v) => Modifier::BackgroundColor { value: v.clone() },
        DurableModifier::LetterSpacing(v) => Modifier::LetterSpacing { value: *v },
        DurableModifier::Link(v) => Modifier::Link { href: v.clone() },
        DurableModifier::Ruby(v) => Modifier::Ruby { text: v.clone() },
        DurableModifier::LineHeight(v) => Modifier::LineHeight { value: *v },
        DurableModifier::BlockGap(v) => Modifier::BlockGap { value: *v },
        DurableModifier::ParagraphIndent(v) => Modifier::ParagraphIndent { value: *v },
        DurableModifier::Alignment(a) => Modifier::Alignment {
            value: match a {
                DurableAlignment::Left => Alignment::Left,
                DurableAlignment::Center => Alignment::Center,
                DurableAlignment::Right => Alignment::Right,
                DurableAlignment::Justify => Alignment::Justify,
                DurableAlignment::Unknown(_) => return Err(Unrepresentable),
            },
        },
        DurableModifier::Unknown(_) => return Err(Unrepresentable),
    })
}

fn from_durable_kind(k: &DurableModifierKind) -> Result<ModifierType, Unrepresentable> {
    Ok(match k {
        DurableModifierKind::Bold => ModifierType::Bold,
        DurableModifierKind::Italic => ModifierType::Italic,
        DurableModifierKind::Underline => ModifierType::Underline,
        DurableModifierKind::Strikethrough => ModifierType::Strikethrough,
        DurableModifierKind::FontSize => ModifierType::FontSize,
        DurableModifierKind::FontFamily => ModifierType::FontFamily,
        DurableModifierKind::FontWeight => ModifierType::FontWeight,
        DurableModifierKind::TextColor => ModifierType::TextColor,
        DurableModifierKind::BackgroundColor => ModifierType::BackgroundColor,
        DurableModifierKind::LetterSpacing => ModifierType::LetterSpacing,
        DurableModifierKind::Link => ModifierType::Link,
        DurableModifierKind::Ruby => ModifierType::Ruby,
        DurableModifierKind::LineHeight => ModifierType::LineHeight,
        DurableModifierKind::BlockGap => ModifierType::BlockGap,
        DurableModifierKind::ParagraphIndent => ModifierType::ParagraphIndent,
        DurableModifierKind::Alignment => ModifierType::Alignment,
        DurableModifierKind::Unknown(_) => return Err(Unrepresentable),
    })
}

fn from_durable_anchor(a: &DurableAnchor) -> editor_model::Anchor {
    editor_model::Anchor {
        id: a.id,
        bias: match a.bias {
            DurableBias::Before => editor_model::Bias::Before,
            DurableBias::After => editor_model::Bias::After,
        },
    }
}

fn from_durable_attr(attr: &DurableAttr) -> NodeAttr {
    use editor_model::{
        ArchivedNodeAttr, BlockquoteNodeAttr, CalloutNodeAttr, EmbedNodeAttr, FileNodeAttr,
        HorizontalRuleNodeAttr, ImageNodeAttr, RootNodeAttr, TableCellNodeAttr, TableNodeAttr,
    };

    fn as_unknown(attr: &DurableAttr) -> NodeAttr {
        let enc = EncCtx::from_parts(&[], vec![]).expect("empty ctx");
        let mut bytes = Vec::new();
        attr.encode(&enc, &mut bytes)
            .expect("attr encode cannot fail: no dots");
        let mut slice = &bytes[..];
        let (tag, body) = read_open_variant(&mut slice).expect("own encoding");
        NodeAttr::Unknown {
            tag,
            bytes: body.to_vec(),
        }
    }

    match attr {
        DurableAttr::Unknown(u) => NodeAttr::Unknown {
            tag: u.tag,
            bytes: u.bytes.clone(),
        },
        DurableAttr::RootLayoutMode(m) => match from_durable_layout(m) {
            Ok(v) => NodeAttr::Root {
                attr: RootNodeAttr::LayoutMode(v),
            },
            Err(Unrepresentable) => as_unknown(attr),
        },
        DurableAttr::BlockquoteVariant(v) => match from_durable_blockquote_variant(v) {
            Ok(v) => NodeAttr::Blockquote {
                attr: BlockquoteNodeAttr::Variant(v),
            },
            Err(Unrepresentable) => as_unknown(attr),
        },
        DurableAttr::CalloutVariant(v) => match from_durable_callout_variant(v) {
            Ok(v) => NodeAttr::Callout {
                attr: CalloutNodeAttr::Variant(v),
            },
            Err(Unrepresentable) => as_unknown(attr),
        },
        DurableAttr::TableBorderStyle(v) => match from_durable_table_border_style(v) {
            Ok(v) => NodeAttr::Table {
                attr: TableNodeAttr::BorderStyle(v),
            },
            Err(Unrepresentable) => as_unknown(attr),
        },
        DurableAttr::TableProportion(p) => NodeAttr::Table {
            attr: TableNodeAttr::Proportion(*p),
        },
        DurableAttr::TableCellColWidth(w) => NodeAttr::TableCell {
            attr: TableCellNodeAttr::ColWidth(*w),
        },
        DurableAttr::TableCellBackgroundColor(c) => NodeAttr::TableCell {
            attr: TableCellNodeAttr::BackgroundColor(c.clone()),
        },
        DurableAttr::ImageId(id) => NodeAttr::Image {
            attr: ImageNodeAttr::Id(id.clone()),
        },
        DurableAttr::ImageProportion(p) => NodeAttr::Image {
            attr: ImageNodeAttr::Proportion(*p),
        },
        DurableAttr::FileId(id) => NodeAttr::File {
            attr: FileNodeAttr::Id(id.clone()),
        },
        DurableAttr::EmbedId(id) => NodeAttr::Embed {
            attr: EmbedNodeAttr::Id(id.clone()),
        },
        DurableAttr::ArchivedId(id) => NodeAttr::Archived {
            attr: ArchivedNodeAttr::Id(id.clone()),
        },
        DurableAttr::HorizontalRuleVariant(v) => match from_durable_horizontal_rule_variant(v) {
            Ok(v) => NodeAttr::HorizontalRule {
                attr: HorizontalRuleNodeAttr::Variant(v),
            },
            Err(Unrepresentable) => as_unknown(attr),
        },
    }
}

fn from_durable_item(item: &DurableItem) -> Result<(SeqItem, bool), Unrepresentable> {
    fn atom_leaf(
        nt: &DurableNodeType,
        init: &[DurableAttr],
    ) -> Result<(AtomLeaf, bool), Unrepresentable> {
        let node_type = from_durable_node_type(nt)?;
        let mut node = node_type.into_node();
        let mut dropped = false;
        for attr in init {
            let attr = from_durable_attr(attr);
            if matches!(attr, NodeAttr::Unknown { .. })
                || node.apply_attr(Dot::new(0, 0), &attr).is_err()
            {
                dropped = true;
            }
        }
        let leaf = AtomLeaf::from_node(node).ok_or(Unrepresentable)?;
        Ok((leaf, dropped))
    }

    Ok(match item {
        DurableItem::Char(c) => (SeqItem::Char(*c), false),
        DurableItem::Atom {
            node_type,
            init,
            tail,
        } => {
            let (leaf, dropped) = atom_leaf(node_type, init)?;
            if leaf.is_block_level() {
                return Err(Unrepresentable);
            }
            (SeqItem::Atom(leaf), dropped || !tail.0.is_empty())
        }
        DurableItem::Block {
            node_type,
            parents,
            init,
            tail,
        } => {
            let (node_type, nt_unknown) = match from_durable_node_type(node_type) {
                Ok(nt) => (nt, false),
                Err(Unrepresentable) => (NodeType::Unknown, true),
            };
            if !nt_unknown && classify(node_type) != SeqClass::Block {
                return Err(Unrepresentable);
            }
            (
                SeqItem::Block {
                    node_type,
                    parents: parents.clone(),
                    attrs: init.iter().map(from_durable_attr).collect(),
                },
                nt_unknown || !tail.0.is_empty(),
            )
        }
        DurableItem::BlockAtom {
            node_type,
            parents,
            init,
            tail,
        } => {
            if from_durable_node_type(node_type).is_err() {
                return Ok((
                    SeqItem::BlockAtom {
                        leaf: AtomLeaf::Unknown(editor_model::UnknownNode),
                        parents: parents.clone(),
                    },
                    true,
                ));
            }
            let (leaf, dropped) = atom_leaf(node_type, init)?;
            if !leaf.is_block_level() {
                return Err(Unrepresentable);
            }
            (
                SeqItem::BlockAtom {
                    leaf,
                    parents: parents.clone(),
                },
                dropped || !tail.0.is_empty(),
            )
        }
        DurableItem::Unknown(_) => return Err(Unrepresentable),
    })
}

fn item_as_unknown(item: &DurableItem, enc: &EncCtx) -> CodecResult<SeqItem> {
    let mut bytes = Vec::new();
    item.encode(enc, &mut bytes)?;
    let mut slice = &bytes[..];
    let (tag, body) = read_open_variant(&mut slice)?;
    Ok(SeqItem::Unknown {
        tag,
        bytes: body.to_vec(),
    })
}

fn from_durable_alias_op(pairs: &[DurableAliasRun]) -> CodecResult<AliasOp> {
    let mut runtime_pairs = Vec::with_capacity(pairs.len());
    for run in pairs {
        if run.len == 0 {
            return Err(Corruption::InvalidAliasOp.into());
        }
        let len = u32::try_from(run.len).map_err(|_| Corruption::VarintOverflow)?;
        runtime_pairs.push(AliasRun {
            old_start: run.old_start,
            len,
            new_start: run.new_start,
        });
    }
    let op = AliasOp {
        pairs: runtime_pairs,
    };
    if alias_op_is_valid(&op) {
        Ok(op)
    } else {
        Err(Corruption::InvalidAliasOp.into())
    }
}

fn op_as_unknown(op: &DurableOp, enc: &EncCtx, record_tail: &[u8]) -> CodecResult<EditOp> {
    let mut bytes = Vec::new();
    op.encode(enc, &mut bytes)?;
    bytes.extend_from_slice(record_tail);
    Ok(EditOp::Unknown { bytes })
}

fn pos_usize(pos: u64) -> CodecResult<usize> {
    usize::try_from(pos).map_err(|_| Corruption::VarintOverflow.into())
}

fn from_durable_op(
    op: &DurableOp,
    enc: &EncCtx,
    record_tail: &[u8],
    lossy: &mut bool,
) -> CodecResult<EditOp> {
    Ok(match op {
        DurableOp::SeqIns { pos, item } => {
            let runtime_item = match from_durable_item(item) {
                Ok((i, dropped)) => {
                    if dropped {
                        *lossy = true;
                    }
                    i
                }
                Err(Unrepresentable) => {
                    *lossy = true;
                    item_as_unknown(item, enc)?
                }
            };
            EditOp::Seq(ListOp::Ins {
                pos: pos_usize(*pos)?,
                item: runtime_item,
            })
        }
        DurableOp::SeqDel { pos, len } => EditOp::Seq(ListOp::Del {
            pos: pos_usize(*pos)?,
            len: pos_usize(*len)?,
        }),
        DurableOp::SeqUndel { del } => EditOp::Seq(ListOp::Undel { del: *del }),
        DurableOp::AddSpan {
            start,
            end,
            modifier,
            tail,
        } => match (from_durable_modifier(modifier), tail.0.is_empty()) {
            (Ok(m), true) => EditOp::Span(SpanOp::AddSpan {
                start: from_durable_anchor(start),
                end: from_durable_anchor(end),
                modifier: m,
            }),
            _ => {
                *lossy = true;
                op_as_unknown(op, enc, record_tail)?
            }
        },
        DurableOp::RemoveSpan {
            start,
            end,
            kind,
            tail,
        } => match (from_durable_kind(kind), tail.0.is_empty()) {
            (Ok(k), true) => EditOp::Span(SpanOp::RemoveSpan {
                start: from_durable_anchor(start),
                end: from_durable_anchor(end),
                modifier_type: k,
            }),
            _ => {
                *lossy = true;
                op_as_unknown(op, enc, record_tail)?
            }
        },
        DurableOp::SetBlockModifier {
            target,
            modifier,
            tail,
        } => match (from_durable_modifier(modifier), tail.0.is_empty()) {
            (Ok(m), true) => EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: *target,
                modifier: m,
            }),
            _ => {
                *lossy = true;
                op_as_unknown(op, enc, record_tail)?
            }
        },
        DurableOp::ClearBlockModifier { target, kind, tail } => {
            match (from_durable_kind(kind), tail.0.is_empty()) {
                (Ok(k), true) => EditOp::BlockModifier(ModifierAttrOp::ClearModifier {
                    target: *target,
                    key: k,
                }),
                _ => {
                    *lossy = true;
                    op_as_unknown(op, enc, record_tail)?
                }
            }
        }
        DurableOp::SetNodeAttr { target, attr, tail } => {
            if tail.0.is_empty() {
                EditOp::NodeAttr(NodeAttrOp {
                    target: *target,
                    attr: from_durable_attr(attr),
                })
            } else {
                *lossy = true;
                op_as_unknown(op, enc, record_tail)?
            }
        }
        DurableOp::SetNodeCarry {
            target,
            modifier,
            tail,
        } => match (from_durable_modifier(modifier), tail.0.is_empty()) {
            (Ok(m), true) => EditOp::NodeCarry(ModifierAttrOp::SetModifier {
                target: *target,
                modifier: m,
            }),
            _ => {
                *lossy = true;
                op_as_unknown(op, enc, record_tail)?
            }
        },
        DurableOp::ClearNodeCarry { target, kind, tail } => {
            match (from_durable_kind(kind), tail.0.is_empty()) {
                (Ok(k), true) => EditOp::NodeCarry(ModifierAttrOp::ClearModifier {
                    target: *target,
                    key: k,
                }),
                _ => {
                    *lossy = true;
                    op_as_unknown(op, enc, record_tail)?
                }
            }
        }
        DurableOp::AliasDots { pairs, tail } => {
            if !tail.0.is_empty() {
                *lossy = true;
                op_as_unknown(op, enc, record_tail)?
            } else {
                EditOp::Alias(from_durable_alias_op(pairs)?)
            }
        }
        DurableOp::Unknown(_) => {
            *lossy = true;
            op_as_unknown(op, enc, record_tail)?
        }
    })
}

pub fn decode_changesets(bytes: &[u8]) -> CodecResult<Decoded> {
    let (ctx, bundles) = decode_bundle_with_ctx(bytes)?;
    let enc = EncCtx::from_parts(&ctx.actors, ctx.baselines.clone())?;
    let mut lossless = true;
    let changesets = bundles
        .into_iter()
        .map(|cs| {
            let ops = cs
                .records
                .into_iter()
                .map(|r| {
                    let payload = match &r.payload {
                        RecordPayload::Known(op) => {
                            if op.contains_ctx_unknown() || !r.record_tail.is_empty() {
                                lossless = false;
                            }
                            let mut lossy = false;
                            let converted = from_durable_op(op, &enc, &r.record_tail, &mut lossy)?;
                            if lossy {
                                lossless = false;
                            }
                            converted
                        }
                        RecordPayload::Preserved(preserved_bytes) => {
                            lossless = false;
                            EditOp::Unknown {
                                bytes: preserved_bytes.clone(),
                            }
                        }
                    };
                    Ok(Op {
                        id: r.id,
                        parents: r.parents,
                        payload,
                    })
                })
                .collect::<CodecResult<Vec<_>>>()?;
            Ok(Changeset { ops })
        })
        .collect::<CodecResult<Vec<_>>>()?;
    Ok(Decoded {
        changesets,
        lossless,
    })
}

pub fn decode_changeset_stream(bytes: &[u8]) -> CodecResult<Decoded> {
    let mut input = bytes;
    let mut changesets = Vec::new();
    let mut lossless = true;
    while !input.is_empty() {
        let before = input;
        unwrap_one(&mut input)?;
        let consumed = before.len() - input.len();
        let mut decoded = decode_changesets(&before[..consumed])?;
        lossless &= decoded.lossless;
        changesets.append(&mut decoded.changesets);
    }
    Ok(Decoded {
        changesets,
        lossless,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CodecError;

    fn representative_changesets() -> Vec<Changeset<EditOp>> {
        let d = |c| Dot::new(1, c);
        let ops = vec![
            Op {
                id: d(0),
                parents: vec![],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 0,
                    item: SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![Dot::ROOT],
                        attrs: vec![],
                    },
                }),
            },
            Op {
                id: d(1),
                parents: vec![d(0)],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 1,
                    item: SeqItem::Char('가'),
                }),
            },
            Op {
                id: d(2),
                parents: vec![d(1)],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 2,
                    item: SeqItem::Atom(AtomLeaf::HardBreak),
                }),
            },
            Op {
                id: d(3),
                parents: vec![d(2)],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 3,
                    item: SeqItem::BlockAtom {
                        leaf: AtomLeaf::Image {
                            node: editor_model::ImageNode::default(),
                        },
                        parents: vec![Dot::ROOT],
                    },
                }),
            },
            Op {
                id: d(4),
                parents: vec![d(3)],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 2,
                    item: SeqItem::Block {
                        node_type: NodeType::Callout,
                        parents: vec![Dot::ROOT],
                        attrs: vec![NodeAttr::Callout {
                            attr: editor_model::CalloutNodeAttr::Variant(
                                editor_model::CalloutVariant::Warning,
                            ),
                        }],
                    },
                }),
            },
            Op {
                id: d(5),
                parents: vec![d(4)],
                payload: EditOp::Span(SpanOp::AddSpan {
                    start: editor_model::Anchor {
                        id: d(1),
                        bias: editor_model::Bias::Before,
                    },
                    end: editor_model::Anchor {
                        id: d(1),
                        bias: editor_model::Bias::After,
                    },
                    modifier: Modifier::Bold,
                }),
            },
            Op {
                id: d(6),
                parents: vec![d(5)],
                payload: EditOp::NodeAttr(NodeAttrOp {
                    target: d(4),
                    attr: NodeAttr::Callout {
                        attr: editor_model::CalloutNodeAttr::Variant(
                            editor_model::CalloutVariant::Danger,
                        ),
                    },
                }),
            },
            Op {
                id: d(7),
                parents: vec![d(6)],
                payload: EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                    target: d(0),
                    modifier: Modifier::FontSize { value: 1400 },
                }),
            },
            Op {
                id: d(8),
                parents: vec![d(7)],
                payload: EditOp::NodeCarry(ModifierAttrOp::ClearModifier {
                    target: d(0),
                    key: ModifierType::FontSize,
                }),
            },
            Op {
                id: d(9),
                parents: vec![d(8)],
                payload: EditOp::Seq(ListOp::Del { pos: 1, len: 1 }),
            },
            Op {
                id: d(10),
                parents: vec![d(9)],
                payload: EditOp::Seq(ListOp::Undel { del: d(9) }),
            },
            Op {
                id: d(11),
                parents: vec![d(10)],
                payload: EditOp::Span(SpanOp::RemoveSpan {
                    start: editor_model::Anchor {
                        id: d(1),
                        bias: editor_model::Bias::Before,
                    },
                    end: editor_model::Anchor {
                        id: d(1),
                        bias: editor_model::Bias::After,
                    },
                    modifier_type: ModifierType::Bold,
                }),
            },
            Op {
                id: d(12),
                parents: vec![d(11)],
                payload: EditOp::BlockModifier(ModifierAttrOp::ClearModifier {
                    target: d(0),
                    key: ModifierType::FontSize,
                }),
            },
            Op {
                id: d(13),
                parents: vec![d(12)],
                payload: EditOp::NodeCarry(ModifierAttrOp::SetModifier {
                    target: d(0),
                    modifier: Modifier::Italic,
                }),
            },
            Op {
                id: d(14),
                parents: vec![d(13)],
                payload: EditOp::Alias(AliasOp {
                    pairs: vec![AliasRun {
                        old_start: Dot::new(1, 10),
                        len: 3,
                        new_start: Dot::new(2, 20),
                    }],
                }),
            },
        ];
        vec![Changeset { ops }]
    }

    #[test]
    fn changesets_round_trip_via_durable_form() {
        let css = representative_changesets();
        let bytes = encode_changesets(ReencodableChangesets::from_local_ops(css.clone())).unwrap();
        let decoded = decode_changesets(&bytes).unwrap().into_graph_input();
        assert_eq!(decoded.len(), css.len());
        for (a, b) in css[0].ops.iter().zip(&decoded[0].ops) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.parents, b.parents);
            assert_eq!(
                to_durable_op(&a.payload).unwrap(),
                to_durable_op(&b.payload).unwrap()
            );
        }
    }

    #[test]
    fn unknown_ops_are_sealed_from_encode() {
        let css = vec![Changeset {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: EditOp::Unknown {
                    bytes: vec![0x63, 0x01, 0xAA],
                },
            }],
        }];
        assert!(matches!(
            encode_changesets(ReencodableChangesets::from_local_ops(css.clone())),
            Err(CodecError::Encode(EncodeInvariant::UnknownPayloadEncode))
        ));
        assert!(changesets_contain_unknown(&css));
    }

    /// 원본 preamble 컨텍스트 안에서 op 수준 unknown(태그 미인식)을 손인코딩한
    /// 단일-레코드 번들 — `into_reencodable_gates_lossy_input`의 lossy negative-path
    /// 픽스처. 공개 `encode_bundle`은 이런 값을 값 수준에서 거부하므로(unknown 봉인),
    /// 수신 경로의 저수준 배관(preamble/frame/envelope 프리미티브)으로 직접 합성한다.
    fn synth_bundle_with_unknown_op() -> Vec<u8> {
        let actors = [7u64];
        let baselines = [0u64];
        let ctx = crate::ctx::EncCtx::from_parts(&actors, baselines.to_vec()).unwrap();
        let mut body = Vec::new();
        crate::ctx::write_preamble(&actors, &baselines, &mut body).unwrap();
        crate::varint::write_varint(1, &mut body); // changeset_count
        body.push(0); // cs_parents: Genesis
        crate::varint::write_varint(1, &mut body); // op_count
        crate::framing::write_frame(&mut body, |b| {
            crate::ctx::write_dot(&Dot::new(7, 0), &ctx, b)?;
            crate::types::op::DurableOp::Unknown(crate::framing::UnknownPayload {
                tag: 12345,
                bytes: vec![0xAB],
            })
            .encode(&ctx, b)
        })
        .unwrap();
        crate::envelope::wrap(&crate::envelope::Envelope::new(
            crate::envelope::PayloadKind::ChangesetBundle,
            body,
        ))
        .unwrap()
    }

    #[test]
    fn into_reencodable_gates_lossy_input() {
        let css = representative_changesets();
        let bytes = encode_changesets(ReencodableChangesets::from_local_ops(css)).unwrap();
        assert!(
            decode_changesets(&bytes)
                .unwrap()
                .into_reencodable()
                .is_ok()
        );

        let lossy_bytes = synth_bundle_with_unknown_op();
        assert!(matches!(
            decode_changesets(&lossy_bytes).unwrap().into_reencodable(),
            Err(CodecError::Fenced(crate::error::Fenced::LossyForReencode))
        ));
    }

    #[test]
    fn atom_round_trip_reseeds_ledger_but_preserves_values() {
        let node = editor_model::ImageNode::default();
        let leaf = AtomLeaf::Image { node };
        let css = vec![Changeset {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 0,
                    item: SeqItem::BlockAtom {
                        leaf,
                        parents: vec![Dot::ROOT],
                    },
                }),
            }],
        }];
        let bytes = encode_changesets(ReencodableChangesets::from_local_ops(css.clone())).unwrap();
        let decoded = decode_changesets(&bytes).unwrap().into_graph_input();
        assert_eq!(
            to_durable_op(&css[0].ops[0].payload).unwrap(),
            to_durable_op(&decoded[0].ops[0].payload).unwrap()
        );
    }

    #[test]
    fn decode_changeset_stream_concatenates_and_folds_lossless() {
        let css_a = representative_changesets();
        let css_b = vec![Changeset {
            ops: vec![Op {
                id: Dot::new(2, 0),
                parents: vec![],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 0,
                    item: SeqItem::Char('b'),
                }),
            }],
        }];
        let mut stream =
            encode_changesets(ReencodableChangesets::from_local_ops(css_a.clone())).unwrap();
        stream.extend(
            encode_changesets(ReencodableChangesets::from_local_ops(css_b.clone())).unwrap(),
        );
        let decoded = decode_changeset_stream(&stream).unwrap();
        assert!(decoded.lossless);
        let changesets = decoded.into_graph_input();
        assert_eq!(changesets.len(), css_a.len() + css_b.len());
    }

    #[test]
    fn reencodable_changesets_accessors_preserve_provenance() {
        let css = representative_changesets();
        let mut wrapped = ReencodableChangesets::from_local_ops(css.clone());
        assert_eq!(wrapped.as_slice().len(), 1);
        wrapped.retain(|_| true);
        assert_eq!(wrapped.as_slice().len(), 1);
        let combined = ReencodableChangesets::concat(vec![
            ReencodableChangesets::from_local_ops(css.clone()),
            ReencodableChangesets::from_local_ops(css),
        ]);
        assert_eq!(combined.as_slice().len(), 2);
    }

    #[test]
    fn to_durable_op_rejects_invalid_alias_op_same_as_admission_gate() {
        let self_run = EditOp::Alias(AliasOp {
            pairs: vec![AliasRun {
                old_start: Dot::new(1, 10),
                len: 1,
                new_start: Dot::new(1, 10),
            }],
        });
        assert!(matches!(
            to_durable_op(&self_run),
            Err(CodecError::Encode(EncodeInvariant::InvalidAliasOp))
        ));

        let zero_len = EditOp::Alias(AliasOp {
            pairs: vec![AliasRun {
                old_start: Dot::new(1, 10),
                len: 0,
                new_start: Dot::new(2, 20),
            }],
        });
        assert!(matches!(
            to_durable_op(&zero_len),
            Err(CodecError::Encode(EncodeInvariant::InvalidAliasOp))
        ));
    }

    #[test]
    fn from_durable_alias_op_rejects_same_invalid_shapes_as_alias_op_is_valid() {
        let invalid = DurableAliasRun {
            old_start: Dot::new(1, 10),
            len: 0,
            new_start: Dot::new(2, 20),
        };
        assert!(matches!(
            from_durable_alias_op(&[invalid]),
            Err(CodecError::Corruption(Corruption::InvalidAliasOp))
        ));

        let len_overflow = DurableAliasRun {
            old_start: Dot::new(1, 10),
            len: u64::from(u32::MAX) + 1,
            new_start: Dot::new(2, 20),
        };
        assert!(matches!(
            from_durable_alias_op(&[len_overflow]),
            Err(CodecError::Corruption(Corruption::VarintOverflow))
        ));

        let cross_run_overlap = [
            DurableAliasRun {
                old_start: Dot::new(1, 0),
                len: 1,
                new_start: Dot::new(1, 1),
            },
            DurableAliasRun {
                old_start: Dot::new(1, 1),
                len: 1,
                new_start: Dot::new(1, 2),
            },
        ];
        assert!(matches!(
            from_durable_alias_op(&cross_run_overlap),
            Err(CodecError::Corruption(Corruption::InvalidAliasOp))
        ));

        let valid = DurableAliasRun {
            old_start: Dot::new(1, 10),
            len: 3,
            new_start: Dot::new(2, 20),
        };
        assert_eq!(
            from_durable_alias_op(&[valid]).unwrap(),
            AliasOp {
                pairs: vec![AliasRun {
                    old_start: Dot::new(1, 10),
                    len: 3,
                    new_start: Dot::new(2, 20),
                }],
            }
        );
    }
}
