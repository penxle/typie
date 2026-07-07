use editor_macros::{content_expr, context_expr};
use enum_map::{EnumMap, enum_map};
use std::sync::LazyLock;

use super::{ModifierSpec, NodeSpec};
use crate::{ModifierType, NodeType};

static INNER: LazyLock<SchemaInner> = LazyLock::new(SchemaInner::default);

pub struct Schema;

impl Schema {
    pub fn node_spec(node_type: NodeType) -> &'static NodeSpec {
        &INNER.nodes[node_type]
    }

    pub fn modifier_spec(modifier_type: ModifierType) -> &'static ModifierSpec {
        &INNER.modifiers[modifier_type]
    }
}

#[derive(Debug)]
struct SchemaInner {
    nodes: EnumMap<NodeType, NodeSpec>,
    modifiers: EnumMap<ModifierType, ModifierSpec>,
}

impl Default for SchemaInner {
    fn default() -> Self {
        Self {
            nodes: enum_map! {
                NodeType::Root => NodeSpec {
                    content: content_expr!((Paragraph | Image | File | Embed | Archived | Blockquote | Callout | BulletList | OrderedList | HorizontalRule | Fold | Table)*, Paragraph),
                    ..Default::default()
                },
                NodeType::Blockquote => NodeSpec {
                    content: content_expr!((Paragraph | BulletList | OrderedList)+),
                    monolithic: true,
                    ..Default::default()
                },
                NodeType::Paragraph => NodeSpec {
                    content: content_expr!((Text | HardBreak | Tab)*, PageBreak?),
                    ..Default::default()
                },
                NodeType::Text => NodeSpec {
                    inline: true,
                    ..Default::default()
                },
                NodeType::Image => NodeSpec {
                    selectable: true,
                    external: true,
                    ..Default::default()
                },
                NodeType::File => NodeSpec {
                    selectable: true,
                    external: true,
                    ..Default::default()
                },
                NodeType::Embed => NodeSpec {
                    selectable: true,
                    external: true,
                    ..Default::default()
                },
                NodeType::Archived => NodeSpec {
                    selectable: true,
                    external: true,
                    ..Default::default()
                },
                NodeType::HardBreak => NodeSpec {
                    inline: true,
                    ..Default::default()
                },
                NodeType::PageBreak => NodeSpec {
                    context: context_expr!(Root > Paragraph > &),
                    inline: true,
                    ..Default::default()
                },
                NodeType::HorizontalRule => NodeSpec {
                    selectable: true,
                    ..Default::default()
                },
                NodeType::BulletList => NodeSpec {
                    content: content_expr!(ListItem+),
                    ..Default::default()
                },
                NodeType::OrderedList => NodeSpec {
                    content: content_expr!(ListItem+),
                    ..Default::default()
                },
                NodeType::ListItem => NodeSpec {
                    content: content_expr!(Paragraph, (BulletList | OrderedList)?),
                    ..Default::default()
                },
                NodeType::Fold => NodeSpec {
                    content: content_expr!(FoldTitle, FoldContent),
                    isolating: true,
                    monolithic: true,
                    ..Default::default()
                },
                NodeType::FoldTitle => NodeSpec {
                    content: content_expr!(Text*),
                    isolating: true,
                    structural: true,
                    ..Default::default()
                },
                NodeType::FoldContent => NodeSpec {
                    content: content_expr!((Paragraph | Image | File | Embed | Archived | Blockquote | Callout | BulletList | OrderedList | HorizontalRule | Fold | Table)+),
                    isolating: true,
                    structural: true,
                    ..Default::default()
                },
                NodeType::Callout => NodeSpec {
                    content: content_expr!((Paragraph | BulletList | OrderedList)+),
                    monolithic: true,
                    ..Default::default()
                },
                NodeType::Table => NodeSpec {
                    content: content_expr!(TableRow+),
                    context: context_expr!(!Table > ** > &),
                    isolating: true,
                    monolithic: true,
                    ..Default::default()
                },
                NodeType::TableRow => NodeSpec {
                    content: content_expr!(TableCell+),
                    structural: true,
                    ..Default::default()
                },
                NodeType::TableCell => NodeSpec {
                    content: content_expr!((Paragraph | Image | File | Embed | Archived | Blockquote | Callout | BulletList | OrderedList | HorizontalRule | Fold)+),
                    isolating: true,
                    structural: true,
                    ..Default::default()
                },
                NodeType::Tab => NodeSpec {
                    inline: true,
                    ..Default::default()
                },
            },
            modifiers: enum_map! {
                ModifierType::Bold => ModifierSpec {
                    context: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    target: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    inheritable: false,
                    ..Default::default()
                },
                ModifierType::Italic => ModifierSpec {
                    context: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    target: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    inheritable: false,
                    ..Default::default()
                },
                ModifierType::Underline => ModifierSpec {
                    context: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    target: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    inheritable: false,
                    ..Default::default()
                },
                ModifierType::Strikethrough => ModifierSpec {
                    context: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    target: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    inheritable: false,
                    ..Default::default()
                },
                ModifierType::FontSize => ModifierSpec {
                    context: context_expr!(Root | Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    target: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    ..Default::default()
                },
                ModifierType::FontFamily => ModifierSpec {
                    context: context_expr!(Root | Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    target: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    ..Default::default()
                },
                ModifierType::FontWeight => ModifierSpec {
                    context: context_expr!(Root | Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    target: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    ..Default::default()
                },
                ModifierType::TextColor => ModifierSpec {
                    context: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    target: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    inheritable: false,
                    ..Default::default()
                },
                ModifierType::BackgroundColor => ModifierSpec {
                    context: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak | TableCell),
                    target: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    inheritable: false,
                    ..Default::default()
                },
                ModifierType::LetterSpacing => ModifierSpec {
                    context: context_expr!(Root | Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    target: context_expr!(Paragraph > Text | Paragraph > Tab | Paragraph > HardBreak),
                    ..Default::default()
                },
                ModifierType::Link => ModifierSpec {
                    context: context_expr!(Paragraph > Text),
                    target: context_expr!(Paragraph > Text),
                    inheritable: false,
                    ..Default::default()
                },
                ModifierType::Ruby => ModifierSpec {
                    context: context_expr!(Paragraph > Text),
                    target: context_expr!(Paragraph > Text),
                    inheritable: false,
                    ..Default::default()
                },
                ModifierType::LineHeight => ModifierSpec {
                    context: context_expr!(Root | Paragraph),
                    target: context_expr!(Paragraph),
                    ..Default::default()
                },
                ModifierType::BlockGap => ModifierSpec {
                    context: context_expr!(Root),
                    target: editor_model::ContextExpr::AnyOf(Vec::new()),
                    ..Default::default()
                },
                ModifierType::ParagraphIndent => ModifierSpec {
                    context: context_expr!(Root | Root > Paragraph),
                    target: context_expr!(Root > Paragraph),
                    ..Default::default()
                },
                ModifierType::Alignment => ModifierSpec {
                    context: context_expr!(Root | Paragraph | Image | Table),
                    target: context_expr!(Paragraph | Image | Table),
                    ..Default::default()
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_style_default_modifier;
    use strum::IntoEnumIterator;

    #[test]
    fn inheritable_kinds_form_the_eight_kind_symmetry() {
        let inheritable: Vec<ModifierType> = ModifierType::iter()
            .filter(|ty| Schema::modifier_spec(*ty).inheritable)
            .collect();
        let expected = [
            ModifierType::FontSize,
            ModifierType::FontFamily,
            ModifierType::FontWeight,
            ModifierType::LetterSpacing,
            ModifierType::LineHeight,
            ModifierType::Alignment,
            ModifierType::BlockGap,
            ModifierType::ParagraphIndent,
        ];
        assert_eq!(inheritable.len(), 8);
        for ty in expected {
            assert!(inheritable.contains(&ty));
            assert!(text_style_default_modifier(ty).is_some());
        }
        for ty in ModifierType::iter().filter(|t| !inheritable.contains(t)) {
            assert!(text_style_default_modifier(ty).is_none());
        }
    }
}
