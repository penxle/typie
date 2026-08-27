use serde::{Deserialize, Serialize};

use editor_model::NodeType;

use crate::address::{
    Address, NodePath, address_of, block_positions, display_path, node_at, node_at_mut, resolve,
    types_along,
};
use crate::error::{Pos, XmlError, XmlErrorDetail};
use crate::names::{is_textblock, modifier_from, modifier_type_of, node_attrs, node_from_attrs};
use crate::outline::{OutlineResult, outline_at};
use crate::reader::{from_xml, from_xml_fragment};
use crate::tree::{XmlChild, XmlNode, XmlTree};
use crate::write_tree::{BlockSpan, write_tree};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    Insert {
        xml: String,
        at: At,
    },
    Delete {
        targets: Vec<String>,
    },
    Move {
        targets: Vec<String>,
        at: At,
    },
    Replace {
        target: String,
        xml: String,
    },
    Set {
        targets: Vec<String>,
        attrs: Vec<SetAttr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum At {
    Before(String),
    After(String),
    FirstChild(String),
    LastChild(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetAttr {
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpError {
    pub op: Option<usize>,
    pub address: Option<String>,
    pub error: XmlError,
}

#[derive(Debug)]
pub struct Applied {
    pub tree: XmlTree,
    pub parents: Vec<NodePath>,
}

#[derive(Debug)]
pub struct Edited {
    pub xml: String,
    pub affected: Vec<OutlineResult>,
}

type NodeId = u32;

const ID_COLUMN: u32 = u32::MAX;

struct Work {
    root: XmlNode,
    next_id: NodeId,
    parents: Vec<NodeId>,
}

fn stamp(node: &mut XmlNode, next: &mut NodeId) {
    node.pos = Pos {
        line: *next,
        column: ID_COLUMN,
    };
    *next += 1;
    for child in &mut node.children {
        if let XmlChild::Block(b) = child {
            stamp(b, next);
        }
    }
}

fn clear_pos(node: &mut XmlNode) {
    node.pos = Pos::default();
    for child in &mut node.children {
        match child {
            XmlChild::Block(b) => clear_pos(b),
            XmlChild::Inline(i) => i.pos = Pos::default(),
        }
    }
}

fn id_of(node: &XmlNode) -> NodeId {
    node.pos.line
}

fn path_of_id(root: &XmlNode, id: NodeId) -> Option<NodePath> {
    fn walk(node: &XmlNode, id: NodeId, path: &mut NodePath) -> bool {
        if id_of(node) == id {
            return true;
        }
        for (i, child) in node.block_children().enumerate() {
            path.push(i);
            if walk(child, id, path) {
                return true;
            }
            path.pop();
        }
        false
    }
    let mut path = Vec::new();
    walk(root, id, &mut path).then_some(path)
}

fn fail(detail: XmlErrorDetail) -> XmlError {
    XmlError::new(detail)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    Before,
    After,
    First,
    Last,
}

impl Work {
    fn resolve(&self, value: &str) -> Result<NodePath, XmlError> {
        let address: Address = value.parse().map_err(fail)?;
        resolve(&self.root, &address).ok_or_else(|| {
            fail(XmlErrorDetail::AddressUnresolved {
                value: value.to_owned(),
            })
        })
    }

    fn resolve_editable(&self, value: &str) -> Result<NodePath, XmlError> {
        let path = self.resolve(value)?;
        if path.is_empty() {
            return Err(fail(XmlErrorDetail::RootNotEditable));
        }
        Ok(path)
    }

    fn resolve_targets(&self, targets: &[String]) -> Result<Vec<NodePath>, XmlError> {
        let mut paths: Vec<NodePath> = Vec::new();
        for t in targets {
            let p = self.resolve_editable(t)?;
            if !paths.contains(&p) {
                paths.push(p);
            }
        }
        paths.sort();
        for pair in paths.windows(2) {
            if pair[1].starts_with(&pair[0]) {
                return Err(fail(XmlErrorDetail::TargetsNested {
                    outer: self.address_at(&pair[0]),
                    inner: self.address_at(&pair[1]),
                }));
            }
        }
        Ok(paths)
    }

    fn address_at(&self, path: &[usize]) -> String {
        node_at(&self.root, path)
            .map(|n| address_of(n, path))
            .unwrap_or_else(|| display_path(path))
    }

    fn note_parent(&mut self, path: &[usize]) {
        let parent = &path[..path.len().saturating_sub(1)];
        let id = id_of(node_at(&self.root, parent).expect("resolved path"));
        if !self.parents.contains(&id) {
            self.parents.push(id);
        }
    }

    fn parse_fragment(&mut self, xml: &str) -> Result<Vec<XmlNode>, XmlError> {
        let mut nodes = from_xml_fragment(xml).map_err(|e| XmlError::new(*e.detail))?;
        for n in &mut nodes {
            stamp(n, &mut self.next_id);
        }
        Ok(nodes)
    }

    fn destination(&self, at: &At) -> Result<(NodeId, Slot), XmlError> {
        let (value, slot) = match at {
            At::Before(v) => (v, Slot::Before),
            At::After(v) => (v, Slot::After),
            At::FirstChild(v) => (v, Slot::First),
            At::LastChild(v) => (v, Slot::Last),
        };
        let path = self.resolve(value)?;
        let node = node_at(&self.root, &path).expect("resolved path");
        match slot {
            Slot::Before | Slot::After if path.is_empty() => {
                return Err(fail(XmlErrorDetail::RootHasNoSiblings));
            }
            Slot::First | Slot::Last => {
                let t = node.node.as_type();
                if is_textblock(t) || crate::names::is_opaque(t) || crate::names::is_block_atom(t) {
                    return Err(fail(XmlErrorDetail::TargetNotContainer {
                        element: crate::names::element_name(t).unwrap_or("text").to_owned(),
                    }));
                }
            }
            _ => {}
        }
        Ok((id_of(node), slot))
    }

    fn place(&mut self, destination: (NodeId, Slot), nodes: Vec<XmlNode>) -> Result<(), XmlError> {
        let (id, slot) = destination;
        let anchor =
            path_of_id(&self.root, id).ok_or_else(|| XmlError::internal("destination vanished"))?;
        let (parent_path, index) = match slot {
            Slot::Before | Slot::After => {
                let (last, parent) = anchor.split_last().expect("non-root anchor");
                (
                    parent.to_vec(),
                    if slot == Slot::Before {
                        *last
                    } else {
                        *last + 1
                    },
                )
            }
            Slot::First => (anchor.clone(), 0),
            Slot::Last => {
                let count = node_at(&self.root, &anchor)
                    .expect("anchor")
                    .block_children()
                    .count();
                (anchor.clone(), count)
            }
        };
        let parent = node_at_mut(&mut self.root, &parent_path).expect("parent");
        let positions = block_positions(parent);
        let slot_index = positions
            .get(index)
            .copied()
            .unwrap_or(parent.children.len());
        let inserted: Vec<XmlChild> = nodes.into_iter().map(XmlChild::Block).collect();
        parent.children.splice(slot_index..slot_index, inserted);
        let parent_id = id_of(node_at(&self.root, &parent_path).expect("parent"));
        if !self.parents.contains(&parent_id) {
            self.parents.push(parent_id);
        }
        Ok(())
    }

    fn detach(&mut self, paths: &[NodePath]) -> Vec<XmlNode> {
        let mut taken = Vec::new();
        for path in paths.iter().rev() {
            self.note_parent(path);
            let (last, parent_path) = path.split_last().expect("non-root");
            let parent = node_at_mut(&mut self.root, parent_path).expect("parent");
            let slot = block_positions(parent)[*last];
            match parent.children.remove(slot) {
                XmlChild::Block(b) => taken.push(b),
                XmlChild::Inline(_) => unreachable!("block slot"),
            }
        }
        taken.reverse();
        taken
    }

    fn apply(&mut self, op: &Op) -> Result<(), XmlError> {
        match op {
            Op::Insert { xml, at } => {
                let destination = self.destination(at)?;
                let nodes = self.parse_fragment(xml)?;
                self.place(destination, nodes)
            }
            Op::Delete { targets } => {
                let paths = self.resolve_targets(targets)?;
                self.detach(&paths);
                Ok(())
            }
            Op::Move { targets, at } => {
                let paths = self.resolve_targets(targets)?;
                let (id, slot) = self.destination(at)?;
                let anchor = path_of_id(&self.root, id).expect("anchor");
                for p in &paths {
                    if anchor.starts_with(p) {
                        return Err(fail(XmlErrorDetail::MoveIntoSelf {
                            target: self.address_at(p),
                        }));
                    }
                }
                let nodes = self.detach(&paths);
                self.place((id, slot), nodes)
            }
            Op::Replace { target, xml } => {
                let path = self.resolve_editable(target)?;
                let mut nodes = self.parse_fragment(xml)?;
                if nodes.len() != 1 {
                    return Err(fail(XmlErrorDetail::FragmentNotSingle {
                        count: nodes.len(),
                    }));
                }
                let mut node = nodes.pop().expect("one");
                self.note_parent(&path);
                let (last, parent_path) = path.split_last().expect("non-root");
                let parent = node_at_mut(&mut self.root, parent_path).expect("parent");
                let slot = block_positions(parent)[*last];
                if let XmlChild::Block(old) = &parent.children[slot]
                    && node.dot.is_none()
                {
                    node.dot = old.dot;
                }
                parent.children[slot] = XmlChild::Block(node);
                Ok(())
            }
            Op::Set { targets, attrs } => {
                let paths = self.resolve_targets(targets)?;
                for path in &paths {
                    self.note_parent(path);
                    let types = types_along(&self.root, path);
                    let node = node_at_mut(&mut self.root, path).expect("target");
                    set_attrs(node, &types, attrs)?;
                }
                Ok(())
            }
        }
    }
}

fn set_attrs(node: &mut XmlNode, types: &[NodeType], attrs: &[SetAttr]) -> Result<(), XmlError> {
    let node_type = node.node.as_type();
    let element = crate::names::element_name(node_type)
        .unwrap_or("text")
        .to_owned();
    for SetAttr { key, value } in attrs {
        if let Some(field) = key.strip_prefix("attr:") {
            let mut current = node_attrs(&node.node);
            match value {
                Some(v) => {
                    crate::writer::escape_attr(v).map_err(|e| fail(*e.detail))?;
                    current.insert(field.to_owned(), v.clone());
                }
                None => {
                    current.remove(field);
                }
            }
            node.node = node_from_attrs(node_type, &current).map_err(fail)?;
        } else if let Some(name) = key.strip_prefix("mod:") {
            let ty = modifier_type_of(name).ok_or_else(|| {
                fail(XmlErrorDetail::UnknownModifier {
                    prefix: "mod".to_owned(),
                    name: name.to_owned(),
                })
            })?;
            match value {
                Some(v) => {
                    crate::writer::escape_attr(v).map_err(|e| fail(*e.detail))?;
                    let modifier = modifier_from(ty, v).map_err(fail)?;
                    if !crate::names::modifier_fits_context(ty, types) {
                        return Err(fail(XmlErrorDetail::BlockModifierNotAllowed {
                            modifier: crate::names::modifier_type_name(ty),
                            element: element.clone(),
                        }));
                    }
                    node.modifiers.insert(ty, modifier);
                }
                None => {
                    node.modifiers.remove(&ty);
                }
            }
        } else if let Some(name) = key.strip_prefix("carry:") {
            let ty = modifier_type_of(name).ok_or_else(|| {
                fail(XmlErrorDetail::UnknownModifier {
                    prefix: "carry".to_owned(),
                    name: name.to_owned(),
                })
            })?;
            match value {
                Some(v) => {
                    crate::writer::escape_attr(v).map_err(|e| fail(*e.detail))?;
                    if !ty.is_carry_kind() {
                        return Err(fail(XmlErrorDetail::ModifierNotCarryKind {
                            name: name.to_owned(),
                        }));
                    }
                    if !is_textblock(node_type) {
                        return Err(fail(XmlErrorDetail::CarryOnNonTextblock {
                            element: element.clone(),
                        }));
                    }
                    node.carry.insert(ty, modifier_from(ty, v).map_err(fail)?);
                }
                None => {
                    node.carry.remove(&ty);
                }
            }
        } else {
            return Err(fail(XmlErrorDetail::SetKeyUnknown { key: key.clone() }));
        }
    }
    Ok(())
}

pub fn apply_ops(tree: &XmlTree, ops: &[Op]) -> Result<Applied, OpError> {
    let mut work = Work {
        root: tree.root.clone(),
        next_id: 0,
        parents: Vec::new(),
    };
    stamp(&mut work.root, &mut work.next_id);
    for (index, op) in ops.iter().enumerate() {
        work.apply(op).map_err(|error| OpError {
            op: Some(index),
            address: None,
            error,
        })?;
    }
    let mut parents: Vec<NodePath> = work
        .parents
        .iter()
        .filter_map(|id| path_of_id(&work.root, *id))
        .collect();
    parents.sort();
    parents.dedup();
    let mut root = work.root;
    clear_pos(&mut root);
    Ok(Applied {
        tree: XmlTree {
            base: tree.base.clone(),
            root,
        },
        parents,
    })
}

fn address_for_line(tree: &XmlTree, spans: &[BlockSpan], line: u32) -> Option<String> {
    spans
        .iter()
        .filter(|s| s.start_line <= line && line <= s.end_line)
        .max_by_key(|s| s.path.len())
        .and_then(|s| node_at(&tree.root, &s.path).map(|n| address_of(n, &s.path)))
}

pub fn edit_file(xml: &str, ops: &[Op]) -> Result<Edited, OpError> {
    let whole = |error: XmlError| OpError {
        op: None,
        address: None,
        error,
    };
    let tree = from_xml(xml).map_err(whole)?;
    let applied = apply_ops(&tree, ops)?;
    let (text, spans) = write_tree(&applied.tree).map_err(whole)?;
    let verified = match from_xml(&text) {
        Ok(t) => t,
        Err(error) => {
            let address = error
                .pos
                .and_then(|p| address_for_line(&applied.tree, &spans, p.line))
                .or_else(|| error.dot.clone());
            return Err(OpError {
                op: None,
                address,
                error,
            });
        }
    };
    let affected = applied
        .parents
        .iter()
        .map(|p| outline_at(&verified, p, 1))
        .collect::<Result<Vec<_>, _>>()
        .map_err(whole)?;
    Ok(Edited {
        xml: text,
        affected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::from_xml;

    use crate::address::{display_path, node_at};
    use crate::test_support::{arb_plain_doc, live_heads};
    use crate::writer::to_xml;
    use proptest::prelude::*;

    fn base() -> String {
        crate::writer::encode_base(&[]).unwrap()
    }

    fn doc(inner: &str) -> String {
        format!(
            "<root dot=\"1_0\" base=\"{}\" attr:layout_mode=\"continuous\" attr:max_width=\"600\">{inner}</root>",
            base()
        )
    }

    fn ops(json: &str) -> Vec<Op> {
        serde_json::from_str(json).unwrap()
    }

    fn paths(edited: &Edited) -> Vec<(String, String)> {
        let tree = from_xml(&edited.xml).unwrap();
        let out = crate::outline::outline_at(&tree, &[], 8).unwrap();
        out.rows
            .into_iter()
            .map(|r| (r.path, r.preview.unwrap_or_else(|| r.name.clone())))
            .collect()
    }

    const THREE: &str = "<paragraph dot=\"1_1\">a</paragraph><paragraph dot=\"1_2\">b</paragraph><paragraph dot=\"1_3\">c</paragraph>";

    #[test]
    fn ops_deserialize_from_the_tool_json() {
        let parsed = ops(
            r#"[{"op":"insert","xml":"<paragraph/>","at":{"after":"1"}},{"op":"delete","targets":["1_1"]},{"op":"move","targets":["2"],"at":{"first_child":"root"}},{"op":"replace","target":"3","xml":"<paragraph>z</paragraph>"},{"op":"set","targets":["1"],"attrs":[{"key":"mod:alignment","value":"center"},{"key":"carry:font_size","value":null}]}]"#,
        );
        assert_eq!(parsed.len(), 5);
        assert!(matches!(&parsed[2], Op::Move { at: At::FirstChild(a), .. } if a == "root"));
        assert!(matches!(&parsed[4], Op::Set { attrs, .. } if attrs[1].value.is_none()));
    }

    #[test]
    fn insert_before_after_first_and_last_child() {
        let out = edit_file(
            &doc(THREE),
            &ops(r#"[
            {"op":"insert","xml":"<paragraph>x</paragraph><paragraph>y</paragraph>","at":{"after":"1_1"}},
            {"op":"insert","xml":"<paragraph>w</paragraph>","at":{"before":"1"}},
            {"op":"insert","xml":"<blockquote/>","at":{"after":"5"}},
            {"op":"insert","xml":"<paragraph>q</paragraph>","at":{"first_child":"6"}}
        ]"#),
        )
        .unwrap();
        assert_eq!(
            paths(&out)
                .iter()
                .map(|(p, t)| format!("{p}={t}"))
                .collect::<Vec<_>>(),
            [
                "1=w",
                "2=a",
                "3=x",
                "4=y",
                "5=b",
                "6=blockquote",
                "6.1=q",
                "7=c"
            ]
        );
        assert_eq!(out.affected.len(), 2);
        assert!(out.affected[0].head.is_none());
        assert_eq!(out.affected[1].head.as_ref().unwrap().path, "6");

        let tree = from_xml(&doc(THREE)).unwrap();
        let applied = apply_ops(
            &tree,
            &ops(r#"[
            {"op":"insert","xml":"<paragraph>x</paragraph>","at":{"after":"1_1"}},
            {"op":"insert","xml":"<blockquote/>","at":{"after":"3"}}
        ]"#),
        )
        .unwrap();
        assert!(pos_is_cleared(&applied.tree.root));
    }

    fn pos_is_cleared(node: &XmlNode) -> bool {
        node.pos == Pos::default()
            && node.children.iter().all(|c| match c {
                XmlChild::Block(b) => pos_is_cleared(b),
                XmlChild::Inline(i) => i.pos == Pos::default(),
            })
    }

    #[test]
    fn delete_and_move_keep_dots_and_document_order() {
        let out = edit_file(
            &doc(THREE),
            &ops(r#"[
            {"op":"move","targets":["1_3","1_1"],"at":{"before":"1_2"}},
            {"op":"delete","targets":["1_2"]}
        ]"#),
        )
        .unwrap();
        let tree = from_xml(&out.xml).unwrap();
        let dots: Vec<String> = tree
            .root
            .block_children()
            .map(|n| n.dot.unwrap().to_string())
            .collect();
        assert_eq!(dots, ["1_1", "1_3"]);
        assert_eq!(
            paths(&out)
                .iter()
                .map(|(_, t)| t.as_str())
                .collect::<Vec<_>>(),
            ["a", "c"]
        );
    }

    #[test]
    fn move_into_a_container_resolves_the_destination_after_detaching() {
        let out = edit_file(
            &doc("<paragraph dot=\"1_1\">a</paragraph><blockquote dot=\"1_2\"><paragraph dot=\"1_3\">b</paragraph></blockquote><paragraph dot=\"1_4\">z</paragraph>"),
            &ops(r#"[{"op":"move","targets":["1"],"at":{"last_child":"2"}}]"#),
        )
        .unwrap();
        assert_eq!(
            paths(&out)
                .iter()
                .map(|(p, _)| p.as_str())
                .collect::<Vec<_>>(),
            ["1", "1.1", "1.2", "2"]
        );
        assert_eq!(paths(&out)[2].1, "a");
    }

    #[test]
    fn replace_inherits_the_dot_unless_the_fragment_carries_one() {
        let out = edit_file(
            &doc(THREE),
            &ops(r#"[
            {"op":"replace","target":"1_2","xml":"<paragraph>B</paragraph>"},
            {"op":"replace","target":"3","xml":"<paragraph dot=\"1_9\">C</paragraph>"}
        ]"#),
        )
        .unwrap();
        let tree = from_xml(&out.xml).unwrap();
        let dots: Vec<String> = tree
            .root
            .block_children()
            .map(|n| n.dot.unwrap().to_string())
            .collect();
        assert_eq!(dots, ["1_1", "1_2", "1_9"]);
    }

    #[test]
    fn set_adds_changes_and_removes_attrs_mods_and_carries() {
        let out = edit_file(
            &doc("<paragraph dot=\"1_1\" mod:alignment=\"right\" carry:font_size=\"1400\">a</paragraph><table dot=\"1_2\"><table_row><table_cell><paragraph/></table_cell></table_row></table><paragraph/>"),
            &ops(r#"[
                {"op":"set","targets":["1_1"],"attrs":[{"key":"mod:alignment","value":"center"},{"key":"carry:font_size","value":null},{"key":"mod:line_height","value":"200"}]},
                {"op":"set","targets":["1_2"],"attrs":[{"key":"attr:border_style","value":"dashed"},{"key":"attr:proportion","value":"80"}]}
            ]"#),
        )
        .unwrap();
        assert!(out.xml.contains(
            "<paragraph dot=\"1_1\" mod:line_height=\"200\" mod:alignment=\"center\">a</paragraph>"
        ));
        assert!(
            out.xml.contains(
                "<table dot=\"1_2\" attr:border_style=\"dashed\" attr:proportion=\"80\">"
            )
        );
    }

    #[test]
    fn move_to_the_last_child_of_the_same_parent_rotates_the_children() {
        let out = edit_file(
            &doc(THREE),
            &ops(r#"[{"op":"move","targets":["1"],"at":{"last_child":"root"}}]"#),
        )
        .unwrap();
        let tree = from_xml(&out.xml).unwrap();
        let dots: Vec<String> = tree
            .root
            .block_children()
            .map(|n| n.dot.unwrap().to_string())
            .collect();
        assert_eq!(dots, ["1_2", "1_3", "1_1"]);
        assert_eq!(
            paths(&out)
                .iter()
                .map(|(_, t)| t.as_str())
                .collect::<Vec<_>>(),
            ["b", "c", "a"]
        );
    }

    #[test]
    fn set_reaches_every_target_and_leaves_the_others_alone() {
        let out = edit_file(
            &doc(THREE),
            &ops(r#"[{"op":"set","targets":["1_1","1_3"],"attrs":[{"key":"mod:alignment","value":"center"}]}]"#),
        )
        .unwrap();
        assert!(
            out.xml
                .contains("<paragraph dot=\"1_1\" mod:alignment=\"center\">a</paragraph>")
        );
        assert!(
            out.xml
                .contains("<paragraph dot=\"1_3\" mod:alignment=\"center\">c</paragraph>")
        );
        assert!(out.xml.contains("<paragraph dot=\"1_2\">b</paragraph>"));
    }

    #[test]
    fn a_control_char_in_a_set_value_names_the_op() {
        let err = edit_file(
            &doc("<archived dot=\"1_5\" attr:id=\"x\"/><paragraph/>"),
            &ops(
                r#"[{"op":"set","targets":["1_5"],"attrs":[{"key":"attr:id","value":"\u0001x"}]}]"#,
            ),
        )
        .unwrap_err();
        assert_eq!(err.op, Some(0));
        assert!(matches!(
            *err.error.detail,
            XmlErrorDetail::ForbiddenCharInDocument { codepoint: 1 }
        ));
    }

    const NESTED: &str = "<blockquote dot=\"1_4\"><paragraph dot=\"1_5\">d</paragraph></blockquote><paragraph dot=\"1_1\">a</paragraph><paragraph dot=\"1_2\">b</paragraph><paragraph dot=\"1_3\">c</paragraph>";

    type DetailCheck = fn(&XmlErrorDetail) -> bool;

    #[test]
    fn every_op_level_failure_names_its_op_and_leaves_the_file_alone() {
        let cases: Vec<(&str, DetailCheck)> = vec![
            (
                r#"[{"op":"delete","targets":["nope"]}]"#,
                |d| matches!(d, XmlErrorDetail::AddressInvalid { value } if value == "nope"),
            ),
            (
                r#"[{"op":"delete","targets":["9_9"]}]"#,
                |d| matches!(d, XmlErrorDetail::AddressUnresolved { value } if value == "9_9"),
            ),
            (r#"[{"op":"delete","targets":["root"]}]"#, |d| {
                matches!(d, XmlErrorDetail::RootNotEditable)
            }),
            (r#"[{"op":"set","targets":["1_0"],"attrs":[]}]"#, |d| {
                matches!(d, XmlErrorDetail::RootNotEditable)
            }),
            (
                r#"[{"op":"insert","xml":"<paragraph/>","at":{"before":"root"}}]"#,
                |d| matches!(d, XmlErrorDetail::RootHasNoSiblings),
            ),
            (
                r#"[{"op":"insert","xml":"<paragraph/>","at":{"first_child":"1_1"}}]"#,
                |d| matches!(d, XmlErrorDetail::TargetNotContainer { element } if element == "paragraph"),
            ),
            (
                r#"[{"op":"move","targets":["1_2"],"at":{"after":"1_2"}}]"#,
                |d| matches!(d, XmlErrorDetail::MoveIntoSelf { .. }),
            ),
            (
                r#"[{"op":"delete","targets":["1_5","1_4"]}]"#,
                |d| matches!(d, XmlErrorDetail::TargetsNested { outer, inner } if outer == "1_4" && inner == "1_5"),
            ),
            (r#"[{"op":"insert","xml":"  ","at":{"after":"1"}}]"#, |d| {
                matches!(d, XmlErrorDetail::FragmentEmpty)
            }),
            (
                r#"[{"op":"insert","xml":"text","at":{"after":"1"}}]"#,
                |d| matches!(d, XmlErrorDetail::FragmentNotBlock),
            ),
            (
                r#"[{"op":"replace","target":"1","xml":"<paragraph/><paragraph/>"}]"#,
                |d| matches!(d, XmlErrorDetail::FragmentNotSingle { count: 2 }),
            ),
            (
                r#"[{"op":"set","targets":["1"],"attrs":[{"key":"dot","value":"1_5"}]}]"#,
                |d| matches!(d, XmlErrorDetail::SetKeyUnknown { key } if key == "dot"),
            ),
            (
                r#"[{"op":"set","targets":["1_1"],"attrs":[{"key":"attr:variant","value":"x"}]}]"#,
                |d| matches!(d, XmlErrorDetail::NodeAttrUnknown { element, field } if element == "paragraph" && field == "variant"),
            ),
            (
                r#"[{"op":"set","targets":["1"],"attrs":[{"key":"mod:nope","value":"x"}]}]"#,
                |d| matches!(d, XmlErrorDetail::UnknownModifier { .. }),
            ),
            (
                r#"[{"op":"set","targets":["1"],"attrs":[{"key":"mod:font_weight","value":"150"}]}]"#,
                |d| matches!(d, XmlErrorDetail::ValueOutOfRange { .. }),
            ),
            (
                r#"[{"op":"insert","xml":"<paragraph><b>x</b></paragraph>","at":{"after":"1"}}]"#,
                |d| matches!(d, XmlErrorDetail::UnknownElement { .. }),
            ),
        ];
        for (json, detail_is) in cases {
            let err = edit_file(&doc(NESTED), &ops(json)).unwrap_err();
            assert_eq!(err.op, Some(0), "{json}");
            assert!(
                detail_is(&err.error.detail),
                "{json}: {:?}",
                err.error.detail
            );
        }
        let err = edit_file(
            &doc(THREE),
            &ops(r#"[{"op":"delete","targets":["1"]},{"op":"delete","targets":["9_9"]}]"#),
        )
        .unwrap_err();
        assert_eq!(err.op, Some(1));
    }

    #[test]
    fn a_whole_file_failure_is_reported_at_the_block_address_not_the_op() {
        let err = edit_file(
            &doc("<table dot=\"1_1\"><table_row dot=\"1_2\"><table_cell dot=\"1_3\"><paragraph/></table_cell></table_row></table><paragraph/>"),
            &ops(r#"[
                {"op":"insert","xml":"<table_row><table_cell><paragraph/></table_cell><table_cell><paragraph/></table_cell></table_row>","at":{"after":"1_2"}}
            ]"#),
        )
        .unwrap_err();
        assert_eq!(err.op, None);
        assert_eq!(err.address.as_deref(), Some("1_2"));
        assert!(matches!(
            *err.error.detail,
            XmlErrorDetail::TableNotRectangular { .. }
        ));

        let err = edit_file(
            &doc(THREE),
            &ops(
                r#"[{"op":"insert","xml":"<paragraph dot=\"1_1\">dup</paragraph>","at":{"after":"3"}}]"#,
            ),
        )
        .unwrap_err();
        assert_eq!(err.op, None);
        assert!(matches!(
            *err.error.detail,
            XmlErrorDetail::DotDuplicate { .. }
        ));
        assert_eq!(err.address.as_deref(), Some("1_1"));
    }

    #[test]
    fn a_table_can_be_built_across_ops_within_one_batch() {
        let out = edit_file(
            &doc("<paragraph dot=\"1_1\">a</paragraph><paragraph dot=\"1_2\">z</paragraph>"),
            &ops(r#"[
            {"op":"insert","xml":"<table><table_row><table_cell><paragraph/></table_cell><table_cell><paragraph/></table_cell></table_row></table>","at":{"after":"1"}},
            {"op":"insert","xml":"<table_row><table_cell><paragraph/></table_cell></table_row>","at":{"last_child":"2"}},
            {"op":"insert","xml":"<table_cell><paragraph>fill</paragraph></table_cell>","at":{"last_child":"2.2"}}
        ]"#),
        )
        .unwrap();
        let tree = from_xml(&out.xml).unwrap();
        assert_eq!(
            tree.root
                .block_children()
                .nth(1)
                .unwrap()
                .block_children()
                .count(),
            2
        );
    }

    fn dots_of(node: &XmlNode, out: &mut Vec<editor_crdt::Dot>) {
        if let Some(d) = node.dot {
            out.push(d);
        }
        for c in node.block_children() {
            dots_of(c, out);
        }
    }

    fn root_paragraph_paths(tree: &XmlTree) -> Vec<NodePath> {
        tree.root
            .block_children()
            .enumerate()
            .filter(|(_, n)| n.node.as_type() == editor_model::NodeType::Paragraph)
            .map(|(i, _)| vec![i])
            .collect()
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 128, max_global_rejects: 2048, ..ProptestConfig::default() })]

        #[test]
        fn moving_root_paragraphs_preserves_dots_and_content(doc in arb_plain_doc(), seed in any::<u64>()) {
            let state = editor_state::State::from_plain(&doc).unwrap();
            let xml = to_xml(&state, &live_heads(&state)).unwrap();
            let tree = from_xml(&xml).unwrap();
            let paragraphs = root_paragraph_paths(&tree);
            prop_assume!(paragraphs.len() >= 2);
            let from = paragraphs[(seed % paragraphs.len() as u64) as usize].clone();
            let to = paragraphs[((seed / 7) % paragraphs.len() as u64) as usize].clone();
            prop_assume!(from != to);
            prop_assume!(from[0] + 1 != tree.root.block_children().count());
            let ops = vec![Op::Move { targets: vec![display_path(&from)], at: At::Before(display_path(&to)) }];
            let out = edit_file(&xml, &ops).unwrap();
            let after = from_xml(&out.xml).unwrap();
            let (mut before_dots, mut after_dots) = (Vec::new(), Vec::new());
            dots_of(&tree.root, &mut before_dots);
            dots_of(&after.root, &mut after_dots);
            before_dots.sort();
            after_dots.sort();
            prop_assert_eq!(before_dots, after_dots);
            let moved = node_at(&tree.root, &from).unwrap();
            let landed = after.root.block_children().find(|n| n.dot == moved.dot).unwrap();
            prop_assert_eq!(landed.to_plain_entry(), moved.to_plain_entry());
        }

        #[test]
        fn a_failing_op_anywhere_leaves_the_file_untouched(doc in arb_plain_doc(), at in 0usize..3) {
            let state = editor_state::State::from_plain(&doc).unwrap();
            let xml = to_xml(&state, &live_heads(&state)).unwrap();
            let mut ops = vec![
                Op::Insert { xml: "<paragraph>p</paragraph>".into(), at: At::LastChild("root".into()) },
                Op::Insert { xml: "<paragraph>r</paragraph>".into(), at: At::After("1".into()) },
                Op::Insert { xml: "<paragraph>q</paragraph>".into(), at: At::FirstChild("root".into()) },
            ];
            ops.insert(at, Op::Delete { targets: vec!["9_9".into()] });
            let err = edit_file(&xml, &ops).unwrap_err();
            prop_assert_eq!(err.op, Some(at));
            let tree = from_xml(&xml).unwrap();
            prop_assert_eq!(apply_ops(&tree, &ops).unwrap_err().op, Some(at));
        }

        #[test]
        fn delete_plus_insert_equals_replace_up_to_the_dot(doc in arb_plain_doc()) {
            let state = editor_state::State::from_plain(&doc).unwrap();
            let xml = to_xml(&state, &live_heads(&state)).unwrap();
            let tree = from_xml(&xml).unwrap();
            prop_assume!(!root_paragraph_paths(&tree).is_empty());
            let target = display_path(&root_paragraph_paths(&tree)[0]);
            let fragment = "<paragraph>replaced</paragraph>";
            let a = edit_file(&xml, &[Op::Replace { target: target.clone(), xml: fragment.into() }]).unwrap();
            let b = edit_file(&xml, &[
                Op::Insert { xml: fragment.into(), at: At::After(target.clone()) },
                Op::Delete { targets: vec![target.clone()] },
            ]).unwrap();
            prop_assert_eq!(from_xml(&a.xml).unwrap().to_plain_doc(), from_xml(&b.xml).unwrap().to_plain_doc());
        }
    }
}
