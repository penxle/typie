use std::collections::BTreeMap;
use std::fmt::Write as _;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use editor_crdt::Dot;
use editor_model::{
    ChildView, LeafView, Modifier, ModifierType, NodeType, NodeView, PlainNode, ProjectedDoc,
};
use editor_state::State;
use strum::IntoEnumIterator;

use crate::error::{XmlError, XmlErrorDetail};
use crate::lexer::is_forbidden;
use crate::names::{
    element_name, inline_modifier_attr, is_opaque, is_textblock, modifier_type_name,
    modifier_value, node_attrs, writable_modifiers,
};

/// The bytes are the `base` attribute's identity, so the order the caller
/// happens to hold the heads in must not reach them.
pub fn encode_base(dots: &[Dot]) -> Result<String, XmlError> {
    let mut dots = dots.to_vec();
    dots.sort();
    let bytes = editor_codec::encode_dots(&dots)
        .map_err(|e| XmlError::internal(format!("encode_dots: {e}")))?;
    Ok(STANDARD.encode(bytes))
}

pub fn decode_base(s: &str) -> Result<Vec<Dot>, XmlError> {
    let bytes = STANDARD
        .decode(s)
        .map_err(|_| XmlError::new(XmlErrorDetail::BaseUndecodable))?;
    editor_codec::decode_dots(&bytes).map_err(|_| XmlError::new(XmlErrorDetail::BaseUndecodable))
}

pub fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;")
}

/// A value the lexer would refuse to read back cannot be written at all: the
/// file would not survive its own round trip.
pub fn escape_attr(s: &str) -> Result<String, XmlError> {
    if let Some(ch) = s.chars().find(|c| is_forbidden(*c)) {
        return Err(XmlError::new(XmlErrorDetail::ForbiddenCharInDocument {
            codepoint: ch as u32,
        }));
    }
    Ok(escape_text(s).replace('"', "&quot;"))
}

pub fn to_xml(state: &State, base: &[Dot]) -> Result<String, XmlError> {
    let view = state.view();
    let root = view.root().ok_or_else(|| XmlError::internal("no root"))?;
    let projected = state.projected.projected();
    let mut out = String::new();
    write_block(projected, &root, 0, Some(base), &[], &mut out)?;
    Ok(out)
}

fn write_block(
    projected: &ProjectedDoc,
    nv: &NodeView<'_>,
    depth: usize,
    base: Option<&[Dot]>,
    path: &[NodeType],
    out: &mut String,
) -> Result<(), XmlError> {
    let node = nv.node().to_plain();
    let node_type = node.as_type();
    let name =
        element_name(node_type).ok_or_else(|| XmlError::internal("text node in block position"))?;
    let mut here = path.to_vec();
    here.push(node_type);
    indent(out, depth);
    out.push('<');
    out.push_str(name);
    write!(out, " dot=\"{}\"", nv.id()).unwrap();
    if let Some(base) = base {
        write!(out, " base=\"{}\"", encode_base(base)?).unwrap();
    }
    for (k, v) in node_attrs(&node) {
        write!(out, " attr:{k}=\"{}\"", escape_attr(&v)?).unwrap();
    }
    if let Some(dot) = nv.dot() {
        if let Some(mods) = projected.block_modifiers.get(&dot) {
            for (ty, m) in writable_modifiers(mods, &here) {
                write!(
                    out,
                    " mod:{}=\"{}\"",
                    modifier_type_name(ty),
                    escape_attr(&modifier_value(&m).unwrap_or_default())?
                )
                .unwrap();
            }
        }
        if is_textblock(node_type) {
            for (ty, m) in projected.carry_modifiers(dot) {
                write!(
                    out,
                    " carry:{}=\"{}\"",
                    modifier_type_name(ty),
                    escape_attr(&modifier_value(&m).unwrap_or_default())?
                )
                .unwrap();
            }
        }
    }
    if is_opaque(node_type) || (nv.child_count() == 0 && !is_textblock(node_type)) {
        out.push_str("/>\n");
        return Ok(());
    }
    out.push('>');
    if is_textblock(node_type) {
        write_inline(nv, out)?;
        out.push_str("</");
        out.push_str(name);
        out.push_str(">\n");
        return Ok(());
    }
    out.push('\n');
    for (slot, child) in nv.children().enumerate() {
        match child {
            ChildView::Block(b) => write_block(projected, &b, depth + 1, None, &here, out)?,
            ChildView::Leaf(l) => write_atom(nv, slot, &l, depth + 1, &here, out)?,
        }
    }
    indent(out, depth);
    out.push_str("</");
    out.push_str(name);
    out.push_str(">\n");
    Ok(())
}

