use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;

use editor_crdt::Dot;
use editor_model::{Modifier, ModifierType, NodeType, PlainNode, context_allows};

use crate::error::{Pos, XmlError, XmlErrorDetail};
use crate::lexer::{Token, tokenize};
use crate::names::{
    Attrs, element_name, inline_modifier_attr, is_inline_atom, is_textblock, modifier_fits_context,
    modifier_from, modifier_type_name, modifier_type_of, node_from_attrs, node_type_of,
};
use crate::tree::{InlineEntry, InlineLeaf, XmlChild, XmlNode, XmlTree};
use crate::writer::decode_base;

pub fn from_xml(input: &str) -> Result<XmlTree, XmlError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser { tokens, index: 0 };
    parser.skip_blank_text();
    let Some(Token::Open {
        name, attrs, pos, ..
    }) = parser.peek().cloned()
    else {
        return Err(match parser.peek() {
            Some(tok) => XmlError::at(token_pos(tok), XmlErrorDetail::RootMissing),
            None => XmlError::new(XmlErrorDetail::RootMissing),
        });
    };
    if name != "root" {
        return Err(XmlError::at(pos, XmlErrorDetail::RootNotRoot { name }));
    }
    if !attrs.iter().any(|(key, _, _)| key == "base") {
        return Err(XmlError::new(XmlErrorDetail::BaseMissing));
    }
    let (root, base) = parser.element(true)?;
    parser.skip_blank_text();
    if let Some(tok) = parser.peek() {
        return Err(XmlError::at(
            token_pos(tok),
            XmlErrorDetail::TrailingContent,
        ));
    }
    let base = base.ok_or_else(|| XmlError::new(XmlErrorDetail::BaseMissing))?;
    let tree = XmlTree { base, root };
    validate_schema(&tree.root, &[])?;
    validate_dots_unique(&tree.root)?;
    Ok(tree)
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

fn token_pos(tok: &Token) -> Pos {
    match tok {
        Token::Open { pos, .. } | Token::Close { pos, .. } | Token::Text { pos, .. } => *pos,
    }
}

