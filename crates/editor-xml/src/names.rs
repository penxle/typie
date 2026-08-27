use std::collections::BTreeMap;

use editor_model::{
    Alignment, BlockquoteVariant, CalloutVariant, HorizontalRuleVariant, LayoutMode, Modifier,
    ModifierType, NodeType, PlainArchivedNode, PlainBlockquoteNode, PlainBulletListNode,
    PlainCalloutNode, PlainEmbedNode, PlainFileNode, PlainFoldContentNode, PlainFoldNode,
    PlainFoldTitleNode, PlainHardBreakNode, PlainHorizontalRuleNode, PlainImageNode,
    PlainListItemNode, PlainNode, PlainOrderedListNode, PlainPageBreakNode, PlainParagraphNode,
    PlainRootNode, PlainTabNode, PlainTableCellNode, PlainTableNode, PlainTableRowNode,
    TableBorderStyle,
};

use crate::error::XmlErrorDetail;

pub fn element_name(t: NodeType) -> Option<&'static str> {
    Some(match t {
        NodeType::Root => "root",
        NodeType::Paragraph => "paragraph",
        NodeType::Blockquote => "blockquote",
        NodeType::Callout => "callout",
        NodeType::Text => return None,
        NodeType::BulletList => "bullet_list",
        NodeType::OrderedList => "ordered_list",
        NodeType::ListItem => "list_item",
        NodeType::Fold => "fold",
        NodeType::FoldTitle => "fold_title",
        NodeType::FoldContent => "fold_content",
        NodeType::Table => "table",
        NodeType::TableRow => "table_row",
        NodeType::TableCell => "table_cell",
        NodeType::Image => "image",
        NodeType::File => "file",
        NodeType::Embed => "embed",
        NodeType::Archived => "archived",
        NodeType::HardBreak => "hard_break",
        NodeType::HorizontalRule => "horizontal_rule",
        NodeType::PageBreak => "page_break",
        NodeType::Tab => "tab",
        NodeType::Unknown => "unknown",
    })
}

pub fn node_type_of(name: &str) -> Option<NodeType> {
    use strum::IntoEnumIterator;
    NodeType::iter().find(|t| element_name(*t) == Some(name))
}

pub fn is_textblock(t: NodeType) -> bool {
    t.spec().is_textblock()
}

pub fn is_inline_atom(t: NodeType) -> bool {
    t.spec().inline && t != NodeType::Text
}

/// Leaf-content block elements: they hold a block slot but project as atom
/// leaves (`ChildView::Leaf`), so they have no `NodeView` of their own.
pub fn is_block_atom(t: NodeType) -> bool {
    !t.spec().inline && t.spec().is_leaf()
}

/// Blocks whose interior the format does not describe: the writer emits them
/// self-closed and the reader refuses children under them.
pub fn is_opaque(t: NodeType) -> bool {
    matches!(t, NodeType::Unknown | NodeType::Archived)
}

pub fn modifier_type_name(ty: ModifierType) -> String {
    serde_json::to_value(ty)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

pub fn modifier_type_of(name: &str) -> Option<ModifierType> {
    serde_json::from_value(serde_json::Value::String(name.to_owned())).ok()
}

pub fn inline_modifier_attr(ty: ModifierType) -> Option<&'static str> {
    match ty {
        ModifierType::Bold
        | ModifierType::Italic
        | ModifierType::Underline
        | ModifierType::Strikethrough => None,
        ModifierType::Link => Some("href"),
        ModifierType::Ruby => Some("text"),
        _ => Some("value"),
    }
}

pub fn modifier_value(m: &Modifier) -> Option<String> {
    match m {
        Modifier::Bold | Modifier::Italic | Modifier::Underline | Modifier::Strikethrough => None,
        Modifier::FontSize { value } => Some(value.to_string()),
        Modifier::FontFamily { value } => Some(value.clone()),
        Modifier::FontWeight { value } => Some(value.to_string()),
        Modifier::TextColor { value } | Modifier::BackgroundColor { value } => Some(value.clone()),
        Modifier::LetterSpacing { value } => Some(value.to_string()),
        Modifier::Link { href } => Some(href.clone()),
        Modifier::Ruby { text } => Some(text.clone()),
        Modifier::LineHeight { value }
        | Modifier::BlockGap { value }
        | Modifier::ParagraphIndent { value } => Some(value.to_string()),
        Modifier::Alignment { value } => Some(enum_str(value)),
    }
}

