use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{Modifier, ModifierType, PlainDoc, PlainNode, PlainNodeEntry, PlainTextNode};

use crate::error::Pos;

#[derive(Debug, Clone, PartialEq)]
pub struct XmlTree {
    pub base: Vec<Dot>,
    pub root: XmlNode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct XmlNode {
    pub dot: Option<Dot>,
    pub pos: Pos,
    pub node: PlainNode,
    pub modifiers: BTreeMap<ModifierType, Modifier>,
    pub carry: BTreeMap<ModifierType, Modifier>,
    pub children: Vec<XmlChild>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum XmlChild {
    Block(XmlNode),
    Inline(InlineEntry),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineEntry {
    pub pos: Pos,
    pub leaf: InlineLeaf,
    pub own: BTreeMap<ModifierType, Modifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineLeaf {
    Char(char),
    Atom(PlainNode),
}

impl XmlTree {
    pub fn to_plain_doc(&self) -> PlainDoc {
        PlainDoc {
            root: self.root.to_plain_entry(),
        }
    }
}

impl XmlNode {
    pub fn to_plain_entry(&self) -> PlainNodeEntry {
        let mut children: Vec<PlainNodeEntry> = Vec::new();
        let mut run: Option<(String, BTreeMap<ModifierType, Modifier>)> = None;
        for child in &self.children {
            match child {
                XmlChild::Block(block) => {
                    flush_run(&mut run, &mut children);
                    children.push(block.to_plain_entry());
                }
                XmlChild::Inline(item) => match &item.leaf {
                    InlineLeaf::Char(ch) => match &mut run {
                        Some((text, own)) if *own == item.own => text.push(*ch),
                        _ => {
                            flush_run(&mut run, &mut children);
                            run = Some((ch.to_string(), item.own.clone()));
                        }
                    },
                    InlineLeaf::Atom(PlainNode::Unknown) => {}
                    InlineLeaf::Atom(node) => {
                        flush_run(&mut run, &mut children);
                        children.push(PlainNodeEntry {
                            node: node.clone(),
                            modifiers: item.own.clone(),
                            carry: Vec::new(),
                            children: Vec::new(),
                        });
                    }
                },
            }
        }
        flush_run(&mut run, &mut children);
        PlainNodeEntry {
            node: self.node.clone(),
            modifiers: self.modifiers.clone(),
            carry: self.carry.values().cloned().collect(),
            children,
        }
    }

    pub fn block_children(&self) -> impl Iterator<Item = &XmlNode> {
        self.children.iter().filter_map(|c| match c {
            XmlChild::Block(b) => Some(b),
            XmlChild::Inline(_) => None,
        })
    }

    pub fn inline_items(&self) -> impl Iterator<Item = &InlineEntry> {
        self.children.iter().filter_map(|c| match c {
            XmlChild::Inline(i) => Some(i),
            XmlChild::Block(_) => None,
        })
    }
}

fn flush_run(
    run: &mut Option<(String, BTreeMap<ModifierType, Modifier>)>,
    out: &mut Vec<PlainNodeEntry>,
) {
    if let Some((text, modifiers)) = run.take() {
        out.push(PlainNodeEntry {
            node: PlainNode::Text(PlainTextNode { text }),
            modifiers,
            carry: Vec::new(),
            children: Vec::new(),
        });
    }
}
