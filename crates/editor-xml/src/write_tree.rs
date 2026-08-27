use std::collections::BTreeMap;
use std::fmt::Write as _;

use editor_model::{Modifier, ModifierType, NodeType};

use crate::address::NodePath;
use crate::error::{XmlError, XmlErrorDetail};
use crate::lexer::is_forbidden;
use crate::names::{
    element_name, is_opaque, is_textblock, modifier_type_name, modifier_value, node_attrs,
    writable_modifiers,
};
use crate::tree::{InlineEntry, InlineLeaf, XmlChild, XmlNode, XmlTree};
use crate::writer::{
    close_modifiers, encode_base, escape_attr, escape_text, indent, open_modifiers,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSpan {
    pub path: NodePath,
    pub start_line: u32,
    pub end_line: u32,
}

struct Out {
    text: String,
    line: u32,
    spans: Vec<BlockSpan>,
}

impl Out {
    fn push_line_end(&mut self) {
        self.text.push('\n');
        self.line += 1;
    }
}

pub fn write_tree(tree: &XmlTree) -> Result<(String, Vec<BlockSpan>), XmlError> {
    let mut out = Out {
        text: String::new(),
        line: 1,
        spans: Vec::new(),
    };
    write_node(
        &tree.root,
        0,
        Some(&tree.base),
        &[],
        &mut Vec::new(),
        &mut out,
    )?;
    Ok((out.text, out.spans))
}

pub fn write_fragment(nodes: &[&XmlNode], ancestors: &[NodeType]) -> Result<String, XmlError> {
    let mut out = Out {
        text: String::new(),
        line: 1,
        spans: Vec::new(),
    };
    for (i, node) in nodes.iter().enumerate() {
        write_node(node, 0, None, ancestors, &mut vec![i], &mut out)?;
    }
    Ok(out.text)
}

fn write_node(
    node: &XmlNode,
    depth: usize,
    base: Option<&[editor_crdt::Dot]>,
    types: &[NodeType],
    path: &mut NodePath,
    out: &mut Out,
) -> Result<(), XmlError> {
    let node_type = node.node.as_type();
    let name =
        element_name(node_type).ok_or_else(|| XmlError::internal("text node in block position"))?;
    let mut here = types.to_vec();
    here.push(node_type);
    let start_line = out.line;
    indent(&mut out.text, depth);
    out.text.push('<');
    out.text.push_str(name);
    if let Some(dot) = node.dot {
        write!(out.text, " dot=\"{dot}\"").unwrap();
    }
    if let Some(base) = base {
        write!(out.text, " base=\"{}\"", encode_base(base)?).unwrap();
    }
    for (k, v) in node_attrs(&node.node) {
        write!(out.text, " attr:{k}=\"{}\"", escape_attr(&v)?).unwrap();
    }
    for (ty, m) in writable_modifiers(&node.modifiers, &here) {
        write!(
            out.text,
            " mod:{}=\"{}\"",
            modifier_type_name(ty),
            escape_attr(&modifier_value(&m).unwrap_or_default())?
        )
        .unwrap();
    }
    if is_textblock(node_type) {
        for (ty, m) in &node.carry {
            if !m.is_valid() {
                continue;
            }
            write!(
                out.text,
                " carry:{}=\"{}\"",
                modifier_type_name(*ty),
                escape_attr(&modifier_value(m).unwrap_or_default())?
            )
            .unwrap();
        }
    }
    if is_opaque(node_type) || (node.children.is_empty() && !is_textblock(node_type)) {
        out.text.push_str("/>");
        out.push_line_end();
        out.spans.push(BlockSpan {
            path: path.clone(),
            start_line,
            end_line: start_line,
        });
        return Ok(());
    }
    out.text.push('>');
    if is_textblock(node_type) {
        write_inline(node, &mut out.text)?;
        out.text.push_str("</");
        out.text.push_str(name);
        out.text.push('>');
        out.push_line_end();
        out.spans.push(BlockSpan {
            path: path.clone(),
            start_line,
            end_line: start_line,
        });
        return Ok(());
    }
    out.push_line_end();
    for (i, child) in node.block_children().enumerate() {
        path.push(i);
        write_node(child, depth + 1, None, &here, path, out)?;
        path.pop();
    }
    indent(&mut out.text, depth);
    out.text.push_str("</");
    out.text.push_str(name);
    out.text.push('>');
    let end_line = out.line;
    out.push_line_end();
    out.spans.push(BlockSpan {
        path: path.clone(),
        start_line,
        end_line,
    });
    Ok(())
}

fn write_inline(node: &XmlNode, out: &mut String) -> Result<(), XmlError> {
    let mut run_own: Option<BTreeMap<ModifierType, Modifier>> = None;
    for child in &node.children {
        let XmlChild::Inline(item) = child else {
            return Err(XmlError::internal("block inside textblock"));
        };
        if run_own.as_ref() != Some(&item.own) {
            if let Some(prev) = run_own.take() {
                close_modifiers(&prev, out);
            }
            open_modifiers(&item.own, out)?;
            run_own = Some(item.own.clone());
        }
        write_leaf(item, out)?;
    }
    if let Some(prev) = run_own.take() {
        close_modifiers(&prev, out);
    }
    Ok(())
}

fn write_leaf(item: &InlineEntry, out: &mut String) -> Result<(), XmlError> {
    match &item.leaf {
        InlineLeaf::Char(ch) => {
            if matches!(ch, '\n' | '\r' | '\t') || is_forbidden(*ch) {
                return Err(XmlError::new(XmlErrorDetail::ForbiddenCharInDocument {
                    codepoint: *ch as u32,
                }));
            }
            out.push_str(&escape_text(&ch.to_string()));
        }
        InlineLeaf::Atom(plain) => {
            let name =
                element_name(plain.as_type()).ok_or_else(|| XmlError::internal("unnamed atom"))?;
            out.push('<');
            out.push_str(name);
            for (k, v) in node_attrs(plain) {
                write!(out, " attr:{k}=\"{}\"", escape_attr(&v)?).unwrap();
            }
            out.push_str("/>");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::reader::{from_xml, from_xml_fragment};
    use crate::test_support::{arb_plain_doc, live_heads};
    use crate::writer::to_xml;

    fn load(doc: &editor_model::PlainDoc) -> editor_state::State {
        editor_state::State::from_plain(doc).unwrap()
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

        #[test]
        fn writes_exactly_what_the_state_writer_writes(doc in arb_plain_doc()) {
            let state = load(&doc);
            let xml = to_xml(&state, &live_heads(&state)).unwrap();
            let tree = from_xml(&xml).unwrap();
            let (again, spans) = write_tree(&tree).unwrap();
            prop_assert_eq!(&again, &xml);
            let root = spans.iter().find(|s| s.path.is_empty()).unwrap();
            prop_assert_eq!(root.start_line, 1);
            prop_assert_eq!(root.end_line as usize, xml.trim_end_matches('\n').lines().count());
        }

        #[test]
        fn round_trips_through_the_reader(doc in arb_plain_doc()) {
            let state = load(&doc);
            let tree = from_xml(&to_xml(&state, &live_heads(&state)).unwrap()).unwrap();
            let (text, _) = write_tree(&tree).unwrap();
            let back = from_xml(&text).unwrap();
            prop_assert_eq!(back.to_plain_doc(), tree.to_plain_doc());
            prop_assert_eq!(back.base, tree.base);
        }
    }

    #[test]
    fn spans_cover_each_block_from_open_to_close_tag() {
        let base = crate::writer::encode_base(&[]).unwrap();
        let tree = from_xml(&format!(
            "<root dot=\"1_0\" base=\"{base}\" attr:layout_mode=\"continuous\" attr:max_width=\"600\">\
             <paragraph dot=\"1_1\">a</paragraph>\
             <blockquote dot=\"1_2\"><paragraph dot=\"1_3\">b</paragraph></blockquote>\
             <horizontal_rule dot=\"1_4\"/><image dot=\"1_5\" attr:id=\"img\"/><paragraph/></root>"
        ))
        .unwrap();
        let (text, spans) = write_tree(&tree).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let span = |path: &[usize]| spans.iter().find(|s| s.path == path).unwrap();
        assert_eq!(span(&[0]).start_line, 2);
        assert_eq!(span(&[0]).end_line, 2);
        assert_eq!(span(&[1]).start_line, 3);
        assert_eq!(span(&[1]).end_line, 5);
        assert_eq!(span(&[1, 0]).start_line, 4);
        assert!(
            lines[5]
                .trim()
                .starts_with("<horizontal_rule dot=\"1_4\" attr:variant=\"line\"")
        );
        assert!(lines[5].trim().ends_with("/>"));
        assert!(
            lines[6]
                .trim()
                .starts_with("<image dot=\"1_5\" attr:id=\"img\"")
        );
        assert!(lines[6].trim().ends_with("/>"));
        assert_eq!(lines[7].trim(), "<paragraph></paragraph>");
        assert_eq!(span(&[]).end_line as usize, lines.len());
    }

    #[test]
    fn fragments_are_written_at_depth_zero_without_base() {
        let nodes = from_xml_fragment("<paragraph dot=\"1_1\"><bold>a</bold>&amp;</paragraph><blockquote><paragraph/></blockquote>").unwrap();
        let refs: Vec<&XmlNode> = nodes.iter().collect();
        assert_eq!(
            write_fragment(&refs, &[]).unwrap(),
            "<paragraph dot=\"1_1\"><bold>a</bold>&amp;</paragraph>\n<blockquote attr:variant=\"left_line\">\n  <paragraph></paragraph>\n</blockquote>\n"
        );
    }

    #[test]
    fn a_fragment_keeps_the_modifiers_its_ancestor_context_allows() {
        let base = crate::writer::encode_base(&[]).unwrap();
        let tree = from_xml(&format!(
            "<root dot=\"1_0\" base=\"{base}\" attr:layout_mode=\"continuous\" attr:max_width=\"600\">\
             <paragraph dot=\"1_1\" mod:paragraph_indent=\"100\">a</paragraph><paragraph/></root>"
        ))
        .unwrap();
        let p = tree.root.block_children().next().unwrap();
        assert_eq!(
            write_fragment(&[p], &[NodeType::Root]).unwrap(),
            "<paragraph dot=\"1_1\" mod:paragraph_indent=\"100\">a</paragraph>\n"
        );
        assert_eq!(
            write_fragment(&[p], &[]).unwrap(),
            "<paragraph dot=\"1_1\">a</paragraph>\n"
        );
    }
}