/// Whether the schema lets a modifier kind sit where `path` ends. The reader
/// validates a file against this and the writer filters by it, so the two
/// never disagree about which stored modifiers the format can carry.
pub fn modifier_fits_context(ty: ModifierType, path: &[NodeType]) -> bool {
    ty.spec().context.matches(path)
}

/// Exactly what the writer puts in the file (`writer.rs`): valid values only,
/// only kinds that carry one, and only where the schema lets the kind sit —
/// `path` runs from the root down to the node holding them. Anything else is
/// invisible to the target and must not read as a difference.
pub fn writable_modifiers(
    mods: &BTreeMap<ModifierType, Modifier>,
    path: &[NodeType],
) -> BTreeMap<ModifierType, Modifier> {
    mods.iter()
        .filter(|(ty, m)| {
            m.is_valid() && modifier_value(m).is_some() && modifier_fits_context(**ty, path)
        })
        .map(|(ty, m)| (*ty, m.clone()))
        .collect()
}

pub fn modifier_from(ty: ModifierType, value: &str) -> Result<Modifier, XmlErrorDetail> {
    let int = |v: &str| {
        v.parse::<i64>()
            .map_err(|_| XmlErrorDetail::ValueNotInteger {
                value: v.to_owned(),
            })
    };
    let out_of_range = || XmlErrorDetail::ValueOutOfRange {
        modifier: modifier_type_name(ty),
        value: value.to_owned(),
    };
    let m = match ty {
        ModifierType::Bold => Modifier::Bold,
        ModifierType::Italic => Modifier::Italic,
        ModifierType::Underline => Modifier::Underline,
        ModifierType::Strikethrough => Modifier::Strikethrough,
        ModifierType::FontSize => Modifier::FontSize {
            value: u32::try_from(int(value)?).map_err(|_| out_of_range())?,
        },
        ModifierType::FontFamily => Modifier::FontFamily {
            value: value.to_owned(),
        },
        ModifierType::FontWeight => Modifier::FontWeight {
            value: u16::try_from(int(value)?).map_err(|_| out_of_range())?,
        },
        ModifierType::TextColor => Modifier::TextColor {
            value: value.to_owned(),
        },
        ModifierType::BackgroundColor => Modifier::BackgroundColor {
            value: value.to_owned(),
        },
        ModifierType::LetterSpacing => Modifier::LetterSpacing {
            value: i32::try_from(int(value)?).map_err(|_| out_of_range())?,
        },
        ModifierType::Link => Modifier::Link {
            href: value.to_owned(),
        },
        ModifierType::Ruby => Modifier::Ruby {
            text: value.to_owned(),
        },
        ModifierType::LineHeight => Modifier::LineHeight {
            value: u32::try_from(int(value)?).map_err(|_| out_of_range())?,
        },
        ModifierType::BlockGap => Modifier::BlockGap {
            value: u32::try_from(int(value)?).map_err(|_| out_of_range())?,
        },
        ModifierType::ParagraphIndent => Modifier::ParagraphIndent {
            value: u32::try_from(int(value)?).map_err(|_| out_of_range())?,
        },
        ModifierType::Alignment => Modifier::Alignment {
            value: enum_parse::<Alignment>(value)?,
        },
    };
    if !m.is_valid() {
        return Err(out_of_range());
    }
    Ok(m)
}

fn enum_str<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn enum_parse<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, XmlErrorDetail> {
    serde_json::from_value(serde_json::Value::String(s.to_owned())).map_err(|_| {
        XmlErrorDetail::EnumValueUnknown {
            value: s.to_owned(),
        }
    })
}

fn text_is_not_an_element() -> XmlErrorDetail {
    XmlErrorDetail::Internal {
        message: "text is not an element".to_owned(),
    }
}

pub type Attrs = BTreeMap<String, String>;