fn is_blank(text: &str) -> bool {
    text.chars().all(|c| matches!(c, ' ' | '\n' | '\r' | '\t'))
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn next(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.index).cloned();
        self.index += 1;
        tok
    }

    fn skip_blank_text(&mut self) {
        while let Some(Token::Text { text, .. }) = self.peek() {
            if !is_blank(text) {
                break;
            }
            self.index += 1;
        }
    }

    fn element(&mut self, is_root: bool) -> Result<(XmlNode, Option<Vec<Dot>>), XmlError> {
        let Some(Token::Open {
            name,
            attrs,
            self_closing,
            pos,
        }) = self.next()
        else {
            return Err(XmlError::internal("element without an open token"));
        };
        let node_type = node_type_of(&name).ok_or_else(|| {
            XmlError::at(
                pos,
                XmlErrorDetail::UnknownElement {
                    name: name.clone(),
                    hint: element_hint(&name),
                },
            )
        })?;
        if node_type == NodeType::Root && !is_root {
            return Err(XmlError::at(pos, XmlErrorDetail::MultipleRoots));
        }
        let mut dot: Option<Dot> = None;
        let mut base: Option<Vec<Dot>> = None;
        let mut node_attrs = Attrs::new();
        let mut modifiers: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
        let mut carry: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
        for (key, value, apos) in attrs {
            if key == "dot" {
                dot = Some(
                    Dot::from_str(&value)
                        .map_err(|_| XmlError::at(apos, XmlErrorDetail::DotInvalid { value }))?,
                );
            } else if key == "base" {
                if !is_root {
                    return Err(XmlError::at(apos, XmlErrorDetail::BaseOnNonRoot));
                }
                base = Some(
                    decode_base(&value)
                        .map_err(|_| XmlError::at(apos, XmlErrorDetail::BaseUndecodable))?,
                );
            } else if let Some(field) = key.strip_prefix("attr:") {
                node_attrs.insert(field.to_owned(), value);
            } else if let Some(ty_name) = key.strip_prefix("mod:") {
                let ty = modifier_type_of(ty_name).ok_or_else(|| {
                    XmlError::at(
                        apos,
                        XmlErrorDetail::UnknownModifier {
                            prefix: "mod".to_owned(),
                            name: ty_name.to_owned(),
                        },
                    )
                })?;
                let modifier = modifier_from(ty, &value).map_err(|d| XmlError::at(apos, d))?;
                modifiers.insert(ty, modifier);
            } else if let Some(ty_name) = key.strip_prefix("carry:") {
                let ty = modifier_type_of(ty_name).ok_or_else(|| {
                    XmlError::at(
                        apos,
                        XmlErrorDetail::UnknownModifier {
                            prefix: "carry".to_owned(),
                            name: ty_name.to_owned(),
                        },
                    )
                })?;
                if !ty.is_carry_kind() {
                    return Err(XmlError::at(
                        apos,
                        XmlErrorDetail::ModifierNotCarryKind {
                            name: ty_name.to_owned(),
                        },
                    ));
                }
                let modifier = modifier_from(ty, &value).map_err(|d| XmlError::at(apos, d))?;
                carry.insert(ty, modifier);
            } else {
                return Err(XmlError::at(
                    apos,
                    XmlErrorDetail::UnknownAttribute {
                        element: name.clone(),
                        attr: key,
                    },
                ));
            }
        }
        let node = node_from_attrs(node_type, &node_attrs).map_err(|d| XmlError::at(pos, d))?;
        if !carry.is_empty() && !is_textblock(node_type) {
            return Err(XmlError::at(
                pos,
                XmlErrorDetail::CarryOnNonTextblock {
                    element: name.clone(),
                },
            ));
        }
        let mut children: Vec<XmlChild> = Vec::new();
        if !self_closing {
            if is_textblock(node_type) {
                self.inline_content(&name, &mut BTreeMap::new(), &mut children)?;
            } else {
                self.block_content(&name, &mut children)?;
            }
        }
        if matches!(
            node_type,
            NodeType::Unknown
                | NodeType::Archived
                | NodeType::Image
                | NodeType::File
                | NodeType::Embed
        ) && dot.is_none()
        {
            return Err(XmlError::at(
                pos,
                XmlErrorDetail::OpaqueNeedsDot { element: name },
            ));
        }
        Ok((
            XmlNode {
                dot,
                pos,
                node,
                modifiers,
                carry,
                children,
            },
            base,
        ))
    }

    fn block_content(&mut self, name: &str, out: &mut Vec<XmlChild>) -> Result<(), XmlError> {
        loop {
            match self.peek().cloned() {
                None => {
                    return Err(XmlError::new(XmlErrorDetail::ElementUnclosed {
                        name: name.to_owned(),
                    }));
                }
                Some(Token::Close { .. }) => {
                    self.index += 1;
                    return Ok(());
                }
                Some(Token::Text { text, pos }) => {
                    if !is_blank(&text) {
                        return Err(XmlError::at(
                            pos,
                            XmlErrorDetail::TextInContainer {
                                element: name.to_owned(),
                            },
                        ));
                    }
                    self.index += 1;
                }
                Some(Token::Open { .. }) => {
                    let (node, _) = self.element(false)?;
                    out.push(XmlChild::Block(node));
                }
            }
        }
    }

    fn inline_content(
        &mut self,
        name: &str,
        own: &mut BTreeMap<ModifierType, Modifier>,
        out: &mut Vec<XmlChild>,
    ) -> Result<(), XmlError> {
        loop {
            match self.next() {
                None => {
                    return Err(XmlError::new(XmlErrorDetail::ElementUnclosed {
                        name: name.to_owned(),
                    }));
                }
                Some(Token::Close { .. }) => return Ok(()),
                Some(Token::Text { text, pos }) => {
                    for (column, ch) in (pos.column..).zip(text.chars()) {
                        let at = Pos {
                            line: pos.line,
                            column,
                        };
                        match ch {
                            '\n' | '\r' => {
                                return Err(XmlError::at(at, XmlErrorDetail::NewlineInText));
                            }
                            '\t' => return Err(XmlError::at(at, XmlErrorDetail::TabInText)),
                            _ => {}
                        }
                        out.push(XmlChild::Inline(InlineEntry {
                            pos: at,
                            leaf: InlineLeaf::Char(ch),
                            own: own.clone(),
                        }));
                    }
                }
                Some(Token::Open {
                    name: child,
                    attrs,
                    self_closing,
                    pos,
                }) => {
                    let child_node_type = node_type_of(&child);
                    if let Some(atom_type) =
                        child_node_type.filter(|t| is_inline_atom(*t) || *t == NodeType::Unknown)
                    {
                        let node = self.inline_atom(atom_type, &child, attrs, self_closing, pos)?;
                        out.push(XmlChild::Inline(InlineEntry {
                            pos,
                            leaf: InlineLeaf::Atom(node),
                            own: own.clone(),
                        }));
                        continue;
                    }
                    if child_node_type.is_some() {
                        return Err(XmlError::at(
                            pos,
                            XmlErrorDetail::BlockInsideTextblock {
                                parent: name.to_owned(),
                                child,
                            },
                        ));
                    }
                    let Some(ty) = modifier_type_of(&child).filter(|t| t.is_text_applicable())
                    else {
                        return Err(XmlError::at(
                            pos,
                            XmlErrorDetail::UnknownElement {
                                hint: element_hint(&child),
                                name: child,
                            },
                        ));
                    };
                    let value = inline_modifier_value(ty, &child, attrs, pos)?;
                    let modifier = modifier_from(ty, &value).map_err(|d| XmlError::at(pos, d))?;
                    if self_closing {
                        continue;
                    }
                    let previous = own.insert(ty, modifier);
                    self.inline_content(&child, own, out)?;
                    match previous {
                        Some(p) => own.insert(ty, p),
                        None => own.remove(&ty),
                    };
                }
            }
        }
    }

    fn inline_atom(
        &mut self,
        atom_type: NodeType,
        child: &str,
        attrs: Vec<(String, String, Pos)>,
        self_closing: bool,
        pos: Pos,
    ) -> Result<PlainNode, XmlError> {
        let mut node_attrs = Attrs::new();
        for (key, value, apos) in attrs {
            match key
                .strip_prefix("attr:")
                .filter(|_| atom_type != NodeType::Unknown)
            {
                Some(field) => {
                    node_attrs.insert(field.to_owned(), value);
                }
                None => {
                    return Err(XmlError::at(
                        apos,
                        XmlErrorDetail::AtomAttrNotAllowed {
                            element: child.to_owned(),
                            attr: key,
                        },
                    ));
                }
            }
        }
        let node = node_from_attrs(atom_type, &node_attrs).map_err(|d| XmlError::at(pos, d))?;
        if !self_closing && !matches!(self.next(), Some(Token::Close { .. })) {
            return Err(XmlError::at(
                pos,
                XmlErrorDetail::AtomHasContent {
                    element: child.to_owned(),
                },
            ));
        }
        Ok(node)
    }
}