/// An atom leaf in a block slot. Modifiers come from the leaf's own span state
/// — the store `to_plain` reports for an atom — and a leaf with no node of its
/// own stays out of the file, as it stays out of `to_plain`.
fn write_atom(
    nv: &NodeView<'_>,
    slot: usize,
    leaf: &LeafView<'_>,
    depth: usize,
    path: &[NodeType],
    out: &mut String,
) -> Result<(), XmlError> {
    let Some(node) = leaf.node() else {
        return Ok(());
    };
    let plain = node.to_plain();
    let name = element_name(plain.as_type()).ok_or_else(|| XmlError::internal("unnamed atom"))?;
    let mut here = path.to_vec();
    here.push(plain.as_type());
    indent(out, depth);
    out.push('<');
    out.push_str(name);
    write!(out, " dot=\"{}\"", leaf.dot()).unwrap();
    for (k, v) in node_attrs(&plain) {
        write!(out, " attr:{k}=\"{}\"", escape_attr(&v)?).unwrap();
    }
    if let Some(state) = nv.leaf_state_at(slot) {
        let own: BTreeMap<ModifierType, Modifier> = state
            .own
            .iter()
            .map(|(ty, o)| (*ty, o.value.clone()))
            .collect();
        for (ty, m) in writable_modifiers(&own, &here) {
            write!(
                out,
                " mod:{}=\"{}\"",
                modifier_type_name(ty),
                escape_attr(&modifier_value(&m).unwrap_or_default())?
            )
            .unwrap();
        }
    }
    out.push_str("/>\n");
    Ok(())
}

fn write_inline(nv: &NodeView<'_>, out: &mut String) -> Result<(), XmlError> {
    let mut run_own: Option<BTreeMap<ModifierType, Modifier>> = None;
    for (slot, child) in nv.children().enumerate() {
        let ChildView::Leaf(leaf) = child else {
            return Err(XmlError::internal("block inside textblock"));
        };
        let own: BTreeMap<ModifierType, Modifier> = nv
            .leaf_state_at(slot)
            .map(|s| s.own.iter().map(|(t, o)| (*t, o.value.clone())).collect())
            .unwrap_or_default();
        if run_own.as_ref() != Some(&own) {
            if let Some(prev) = run_own.take() {
                close_modifiers(&prev, out);
            }
            open_modifiers(&own, out)?;
            run_own = Some(own);
        }
        if let Some(ch) = leaf.as_char() {
            if matches!(ch, '\n' | '\r' | '\t') || is_forbidden(ch) {
                return Err(XmlError::new(XmlErrorDetail::ForbiddenCharInDocument {
                    codepoint: ch as u32,
                }));
            }
            out.push_str(&escape_text(&ch.to_string()));
        } else {
            let plain = leaf
                .node()
                .map(|node| node.to_plain())
                .unwrap_or(PlainNode::Unknown);
            let name =
                element_name(plain.as_type()).ok_or_else(|| XmlError::internal("unnamed atom"))?;
            out.push('<');
            out.push_str(name);
            for (k, v) in node_attrs(&plain) {
                write!(out, " attr:{k}=\"{}\"", escape_attr(&v)?).unwrap();
            }
            out.push_str("/>");
        }
    }
    if let Some(prev) = run_own.take() {
        close_modifiers(&prev, out);
    }
    Ok(())
}

fn open_modifiers(
    own: &BTreeMap<ModifierType, Modifier>,
    out: &mut String,
) -> Result<(), XmlError> {
    for ty in ModifierType::iter() {
        let Some(m) = own.get(&ty) else { continue };
        out.push('<');
        out.push_str(&modifier_type_name(ty));
        if let (Some(attr), Some(v)) = (inline_modifier_attr(ty), modifier_value(m)) {
            write!(out, " {attr}=\"{}\"", escape_attr(&v)?).unwrap();
        }
        out.push('>');
    }
    Ok(())
}