pub fn node_attrs(node: &PlainNode) -> Attrs {
    let mut out = Attrs::new();
    match node {
        PlainNode::Root(PlainRootNode { layout_mode }) => match layout_mode {
            LayoutMode::Continuous { max_width } => {
                out.insert("layout_mode".into(), "continuous".into());
                out.insert("max_width".into(), max_width.to_string());
            }
            LayoutMode::Paginated {
                page_width,
                page_height,
                page_margin_top,
                page_margin_bottom,
                page_margin_left,
                page_margin_right,
            } => {
                out.insert("layout_mode".into(), "paginated".into());
                out.insert("page_width".into(), page_width.to_string());
                out.insert("page_height".into(), page_height.to_string());
                out.insert("page_margin_top".into(), page_margin_top.to_string());
                out.insert("page_margin_bottom".into(), page_margin_bottom.to_string());
                out.insert("page_margin_left".into(), page_margin_left.to_string());
                out.insert("page_margin_right".into(), page_margin_right.to_string());
            }
        },
        PlainNode::Paragraph(PlainParagraphNode {}) => {}
        PlainNode::Blockquote(PlainBlockquoteNode { variant }) => {
            out.insert("variant".into(), enum_str(variant));
        }
        PlainNode::Callout(PlainCalloutNode { variant }) => {
            out.insert("variant".into(), enum_str(variant));
        }
        PlainNode::Text(_) => {}
        PlainNode::BulletList(PlainBulletListNode {})
        | PlainNode::OrderedList(PlainOrderedListNode {})
        | PlainNode::ListItem(PlainListItemNode {}) => {}
        PlainNode::Fold(PlainFoldNode {})
        | PlainNode::FoldTitle(PlainFoldTitleNode {})
        | PlainNode::FoldContent(PlainFoldContentNode {}) => {}
        PlainNode::Table(PlainTableNode {
            border_style,
            proportion,
        }) => {
            out.insert("border_style".into(), enum_str(border_style));
            out.insert("proportion".into(), proportion.to_string());
        }
        PlainNode::TableRow(PlainTableRowNode {}) => {}
        PlainNode::TableCell(PlainTableCellNode {
            col_width,
            background_color: _,
        }) => {
            if let Some(w) = col_width {
                out.insert("col_width".into(), w.to_string());
            }
        }
        PlainNode::Image(PlainImageNode { id, proportion }) => {
            if let Some(id) = id {
                out.insert("id".into(), id.clone());
            }
            out.insert("proportion".into(), proportion.to_string());
        }
        PlainNode::File(PlainFileNode { id })
        | PlainNode::Embed(PlainEmbedNode { id })
        | PlainNode::Archived(PlainArchivedNode { id }) => {
            if let Some(id) = id {
                out.insert("id".into(), id.clone());
            }
        }
        PlainNode::HardBreak(PlainHardBreakNode {})
        | PlainNode::PageBreak(PlainPageBreakNode {})
        | PlainNode::Tab(PlainTabNode {}) => {}
        PlainNode::HorizontalRule(PlainHorizontalRuleNode { variant }) => {
            out.insert("variant".into(), enum_str(variant));
        }
        PlainNode::Unknown => {}
    }
    out
}