fn inline_modifier_value(
    ty: ModifierType,
    child: &str,
    attrs: Vec<(String, String, Pos)>,
    pos: Pos,
) -> Result<String, XmlError> {
    let Some(expected) = inline_modifier_attr(ty) else {
        return match attrs.into_iter().next() {
            Some((key, _, apos)) => Err(XmlError::at(
                apos,
                XmlErrorDetail::InlineModifierAttrNotAllowed {
                    element: child.to_owned(),
                    attr: key,
                },
            )),
            None => Ok(String::new()),
        };
    };
    let mut found: Option<String> = None;
    for (key, value, apos) in attrs {
        if key != expected {
            return Err(XmlError::at(
                apos,
                XmlErrorDetail::InlineModifierAttrNotAllowed {
                    element: child.to_owned(),
                    attr: key,
                },
            ));
        }
        found = Some(value);
    }
    found.ok_or_else(|| {
        XmlError::at(
            pos,
            XmlErrorDetail::InlineModifierAttrMissing {
                element: child.to_owned(),
                attr: expected.to_owned(),
            },
        )
    })
}

fn element_hint(name: &str) -> Option<String> {
    let hint = match name {
        "b" | "strong" => "bold",
        "i" | "em" => "italic",
        "u" => "underline",
        "s" | "del" => "strikethrough",
        "p" => "paragraph",
        "br" => "hard_break",
        "a" => "link",
        "ul" | "ol" | "li" => "bullet_list",
        _ => return None,
    };
    Some(hint.to_owned())
}

