use std::collections::BTreeMap;

use editor_crdt::ToPlain;
use editor_model::{Doc, Modifier, ModifierType, NodeRef, NodeType, PlainDoc, PlainNodeEntry};

use crate::measure::resolve::resolve_inherited;
use crate::measure::text::extract::resolve_text_colors;

pub fn to_plain_resolved(doc: &Doc) -> PlainDoc {
    let nodes = doc
        .nodes_iter()
        .map(|(id, _)| {
            let entry = doc.get_entry(*id).expect("nodes_iter consistency");
            let nr = doc.node(*id).expect("nodes_iter consistency");
            (
                *id,
                PlainNodeEntry {
                    parent: entry.parent.to_plain(),
                    children: entry.children.to_plain(),
                    modifiers: export_modifiers(doc, &nr),
                    style: None,
                    node: entry.node.to_plain(),
                },
            )
        })
        .collect();

    PlainDoc {
        nodes,
        styles: Default::default(),
    }
}

fn export_modifiers(doc: &Doc, nr: &NodeRef<'_>) -> BTreeMap<ModifierType, Modifier> {
    use ModifierType::*;

    let mut out: BTreeMap<ModifierType, Modifier> = BTreeMap::new();

    match nr.as_type() {
        NodeType::Text => {
            let (text_color, bg) = resolve_text_colors(doc, nr.id());
            put(&mut out, TextColor, strip_color(TextColor, &text_color));
            put(
                &mut out,
                BackgroundColor,
                bg.as_deref().and_then(|s| strip_color(BackgroundColor, s)),
            );
            for ty in [
                FontFamily,
                FontSize,
                FontWeight,
                LetterSpacing,
                Bold,
                Italic,
                Underline,
                Strikethrough,
            ] {
                put(&mut out, ty, resolve_inherited(nr, ty).cloned());
            }
            for ty in [Link, Ruby] {
                put(&mut out, ty, own_modifier(nr, ty));
            }
        }
        NodeType::Paragraph | NodeType::Image | NodeType::Table => {
            put(
                &mut out,
                LineHeight,
                resolve_inherited(nr, LineHeight).cloned(),
            );
            put(&mut out, Alignment, own_modifier(nr, Alignment));
        }
        NodeType::TableCell => {
            put(&mut out, BackgroundColor, own_modifier(nr, BackgroundColor));
        }
        NodeType::Root => {
            for ty in [FontFamily, FontSize, LineHeight, BlockGap, ParagraphIndent] {
                put(&mut out, ty, own_modifier(nr, ty));
            }
        }
        _ => {}
    }

    out
}

fn put(out: &mut BTreeMap<ModifierType, Modifier>, ty: ModifierType, m: Option<Modifier>) {
    if let Some(m) = m {
        out.insert(ty, m);
    }
}

fn own_modifier(nr: &NodeRef<'_>, ty: ModifierType) -> Option<Modifier> {
    nr.modifiers_with_style()
        .find(|m| m.as_type() == ty)
        .cloned()
}

fn strip_color(ty: ModifierType, value: &str) -> Option<Modifier> {
    let prefix = match ty {
        ModifierType::TextColor => "text.",
        ModifierType::BackgroundColor => "bg.",
        _ => return None,
    };
    let raw = value.strip_prefix(prefix).unwrap_or(value);
    if raw.is_empty() || raw == "none" {
        return None;
    }
    Some(match ty {
        ModifierType::TextColor => Modifier::TextColor {
            value: raw.to_string(),
        },
        ModifierType::BackgroundColor => Modifier::BackgroundColor {
            value: raw.to_string(),
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::Modifier;

    use super::*;

    #[test]
    fn to_plain_resolved_flattens_inherited_and_link_color() {
        let (doc, t1) = doc! {
            root [font_family("A".to_string())] {
                paragraph {
                    t1: text("hello") [link(href: "https://example.com".into())]
                }
            }
        };

        let plain = to_plain_resolved(&doc);
        let entry = plain.nodes.get(&t1).expect("text node present");

        assert_eq!(
            entry.modifiers.get(&ModifierType::FontFamily),
            Some(&Modifier::FontFamily {
                value: "A".to_string()
            })
        );
        assert_eq!(
            entry.modifiers.get(&ModifierType::TextColor),
            Some(&Modifier::TextColor {
                value: "blue".to_string()
            })
        );
    }
}