pub fn node_from_attrs(t: NodeType, attrs: &Attrs) -> Result<PlainNode, XmlErrorDetail> {
    let Some(element) = element_name(t) else {
        return Err(text_is_not_an_element());
    };
    let get = |k: &str| attrs.get(k).map(String::as_str);
    let need = |k: &str| {
        get(k).ok_or_else(|| XmlErrorDetail::NodeAttrMissing {
            element: element.to_owned(),
            field: k.to_owned(),
        })
    };
    let num = |k: &str| -> Result<u32, XmlErrorDetail> {
        need(k)?
            .parse::<u32>()
            .map_err(|_| XmlErrorDetail::NodeAttrNotUnsignedInteger {
                element: element.to_owned(),
                field: k.to_owned(),
            })
    };
    let known: &[&str] = match t {
        NodeType::Root => &[
            "layout_mode",
            "max_width",
            "page_width",
            "page_height",
            "page_margin_top",
            "page_margin_bottom",
            "page_margin_left",
            "page_margin_right",
        ],
        NodeType::Blockquote | NodeType::Callout | NodeType::HorizontalRule => &["variant"],
        NodeType::Table => &["border_style", "proportion"],
        NodeType::TableCell => &["col_width"],
        NodeType::Image => &["id", "proportion"],
        NodeType::File | NodeType::Embed | NodeType::Archived => &["id"],
        _ => &[],
    };
    if let Some(extra) = attrs.keys().find(|k| !known.contains(&k.as_str())) {
        return Err(XmlErrorDetail::NodeAttrUnknown {
            element: element.to_owned(),
            field: extra.clone(),
        });
    }
    Ok(match t {
        NodeType::Root => {
            let layout_mode = match need("layout_mode")? {
                "continuous" => LayoutMode::Continuous {
                    max_width: num("max_width")?,
                },
                "paginated" => LayoutMode::Paginated {
                    page_width: num("page_width")?,
                    page_height: num("page_height")?,
                    page_margin_top: num("page_margin_top")?,
                    page_margin_bottom: num("page_margin_bottom")?,
                    page_margin_left: num("page_margin_left")?,
                    page_margin_right: num("page_margin_right")?,
                },
                other => {
                    return Err(XmlErrorDetail::LayoutModeInvalid {
                        value: other.to_owned(),
                    });
                }
            };
            PlainNode::Root(PlainRootNode { layout_mode })
        }
        NodeType::Paragraph => PlainNode::Paragraph(PlainParagraphNode {}),
        NodeType::Blockquote => PlainNode::Blockquote(PlainBlockquoteNode {
            variant: get("variant")
                .map(enum_parse::<BlockquoteVariant>)
                .transpose()?
                .unwrap_or_default(),
        }),
        NodeType::Callout => PlainNode::Callout(PlainCalloutNode {
            variant: get("variant")
                .map(enum_parse::<CalloutVariant>)
                .transpose()?
                .unwrap_or_default(),
        }),
        NodeType::Text => return Err(text_is_not_an_element()),
        NodeType::BulletList => PlainNode::BulletList(PlainBulletListNode {}),
        NodeType::OrderedList => PlainNode::OrderedList(PlainOrderedListNode {}),
        NodeType::ListItem => PlainNode::ListItem(PlainListItemNode {}),
        NodeType::Fold => PlainNode::Fold(PlainFoldNode {}),
        NodeType::FoldTitle => PlainNode::FoldTitle(PlainFoldTitleNode {}),
        NodeType::FoldContent => PlainNode::FoldContent(PlainFoldContentNode {}),
        NodeType::Table => PlainNode::Table(PlainTableNode {
            border_style: get("border_style")
                .map(enum_parse::<TableBorderStyle>)
                .transpose()?
                .unwrap_or_default(),
            proportion: get("proportion")
                .map(|_| num("proportion"))
                .transpose()?
                .unwrap_or(100),
        }),
        NodeType::TableRow => PlainNode::TableRow(PlainTableRowNode {}),
        NodeType::TableCell => PlainNode::TableCell(PlainTableCellNode {
            col_width: get("col_width").map(|_| num("col_width")).transpose()?,
            background_color: None,
        }),
        NodeType::Image => PlainNode::Image(PlainImageNode {
            id: get("id").map(str::to_owned),
            proportion: get("proportion")
                .map(|_| num("proportion"))
                .transpose()?
                .unwrap_or(100),
        }),
        NodeType::File => PlainNode::File(PlainFileNode {
            id: get("id").map(str::to_owned),
        }),
        NodeType::Embed => PlainNode::Embed(PlainEmbedNode {
            id: get("id").map(str::to_owned),
        }),
        NodeType::Archived => PlainNode::Archived(PlainArchivedNode {
            id: get("id").map(str::to_owned),
        }),
        NodeType::HardBreak => PlainNode::HardBreak(PlainHardBreakNode {}),
        NodeType::HorizontalRule => PlainNode::HorizontalRule(PlainHorizontalRuleNode {
            variant: get("variant")
                .map(enum_parse::<HorizontalRuleVariant>)
                .transpose()?
                .unwrap_or_default(),
        }),
        NodeType::PageBreak => PlainNode::PageBreak(PlainPageBreakNode {}),
        NodeType::Tab => PlainNode::Tab(PlainTabNode {}),
        NodeType::Unknown => PlainNode::Unknown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn every_node_type_has_a_unique_element_name_except_text() {
        let names: Vec<&str> = NodeType::iter().filter_map(element_name).collect();
        let mut dedup = names.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(names.len(), dedup.len());
        assert_eq!(names.len(), NodeType::iter().count() - 1);
        for t in NodeType::iter().filter(|t| *t != NodeType::Text) {
            assert_eq!(node_type_of(element_name(t).unwrap()), Some(t));
        }
    }

    #[test]
    fn inline_atoms_are_hard_break_tab_and_page_break() {
        assert!(is_inline_atom(NodeType::HardBreak));
        assert!(is_inline_atom(NodeType::Tab));
        assert!(is_inline_atom(NodeType::PageBreak));
        assert!(!is_inline_atom(NodeType::Text));
        assert!(!is_inline_atom(NodeType::Paragraph));
    }

    #[test]
    fn block_atoms_are_the_five_leaf_content_block_types() {
        let atoms: Vec<NodeType> = NodeType::iter().filter(|t| is_block_atom(*t)).collect();
        assert_eq!(
            atoms,
            vec![
                NodeType::Image,
                NodeType::File,
                NodeType::Embed,
                NodeType::Archived,
                NodeType::HorizontalRule,
            ]
        );
        for t in [NodeType::HardBreak, NodeType::Tab, NodeType::PageBreak] {
            assert!(is_inline_atom(t) && !is_block_atom(t));
        }
        assert!(!is_block_atom(NodeType::Unknown));
        assert!(!is_block_atom(NodeType::Paragraph));
    }

    #[test]
    fn modifier_values_round_trip() {
        for m in [
            Modifier::Bold,
            Modifier::FontSize { value: 1600 },
            Modifier::FontFamily {
                value: "Noto Serif KR".into(),
            },
            Modifier::LetterSpacing { value: -5 },
            Modifier::Link {
                href: "https://typie.co/a?b=1&c=2".into(),
            },
            Modifier::Alignment {
                value: Alignment::Justify,
            },
        ] {
            let ty = m.as_type();
            let back = modifier_from(ty, modifier_value(&m).as_deref().unwrap_or("")).unwrap();
            assert_eq!(back, m);
        }
        assert_eq!(
            modifier_from(ModifierType::FontSize, "99"),
            Err(XmlErrorDetail::ValueOutOfRange {
                modifier: "font_size".into(),
                value: "99".into()
            })
        );
        assert_eq!(
            modifier_from(ModifierType::Alignment, "middle"),
            Err(XmlErrorDetail::EnumValueUnknown {
                value: "middle".into()
            })
        );
        assert_eq!(
            modifier_type_of(&modifier_type_name(ModifierType::ParagraphIndent)),
            Some(ModifierType::ParagraphIndent)
        );
    }

    #[test]
    fn node_attrs_round_trip_and_reject_unknown() {
        let node = PlainNode::Root(PlainRootNode {
            layout_mode: LayoutMode::Paginated {
                page_width: 1,
                page_height: 2,
                page_margin_top: 3,
                page_margin_bottom: 4,
                page_margin_left: 5,
                page_margin_right: 6,
            },
        });
        assert_eq!(
            node_from_attrs(NodeType::Root, &node_attrs(&node)).unwrap(),
            node
        );
        let cell = PlainNode::TableCell(PlainTableCellNode {
            col_width: Some(120),
            background_color: None,
        });
        assert_eq!(
            node_from_attrs(NodeType::TableCell, &node_attrs(&cell)).unwrap(),
            cell
        );
        let mut bad = Attrs::new();
        bad.insert("alignment".into(), "center".into());
        assert_eq!(
            node_from_attrs(NodeType::Paragraph, &bad),
            Err(XmlErrorDetail::NodeAttrUnknown {
                element: "paragraph".into(),
                field: "alignment".into()
            })
        );
    }
}