fn type_name(t: NodeType) -> String {
    element_name(t).unwrap_or("text").to_owned()
}

fn leaf_type(leaf: &InlineLeaf) -> NodeType {
    match leaf {
        InlineLeaf::Char(_) => NodeType::Text,
        InlineLeaf::Atom(node) => node.as_type(),
    }
}

fn child_type(child: &XmlChild) -> NodeType {
    match child {
        XmlChild::Block(block) => block.node.as_type(),
        XmlChild::Inline(item) => leaf_type(&item.leaf),
    }
}

pub(crate) fn validate_schema(node: &XmlNode, path: &[NodeType]) -> Result<(), XmlError> {
    let t = node.node.as_type();
    let mut here = path.to_vec();
    here.push(t);
    for ty in node.modifiers.keys() {
        if !modifier_fits_context(*ty, &here) {
            return Err(XmlError::at(
                node.pos,
                XmlErrorDetail::BlockModifierNotAllowed {
                    modifier: modifier_type_name(*ty),
                    element: type_name(t),
                },
            ));
        }
    }
    if matches!(t, NodeType::Unknown | NodeType::Archived) {
        if !node.children.is_empty() {
            return Err(XmlError::at(
                node.pos,
                XmlErrorDetail::OpaqueHasChildren {
                    element: type_name(t),
                },
            ));
        }
        return Ok(());
    }
    let child_types: Vec<NodeType> = node
        .children
        .iter()
        .map(child_type)
        .filter(|t| *t != NodeType::Unknown)
        .collect();
    if let Err(e) = t.spec().content.validate(&child_types) {
        return Err(XmlError::at(
            node.pos,
            XmlErrorDetail::ContentRule {
                parent: type_name(t),
                allowed: t
                    .spec()
                    .content
                    .allowed_types()
                    .into_iter()
                    .map(type_name)
                    .collect(),
                got: child_types.iter().copied().map(type_name).collect(),
                rule: e.to_string(),
            },
        ));
    }
    for child in &node.children {
        match child {
            XmlChild::Block(block) => {
                let ct = block.node.as_type();
                if ct != NodeType::Unknown && !context_allows(&here, ct) {
                    return Err(XmlError::at(
                        block.pos,
                        XmlErrorDetail::ContextNotAllowed {
                            element: type_name(ct),
                        },
                    ));
                }
                validate_schema(block, &here)?;
            }
            XmlChild::Inline(item) => {
                let lt = leaf_type(&item.leaf);
                if lt == NodeType::Unknown {
                    continue;
                }
                if !context_allows(&here, lt) {
                    return Err(XmlError::at(
                        item.pos,
                        XmlErrorDetail::ContextNotAllowed {
                            element: type_name(lt),
                        },
                    ));
                }
                let mut leaf_path = here.clone();
                leaf_path.push(lt);
                for ty in item.own.keys() {
                    if !modifier_fits_context(*ty, &leaf_path) {
                        return Err(XmlError::at(
                            item.pos,
                            XmlErrorDetail::InlineModifierNotAllowed {
                                modifier: modifier_type_name(*ty),
                                leaf: type_name(lt),
                            },
                        ));
                    }
                }
            }
        }
    }
    if t == NodeType::Root
        && let Some(pos) = trailing_page_break(node)
    {
        return Err(XmlError::at(pos, XmlErrorDetail::TrailingPageBreak));
    }
    if t == NodeType::Table
        && let Some((row, expected, got)) = short_table_row(node)
    {
        return Err(XmlError::at(
            row.pos,
            XmlErrorDetail::TableNotRectangular { expected, got },
        ));
    }
    Ok(())
}