fn close_modifiers(own: &BTreeMap<ModifierType, Modifier>, out: &mut String) {
    for ty in ModifierType::iter().collect::<Vec<_>>().into_iter().rev() {
        if own.contains_key(&ty) {
            out.push_str("</");
            out.push_str(&modifier_type_name(ty));
            out.push('>');
        }
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::live_heads;
    use editor_macros::state;

    const ROOT_ATTRS: &str = concat!(
        " attr:layout_mode=\"continuous\" attr:max_width=\"600\"",
        " mod:font_size=\"1200\" mod:font_family=\"Pretendard\" mod:font_weight=\"400\"",
        " mod:letter_spacing=\"0\" mod:line_height=\"160\" mod:block_gap=\"100\"",
        " mod:paragraph_indent=\"100\" mod:alignment=\"left\"",
    );

    fn block_child(state: &State, slot: usize) -> Dot {
        let view = state.view();
        let root = view.root().expect("root");
        match root.child_at(slot).expect("child") {
            ChildView::Block(b) => b.id(),
            ChildView::Leaf(_) => panic!("root child is a block"),
        }
    }

    #[test]
    fn a_control_character_in_an_attribute_value_is_refused() {
        let (state, _p1, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    image(id: Some("IMG\u{1}1".to_string()))
                    paragraph { }
                }
            }
            selection: (p1, 0)
        };

        let err = to_xml(&state, &live_heads(&state)).unwrap_err();

        assert_eq!(
            *err.detail,
            XmlErrorDetail::ForbiddenCharInDocument { codepoint: 1 }
        );
    }

    #[test]
    fn the_base_attribute_does_not_depend_on_the_order_of_the_heads() {
        let a = Dot::new(3, 7);
        let b = Dot::new(1, 2);
        assert_eq!(encode_base(&[a, b]).unwrap(), encode_base(&[b, a]).unwrap());
    }

    #[test]
    fn paragraph_with_runs_and_atom() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text(" 안녕") [bold] text("하세요") hard_break text("&<") [italic] } } }
            selection: (p1, 0)
        };
        let base = live_heads(&state);
        let xml = to_xml(&state, &base).unwrap();
        let root_dot = state.view().root().unwrap().id().to_string();
        let expected = format!(
            "<root dot=\"{root_dot}\" base=\"{b}\"{ROOT_ATTRS}>\n  <paragraph dot=\"{p1}\"><bold> 안녕</bold>하세요<hard_break/><italic>&amp;&lt;</italic></paragraph>\n</root>\n",
            b = encode_base(&base).unwrap(),
        );
        assert_eq!(xml, expected);
    }

    #[test]
    fn block_modifiers_carry_and_nested_container() {
        let (state, p1) = state! {
            doc { root { blockquote(variant: BlockquoteVariant::LeftQuote) { p1: paragraph [alignment(Alignment::Center)] carry([bold]) { text("가") } } } }
            selection: (p1, 0)
        };
        let base = live_heads(&state);
        let xml = to_xml(&state, &base).unwrap();
        let root_dot = state.view().root().unwrap().id().to_string();
        let blockquote = block_child(&state, 0);
        let scaffold = block_child(&state, 1);
        let expected = format!(
            "<root dot=\"{root_dot}\" base=\"{b}\"{ROOT_ATTRS}>\n  <blockquote dot=\"{blockquote}\" attr:variant=\"left_quote\">\n    <paragraph dot=\"{p1}\" mod:alignment=\"center\" carry:bold=\"\">가</paragraph>\n  </blockquote>\n  <paragraph dot=\"{scaffold}\"></paragraph>\n</root>\n",
            b = encode_base(&base).unwrap(),
        );
        assert_eq!(xml, expected);
    }

    #[test]
    fn block_atoms_are_written_as_self_closed_elements_with_dot_attrs_and_modifiers() {
        let (state, p1, image, rule, p2) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    image: image(id: Some("IMG1".to_string())) [alignment(Alignment::Center)]
                    rule: horizontal_rule
                    p2: paragraph { text("b") }
                }
            }
            selection: (p1, 0)
        };
        let base = live_heads(&state);
        let xml = to_xml(&state, &base).unwrap();
        let root_dot = state.view().root().unwrap().id().to_string();
        let expected = format!(
            "<root dot=\"{root_dot}\" base=\"{b}\"{ROOT_ATTRS}>\n  <paragraph dot=\"{p1}\">a</paragraph>\n  <image dot=\"{image}\" attr:id=\"IMG1\" attr:proportion=\"100\" mod:alignment=\"center\"/>\n  <horizontal_rule dot=\"{rule}\" attr:variant=\"line\"/>\n  <paragraph dot=\"{p2}\">b</paragraph>\n</root>\n",
            b = encode_base(&base).unwrap(),
        );
        assert_eq!(xml, expected);
    }

    #[test]
    fn a_block_atom_in_a_nested_container_is_written_at_its_depth() {
        let (state, _title, image) = state! {
            doc {
                root {
                    fold {
                        title: fold_title { }
                        fold_content {
                            image: image(id: Some("IMG1".to_string()))
                            paragraph { text("x") }
                        }
                    }
                    paragraph { }
                }
            }
            selection: (title, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        assert!(
            xml.contains(&format!(
                "      <image dot=\"{image}\" attr:id=\"IMG1\" attr:proportion=\"100\"/>\n"
            )),
            "unexpected xml: {xml}"
        );
    }

    #[test]
    fn a_block_atom_round_trips_through_the_reader() {
        let (state, _p1, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    image: image(id: Some("IMG1".to_string())) [alignment(Alignment::Center)]
                    rule: horizontal_rule
                    p2: paragraph { text("b") }
                }
            }
            selection: (p1, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let tree = crate::reader::from_xml(&xml).unwrap();
        assert_eq!(tree.to_plain_doc(), state.to_plain());
    }

    #[test]
    fn text_with_newline_is_a_serialization_error() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("a\nb") } } }
            selection: (p1, 0)
        };
        let err = to_xml(&state, &live_heads(&state)).unwrap_err();
        assert_eq!(
            *err.detail,
            crate::XmlErrorDetail::ForbiddenCharInDocument { codepoint: 0x0A }
        );
    }

    #[test]
    fn inline_modifiers_nest_in_enum_order_and_never_merge_across_runs() {
        assert!(ModifierType::Bold < ModifierType::Link);

        let (state, p1) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("a") [bold, link(href: "https://x?a=1&b=2".to_string())]
                        text("b") [bold]
                    }
                }
            }
            selection: (p1, 0)
        };
        let base = live_heads(&state);
        let xml = to_xml(&state, &base).unwrap();
        let root_dot = state.view().root().unwrap().id().to_string();
        let expected = format!(
            "<root dot=\"{root_dot}\" base=\"{b}\"{ROOT_ATTRS}>\n  <paragraph dot=\"{p1}\"><bold><link href=\"https://x?a=1&amp;b=2\">a</link></bold><bold>b</bold></paragraph>\n</root>\n",
            b = encode_base(&base).unwrap(),
        );
        assert_eq!(xml, expected);
    }

    #[test]
    fn invalid_block_modifier_is_not_written() {
        let (state, p1) = state! {
            doc { root { p1: paragraph [font_size(99)] { text("a") } } }
            selection: (p1, 0)
        };
        assert!(
            state
                .projected
                .projected()
                .block_modifiers
                .get(&p1)
                .is_some_and(|m| m.contains_key(&ModifierType::FontSize)),
            "the fixture must actually hold the out-of-range block modifier"
        );
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        assert!(xml.contains(&format!("<paragraph dot=\"{p1}\">a</paragraph>")));
        assert!(!xml.contains("mod:font_size=\"99\""));
    }

    #[test]
    fn unknown_inline_leaf_is_written_as_an_opaque_element() {
        use editor_crdt::ListOp;
        use editor_model::{EditOp, SeqItem};

        let (mut state, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 0)
        };
        state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 2,
                item: SeqItem::Unknown {
                    tag: 999,
                    bytes: vec![0xAA],
                },
            }))
            .unwrap();
        let base = live_heads(&state);
        let xml = to_xml(&state, &base).unwrap();
        let root_dot = state.view().root().unwrap().id().to_string();
        let expected = format!(
            "<root dot=\"{root_dot}\" base=\"{b}\"{ROOT_ATTRS}>\n  <paragraph dot=\"{p1}\">a<unknown/>b</paragraph>\n</root>\n",
            b = encode_base(&base).unwrap(),
        );
        assert_eq!(xml, expected);
    }

    #[test]
    fn opaque_block_is_written_self_closed_without_its_children() {
        use editor_crdt::ListOp;
        use editor_model::{EditOp, NodeType, SeqItem};

        let (mut state, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 0)
        };
        let unknown = state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: SeqItem::Block {
                    node_type: NodeType::Unknown,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }))
            .unwrap()
            .id;
        let inner = state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 4,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, unknown],
                    attrs: vec![],
                },
            }))
            .unwrap()
            .id;
        assert_eq!(
            state
                .view()
                .node(unknown)
                .expect("unknown block")
                .child_blocks()
                .map(|b| b.id())
                .collect::<Vec<_>>(),
            vec![inner],
            "the fixture must actually hold a child under the opaque block"
        );

        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        assert!(xml.contains(&format!("<unknown dot=\"{unknown}\"/>")));
        assert!(!xml.contains(&format!("dot=\"{inner}\"")));
    }

    #[test]
    fn base_round_trips_through_the_attribute() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("a") } } }
            selection: (p1, 0)
        };
        let base = live_heads(&state);
        assert!(!base.is_empty());
        assert_eq!(decode_base(&encode_base(&base).unwrap()).unwrap(), base);
        assert_eq!(
            *decode_base("not base64!").unwrap_err().detail,
            XmlErrorDetail::BaseUndecodable
        );
    }
}