/// The first row of a table that holds fewer cells than the widest row. The
/// projection fills every row out to that width with scaffold cells, so the
/// document cannot hold the file as written.
fn short_table_row(table: &XmlNode) -> Option<(&XmlNode, usize, usize)> {
    fn cells(row: &XmlNode) -> usize {
        row.block_children()
            .filter(|cell| cell.node.as_type() == NodeType::TableCell)
            .count()
    }
    let rows: Vec<&XmlNode> = table
        .block_children()
        .filter(|row| row.node.as_type() == NodeType::TableRow)
        .collect();
    let width = rows.iter().map(|row| cells(row)).max()?;
    rows.into_iter()
        .map(|row| (row, width, cells(row)))
        .find(|(_, width, got)| got < width)
}

/// Where a root's last paragraph ends in a page break. The projection
/// completes such a root with a trailing scaffold paragraph, so the document
/// cannot hold the file as written. Unknown children are skipped at both
/// levels, the way the projection reads the last child.
fn trailing_page_break(root: &XmlNode) -> Option<Pos> {
    fn known_last(node: &XmlNode) -> Option<&XmlChild> {
        node.children
            .iter()
            .rev()
            .find(|child| child_type(child) != NodeType::Unknown)
    }
    let XmlChild::Block(paragraph) = known_last(root)? else {
        return None;
    };
    if paragraph.node.as_type() != NodeType::Paragraph {
        return None;
    }
    match known_last(paragraph)? {
        XmlChild::Inline(item) if leaf_type(&item.leaf) == NodeType::PageBreak => Some(item.pos),
        _ => None,
    }
}

fn validate_dots_unique(root: &XmlNode) -> Result<(), XmlError> {
    fn walk(node: &XmlNode, seen: &mut HashSet<Dot>) -> Result<(), XmlError> {
        if let Some(dot) = node.dot
            && !seen.insert(dot)
        {
            return Err(XmlError::at(
                node.pos,
                XmlErrorDetail::DotDuplicate {
                    dot: dot.to_string(),
                },
            )
            .with_dot(dot));
        }
        for child in node.block_children() {
            walk(child, seen)?;
        }
        Ok(())
    }
    walk(root, &mut HashSet::new())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::{Modifier, ModifierType, NodeType};

    use super::*;
    use crate::test_support::live_heads;
    use crate::writer::to_xml;
    use crate::{InlineLeaf, XmlChild};

    type DetailCheck = fn(&XmlErrorDetail) -> bool;

    fn root_xml(inner: &str) -> String {
        format!(
            "<root dot=\"1_0\" base=\"{}\" attr:layout_mode=\"continuous\" attr:max_width=\"600\">{inner}</root>",
            crate::writer::encode_base(&[]).unwrap()
        )
    }

    fn doc_xml(inner: &str) -> String {
        root_xml(&format!("{inner}<paragraph/>"))
    }

    #[test]
    fn round_trips_writer_output() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text(" a ") [bold, italic] text("b") hard_break } paragraph { text("c") [link(href: "https://x.y/?a=1&b=2".to_string())] } } }
            selection: (p1, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let tree = from_xml(&xml).unwrap();
        assert_eq!(tree.to_plain_doc(), state.to_plain());
        assert_eq!(tree.base, live_heads(&state));
        let first = tree.root.block_children().next().unwrap();
        assert_eq!(first.dot.map(|d| d.to_string()), Some(p1.to_string()));
        let items: Vec<&InlineEntry> = first.inline_items().collect();
        assert_eq!(
            items[0].own.keys().copied().collect::<Vec<_>>(),
            vec![ModifierType::Bold, ModifierType::Italic]
        );
        assert!(
            matches!(items[4].leaf, InlineLeaf::Atom(ref n) if n.as_type() == NodeType::HardBreak)
        );
    }

    #[test]
    fn accepts_any_nesting_order_and_whitespace_between_blocks() {
        let tree = from_xml(&root_xml(
            "\n  <paragraph>a</paragraph>\n  <paragraph><italic><bold>x</bold></italic>y</paragraph>\n",
        ))
        .unwrap();
        let blocks: Vec<&XmlNode> = tree.root.block_children().collect();
        assert_eq!(blocks.len(), 2);
        assert!(matches!(
            blocks[0].children.first(),
            Some(XmlChild::Inline(_))
        ));
        let x = blocks[1].inline_items().next().unwrap();
        assert_eq!(x.own.get(&ModifierType::Bold), Some(&Modifier::Bold));
        assert_eq!(x.own.len(), 2);
    }

    #[test]
    fn new_block_without_dot_and_defaults() {
        let tree = from_xml(&doc_xml(
            "<blockquote><paragraph>q</paragraph></blockquote>",
        ))
        .unwrap();
        let bq = tree.root.block_children().next().unwrap();
        assert!(bq.dot.is_none());
        assert_eq!(
            bq.node,
            PlainNode::Blockquote(editor_model::PlainBlockquoteNode {
                variant: Default::default()
            })
        );
    }

    #[test]
    fn unknown_inline_leaf_is_kept_but_stays_transparent() {
        let tree = from_xml(&doc_xml("<paragraph>a<unknown/>b</paragraph>")).unwrap();
        let para = tree.root.block_children().next().unwrap();
        let items: Vec<&InlineEntry> = para.inline_items().collect();
        assert_eq!(items.len(), 3);
        assert!(matches!(
            items[1].leaf,
            InlineLeaf::Atom(PlainNode::Unknown)
        ));
        let plain = tree.to_plain_doc();
        assert_eq!(plain.root.children[0].children.len(), 1);
        assert_eq!(
            plain.root.children[0].children[0].node,
            PlainNode::Text(editor_model::PlainTextNode { text: "ab".into() })
        );
    }

    #[test]
    fn a_root_ending_after_a_page_break_is_refused_at_the_break() {
        let err = from_xml(&root_xml(
            "<paragraph>a</paragraph>\n<paragraph>b<page_break/></paragraph>",
        ))
        .unwrap_err();
        assert_eq!(*err.detail, XmlErrorDetail::TrailingPageBreak);
        assert_eq!(
            err.pos,
            Some(Pos {
                line: 2,
                column: 13
            })
        );

        let behind_unknown = from_xml(&root_xml(
            "<paragraph>b<page_break/></paragraph><unknown dot=\"1_1\"/>",
        ))
        .unwrap_err();
        assert_eq!(*behind_unknown.detail, XmlErrorDetail::TrailingPageBreak);

        from_xml(&root_xml(
            "<paragraph>b<page_break/></paragraph><paragraph/>",
        ))
        .expect("a paragraph after the break makes the file writable");
    }

    #[test]
    fn a_table_row_short_of_the_widest_is_refused_at_the_row() {
        let cell = "<table_cell><paragraph/></table_cell>";
        let err = from_xml(&doc_xml(&format!(
            "<table><table_row>{cell}{cell}</table_row>\n<table_row>{cell}</table_row></table>"
        )))
        .unwrap_err();
        assert_eq!(
            *err.detail,
            XmlErrorDetail::TableNotRectangular {
                expected: 2,
                got: 1
            }
        );
        assert_eq!(err.pos, Some(Pos { line: 2, column: 1 }));

        from_xml(&doc_xml(&format!(
            "<table><table_row>{cell}{cell}</table_row><table_row>{cell}{cell}</table_row></table>"
        )))
        .expect("a rectangular table is a table the document can hold");
    }

    #[test]
    fn errors_report_a_position_and_a_matching_detail() {
        let cases: [(&str, DetailCheck); 16] = [
            ("<paragraph><b>x</b></paragraph>", |d| {
                matches!(d, XmlErrorDetail::UnknownElement { name, hint }
                        if name == "b" && hint.as_deref() == Some("bold"))
            }),
            ("<paragraph foo=\"1\">x</paragraph>", |d| {
                matches!(d, XmlErrorDetail::UnknownAttribute { element, attr }
                        if element == "paragraph" && attr == "foo")
            }),
            ("<paragraph attr:variant=\"x\">x</paragraph>", |d| {
                matches!(d, XmlErrorDetail::NodeAttrUnknown { element, field }
                        if element == "paragraph" && field == "variant")
            }),
            ("<blockquote><table dot=\"1_1\"/></blockquote>", |d| {
                matches!(d, XmlErrorDetail::ContentRule { parent, got, .. }
                        if parent == "blockquote" && got == &["table".to_owned()])
            }),
            (
                "<table><table_row><table_cell><table><table_row><table_cell><paragraph/></table_cell></table_row></table></table_cell></table_row></table>",
                |d| {
                    matches!(d, XmlErrorDetail::ContentRule { parent, got, .. }
                        if parent == "table_cell" && got == &["table".to_owned()])
                },
            ),
            (
                "<paragraph>a</paragraph>b",
                |d| matches!(d, XmlErrorDetail::TextInContainer { element } if element == "root"),
            ),
            ("<paragraph mod:font_size=\"99\">x</paragraph>", |d| {
                matches!(d, XmlErrorDetail::ValueOutOfRange { modifier, value }
                        if modifier == "font_size" && value == "99")
            }),
            (
                "<paragraph carry:line_height=\"160\">x</paragraph>",
                |d| matches!(d, XmlErrorDetail::ModifierNotCarryKind { name } if name == "line_height"),
            ),
            (
                "<bullet_list mod:alignment=\"center\"><list_item><paragraph/></list_item></bullet_list>",
                |d| {
                    matches!(d, XmlErrorDetail::BlockModifierNotAllowed { modifier, element }
                        if modifier == "alignment" && element == "bullet_list")
                },
            ),
            ("<paragraph>a\nb</paragraph>", |d| {
                matches!(d, XmlErrorDetail::NewlineInText)
            }),
            ("<paragraph>a\tb</paragraph>", |d| {
                matches!(d, XmlErrorDetail::TabInText)
            }),
            (
                "<unknown/>",
                |d| matches!(d, XmlErrorDetail::OpaqueNeedsDot { element } if element == "unknown"),
            ),
            (
                "<image attr:proportion=\"50\"/>",
                |d| matches!(d, XmlErrorDetail::OpaqueNeedsDot { element } if element == "image"),
            ),
            (
                "<paragraph dot=\"zz\">x</paragraph>",
                |d| matches!(d, XmlErrorDetail::DotInvalid { value } if value == "zz"),
            ),
            (
                "<paragraph dot=\"1_5\">x</paragraph><paragraph dot=\"1_5\">y</paragraph>",
                |d| matches!(d, XmlErrorDetail::DotDuplicate { dot } if dot == "1_5"),
            ),
            (
                "<blockquote><paragraph><page_break/></paragraph></blockquote>",
                |d| matches!(d, XmlErrorDetail::ContextNotAllowed { element } if element == "page_break"),
            ),
        ];
        for (inner, detail_matches) in cases {
            let err = from_xml(&doc_xml(inner)).unwrap_err();
            assert!(err.pos.is_some(), "{inner}");
            assert!(detail_matches(&err.detail), "{inner}: {:?}", err.detail);
        }

        let duplicate = from_xml(&doc_xml(
            "<paragraph dot=\"1_5\">x</paragraph><paragraph dot=\"1_5\">y</paragraph>",
        ))
        .unwrap_err();
        assert_eq!(duplicate.dot.as_deref(), Some("1_5"));

        let missing_base = from_xml("<root dot=\"1_0\"><paragraph/></root>").unwrap_err();
        assert_eq!(*missing_base.detail, XmlErrorDetail::BaseMissing);
        assert_eq!(missing_base.pos, None);

        let not_root = from_xml("<paragraph/>").unwrap_err();
        assert_eq!(
            *not_root.detail,
            XmlErrorDetail::RootNotRoot {
                name: "paragraph".into()
            }
        );
    }

    #[test]
    fn an_opaque_element_takes_no_modifier_the_schema_places_elsewhere() {
        for (inner, element) in [
            (
                "<archived dot=\"1_1\" mod:line_height=\"160\"/>",
                "archived",
            ),
            ("<unknown dot=\"1_2\" mod:alignment=\"center\"/>", "unknown"),
        ] {
            let xml = doc_xml(inner);
            let err = from_xml(&xml).unwrap_err();
            let modifier = if element == "archived" {
                "line_height"
            } else {
                "alignment"
            };
            assert_eq!(
                *err.detail,
                XmlErrorDetail::BlockModifierNotAllowed {
                    modifier: modifier.to_owned(),
                    element: element.to_owned(),
                }
            );
            let column = u32::try_from(xml.find(inner).expect("the element is in the file") + 1)
                .expect("the column fits");
            assert_eq!(err.pos, Some(Pos { line: 1, column }));
        }

        from_xml(&doc_xml("<archived dot=\"1_1\"/>"))
            .expect("an opaque element without modifiers is a file the document can hold");
    }

    #[test]
    fn errors_by_detail() {
        let base = crate::writer::encode_base(&[]).unwrap();
        let cases: [(String, DetailCheck); 15] = [
            (String::new(), |d| {
                matches!(d, XmlErrorDetail::RootMissing)
            }),
            (format!("{}<paragraph/>", doc_xml("")), |d| {
                matches!(d, XmlErrorDetail::TrailingContent)
            }),
            (doc_xml("<root dot=\"1_1\"/>"), |d| {
                matches!(d, XmlErrorDetail::MultipleRoots)
            }),
            (
                doc_xml(&format!("<paragraph base=\"{base}\"/>")),
                |d| matches!(d, XmlErrorDetail::BaseOnNonRoot),
            ),
            (
                "<root dot=\"1_0\" base=\"!!\" attr:layout_mode=\"continuous\" attr:max_width=\"600\"><paragraph/></root>".to_owned(),
                |d| matches!(d, XmlErrorDetail::BaseUndecodable),
            ),
            (doc_xml("<paragraph mod:nope=\"1\"/>"), |d| {
                matches!(d, XmlErrorDetail::UnknownModifier { prefix, name }
                    if prefix == "mod" && name == "nope")
            }),
            (doc_xml("<paragraph carry:nope=\"1\"/>"), |d| {
                matches!(d, XmlErrorDetail::UnknownModifier { prefix, name }
                    if prefix == "carry" && name == "nope")
            }),
            (
                doc_xml("<blockquote carry:bold=\"\"><paragraph/></blockquote>"),
                |d| {
                    matches!(d, XmlErrorDetail::CarryOnNonTextblock { element } if element == "blockquote")
                },
            ),
            (doc_xml("<paragraph><hard_break dot=\"1_1\"/></paragraph>"), |d| {
                matches!(d, XmlErrorDetail::AtomAttrNotAllowed { element, attr }
                    if element == "hard_break" && attr == "dot")
            }),
            (doc_xml("<paragraph><unknown attr:id=\"x\"/></paragraph>"), |d| {
                matches!(d, XmlErrorDetail::AtomAttrNotAllowed { element, attr }
                    if element == "unknown" && attr == "attr:id")
            }),
            (doc_xml("<paragraph><hard_break>x</hard_break></paragraph>"), |d| {
                matches!(d, XmlErrorDetail::AtomHasContent { element } if element == "hard_break")
            }),
            (doc_xml("<paragraph><blockquote/></paragraph>"), |d| {
                matches!(d, XmlErrorDetail::BlockInsideTextblock { parent, child }
                    if parent == "paragraph" && child == "blockquote")
            }),
            (doc_xml("<paragraph><bold value=\"1\">x</bold></paragraph>"), |d| {
                matches!(d, XmlErrorDetail::InlineModifierAttrNotAllowed { element, attr }
                    if element == "bold" && attr == "value")
            }),
            (doc_xml("<paragraph><link>x</link></paragraph>"), |d| {
                matches!(d, XmlErrorDetail::InlineModifierAttrMissing { element, attr }
                    if element == "link" && attr == "href")
            }),
            (
                doc_xml("<paragraph><link href=\"https://x.y\"><hard_break/></link></paragraph>"),
                |d| {
                    matches!(d, XmlErrorDetail::InlineModifierNotAllowed { modifier, leaf }
                        if modifier == "link" && leaf == "hard_break")
                },
            ),
        ];
        for (input, detail_matches) in cases {
            let err = from_xml(&input).unwrap_err();
            assert!(detail_matches(&err.detail), "{input}: {:?}", err.detail);
        }

        let opaque = from_xml(&doc_xml("<unknown dot=\"1_1\"><paragraph/></unknown>")).unwrap_err();
        assert_eq!(
            *opaque.detail,
            XmlErrorDetail::OpaqueHasChildren {
                element: "unknown".into()
            }
        );
    }
}
