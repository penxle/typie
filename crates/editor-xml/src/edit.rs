use std::collections::{BTreeMap, HashSet};

use editor_crdt::Dot;
use editor_model::{
    DocView, Modifier, ModifierType, NodeType, PlainDoc, PlainNode, PlainNodeEntry,
};
use editor_state::State;
use editor_transaction::{HistoryMeta, Transaction};

use crate::diff::{ChangeCounts, Diff};
use crate::error::{XmlError, XmlErrorDetail};
use crate::lcs::MAX_EDIT_DISTANCE;
use crate::names::{
    element_name, is_block_atom, is_opaque, modifier_type_name, writable_modifiers,
};
use crate::tree::{InlineLeaf, XmlChild, XmlNode, XmlTree};

pub struct EditOutcome {
    pub state: State,
    pub changed: ChangeCounts,
}

pub fn validate_against(base: &State, target: &XmlTree) -> Result<(), XmlError> {
    let view = base.view();
    let root = view.root().ok_or_else(|| XmlError::internal("no root"))?;
    match target.root.dot {
        Some(d) if d == root.id() => {}
        _ => {
            return Err(XmlError::at(
                target.root.pos,
                XmlErrorDetail::RootDotMismatch,
            ));
        }
    }
    let mut seen = HashSet::new();
    walk(&view, &target.root, &mut seen)
}

fn walk(view: &DocView<'_>, node: &XmlNode, seen: &mut HashSet<Dot>) -> Result<(), XmlError> {
    if let Some(dot) = node.dot {
        let Some((base_type, base_node)) = live_node(view, dot) else {
            return Err(XmlError::at(
                node.pos,
                XmlErrorDetail::DotNotInDocument {
                    dot: dot.to_string(),
                },
            )
            .with_dot(dot));
        };
        if !seen.insert(dot) {
            return Err(XmlError::at(
                node.pos,
                XmlErrorDetail::DotDuplicate {
                    dot: dot.to_string(),
                },
            )
            .with_dot(dot));
        }
        let target_type = node.node.as_type();
        if base_type != target_type && !is_convertible(base_type, target_type, node) {
            return Err(XmlError::at(
                node.pos,
                XmlErrorDetail::DotTypeIncompatible {
                    dot: dot.to_string(),
                    new_type: type_name(target_type),
                },
            )
            .with_dot(dot));
        }
        if let Some(element) = changed_opaque_id(&base_node, &node.node) {
            return Err(XmlError::at(
                node.pos,
                XmlErrorDetail::OpaqueIdChanged {
                    element: element.to_string(),
                    dot: dot.to_string(),
                },
            )
            .with_dot(dot));
        }
    }
    for child in node.block_children() {
        walk(view, child, seen)?;
    }
    Ok(())
}

/// The type and node a dot stands for: a block node, or an atom leaf holding a
/// block slot.
fn live_node(view: &DocView<'_>, dot: Dot) -> Option<(NodeType, PlainNode)> {
    if let Some(nv) = view.node(dot) {
        return Some((nv.node_type(), nv.node().to_plain()));
    }
    let leaf = view.leaf(dot)?;
    let node = leaf.node()?;
    Some((leaf.node_type(), node.to_plain()))
}

fn is_convertible(base_type: NodeType, target_type: NodeType, node: &XmlNode) -> bool {
    if is_block_atom(base_type) || is_block_atom(target_type) {
        return false;
    }
    if !is_replaceable(base_type) || !is_replaceable(target_type) {
        return false;
    }
    let child_types: Vec<NodeType> = node
        .children
        .iter()
        .map(child_type)
        .filter(|t| *t != NodeType::Unknown)
        .collect();
    target_type.spec().content.matches_sequence(&child_types)
}

fn is_replaceable(t: NodeType) -> bool {
    !matches!(t, NodeType::Root | NodeType::Unknown)
}

fn child_type(child: &XmlChild) -> NodeType {
    match child {
        XmlChild::Block(block) => block.node.as_type(),
        XmlChild::Inline(item) => match &item.leaf {
            InlineLeaf::Char(_) => NodeType::Text,
            InlineLeaf::Atom(node) => node.as_type(),
        },
    }
}

fn changed_opaque_id(base: &PlainNode, target: &PlainNode) -> Option<&'static str> {
    let changed = match (base, target) {
        (PlainNode::Image(a), PlainNode::Image(b)) => a.id != b.id,
        (PlainNode::File(a), PlainNode::File(b)) => a.id != b.id,
        (PlainNode::Embed(a), PlainNode::Embed(b)) => a.id != b.id,
        (PlainNode::Archived(a), PlainNode::Archived(b)) => a.id != b.id,
        _ => false,
    };
    changed.then(|| element_name(base.as_type()).unwrap_or("?"))
}

fn type_name(t: NodeType) -> String {
    element_name(t).unwrap_or("?").to_string()
}

pub fn edit(base: State, target: &XmlTree) -> Result<EditOutcome, XmlError> {
    edit_bounded(base, target, MAX_EDIT_DISTANCE)
}

/// As [`edit`], with the bound on the block reorder search the anchors come
/// from — the fallback past it keeps a document too far reordered to search
/// reachable, at the price of moving every child.
pub(crate) fn edit_bounded(
    base: State,
    target: &XmlTree,
    block_lcs_bound: usize,
) -> Result<EditOutcome, XmlError> {
    validate_against(&base, target)?;
    let repairs_before = base.projected.repair_stats().repairs;
    let root = base
        .view()
        .root()
        .map(|r| r.id())
        .ok_or_else(|| XmlError::internal("no root"))?;

    let mut tr = Transaction::new(&base);
    tr.update_meta(|m| m.history = HistoryMeta::Skip);
    let outcome = {
        let mut diff = Diff::bounded(&mut tr, target, block_lcs_bound);
        diff.reconcile_node(root, &target.root, &[])?;
        diff.finish()?
    };
    let (state, ..) = tr.commit();

    let produced = sealed(state.to_plain());
    let wanted = sealed(reachable_target(target, &outcome.vanished));
    if produced != wanted {
        return Err(XmlError::internal(format!(
            "post-condition: {}",
            first_divergence(&produced, &wanted)
        )));
    }
    let repairs_after = state.projected.repair_stats().repairs;
    if repairs_after != repairs_before {
        return Err(XmlError::internal(format!(
            "post-condition: {} projection repairs",
            repairs_after - repairs_before
        )));
    }
    Ok(EditOutcome {
        state,
        changed: outcome.counts,
    })
}

/// The target without the scaffolds the reconcile found already gone: they are
/// projection-owned, so a file that names one describes a node the document
/// cannot hold once the slot it stood in is filled.
fn reachable_target(target: &XmlTree, vanished: &HashSet<Dot>) -> PlainDoc {
    if vanished.is_empty() {
        return target.to_plain_doc();
    }
    PlainDoc {
        root: prune(&target.root, vanished).to_plain_entry(),
    }
}

fn prune(node: &XmlNode, vanished: &HashSet<Dot>) -> XmlNode {
    let mut out = node.clone();
    out.children = node
        .children
        .iter()
        .filter_map(|child| match child {
            XmlChild::Block(block) => (!block.dot.is_some_and(|d| vanished.contains(&d)))
                .then(|| XmlChild::Block(prune(block, vanished))),
            XmlChild::Inline(_) => Some(child.clone()),
        })
        .collect();
    out
}

fn sealed(mut doc: PlainDoc) -> PlainDoc {
    seal(&mut doc.root, &[]);
    doc
}

fn seal(entry: &mut PlainNodeEntry, path: &[NodeType]) {
    let mut here = path.to_vec();
    here.push(entry.node.as_type());
    entry.modifiers = writable_modifiers(&entry.modifiers, &here);
    if is_opaque(entry.node.as_type()) {
        entry.children.clear();
        return;
    }
    for child in &mut entry.children {
        seal(child, &here);
    }
}

/// Never carries document content: element kinds, modifier kinds and counts
/// only — the message travels to logs and to the FFI boundary.
fn first_divergence(produced: &PlainDoc, wanted: &PlainDoc) -> String {
    fn shape(node: &PlainNode) -> String {
        match node {
            PlainNode::Text(_) => "text".to_string(),
            other => type_name(other.as_type()),
        }
    }
    fn kinds(modifiers: &BTreeMap<ModifierType, Modifier>) -> Vec<String> {
        modifiers.keys().copied().map(modifier_type_name).collect()
    }
    fn go(a: &PlainNodeEntry, b: &PlainNodeEntry, path: &mut Vec<usize>) -> Option<String> {
        if a.node != b.node {
            let what = match (&a.node, &b.node) {
                (PlainNode::Text(x), PlainNode::Text(y)) => format!(
                    "text differs ({} vs {} chars)",
                    x.text.chars().count(),
                    y.text.chars().count()
                ),
                _ if a.node.as_type() == b.node.as_type() => {
                    format!("<{}> attributes differ", shape(&a.node))
                }
                _ => format!("node {} vs {}", shape(&a.node), shape(&b.node)),
            };
            return Some(format!("at {path:?}: {what}"));
        }
        if a.modifiers != b.modifiers {
            return Some(format!(
                "at {path:?}: modifiers {:?} vs {:?}",
                kinds(&a.modifiers),
                kinds(&b.modifiers)
            ));
        }
        if a.carry != b.carry {
            let carry = |c: &[Modifier]| -> Vec<String> {
                c.iter().map(|m| modifier_type_name(m.as_type())).collect()
            };
            return Some(format!(
                "at {path:?}: carry {:?} vs {:?}",
                carry(&a.carry),
                carry(&b.carry)
            ));
        }
        if a.children.len() != b.children.len() {
            return Some(format!(
                "at {path:?}: {} vs {} children",
                a.children.len(),
                b.children.len()
            ));
        }
        for (i, (x, y)) in a.children.iter().zip(&b.children).enumerate() {
            path.push(i);
            if let Some(message) = go(x, y, path) {
                return Some(message);
            }
            path.pop();
        }
        None
    }
    go(&produced.root, &wanted.root, &mut Vec::new()).unwrap_or_else(|| "equal".to_string())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use hashbrown::HashSet as FastSet;

    use super::*;
    use crate::reader::from_xml;
    use crate::test_support::live_heads;
    use crate::writer::to_xml;

    fn rejected<T>(outcome: Result<T, XmlError>) -> XmlError {
        match outcome {
            Ok(_) => panic!("the edit was expected to fail"),
            Err(err) => err,
        }
    }

    #[test]
    fn combined_edit_applies_and_counts() {
        let (state, _p) = state! {
            doc { root { p: paragraph { text("hallo") } } }
            selection: (p, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap().replace(
            "hallo</paragraph>",
            "hello</paragraph>\n  <paragraph>new</paragraph>",
        );

        let out = edit(state, &from_xml(&xml).unwrap()).unwrap();

        let (expected, ..) = state! {
            doc { root { paragraph { text("hello") } paragraph { text("new") } } }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 1,
                chars_inserted: 4,
                chars_deleted: 1,
                ..Default::default()
            }
        );
    }

    #[test]
    fn unchanged_target_emits_nothing() {
        let (state, _p) = state! {
            doc { root { p: paragraph { text("same") } } }
            selection: (p, 0)
        };
        let heads: FastSet<Dot> = state.graph().current_heads().copied().collect();
        let xml = to_xml(&state, &live_heads(&state)).unwrap();

        let out = edit(state, &from_xml(&xml).unwrap()).unwrap();

        assert_eq!(out.changed, ChangeCounts::default());
        assert!(
            out.state
                .graph()
                .local_changesets_since(&heads)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_a_root_mismatch_an_unknown_dot_and_a_repeated_dot() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("x") } } }
            selection: (p, 0)
        };
        let root = state.view().root().expect("root").id();
        let xml = to_xml(&state, &live_heads(&state)).unwrap();

        let bad_root = xml.replacen(&format!("dot=\"{root}\""), "dot=\"8_8\"", 1);
        let err = rejected(edit(state.clone(), &from_xml(&bad_root).unwrap()));
        assert_eq!(*err.detail, XmlErrorDetail::RootDotMismatch);

        let bad_dot = xml.replace(&format!("dot=\"{p}\""), "dot=\"9_9\"");
        let err = rejected(edit(state.clone(), &from_xml(&bad_dot).unwrap()));
        assert!(matches!(
            *err.detail,
            XmlErrorDetail::DotNotInDocument { .. }
        ));
        assert_eq!(err.dot, Some("9_9".to_string()));

        let mut repeated = from_xml(&xml).unwrap();
        let twin = repeated.root.children[0].clone();
        repeated.root.children.push(twin);
        let err = rejected(validate_against(&state, &repeated));
        assert!(matches!(*err.detail, XmlErrorDetail::DotDuplicate { .. }));
        assert_eq!(err.dot, Some(p.to_string()));
    }

    #[test]
    fn rejects_a_type_change_the_document_cannot_make() {
        let (state, a, b) = state! {
            doc { root { a: paragraph { text("x") } b: paragraph { text("y") } } }
            selection: (a, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = xml.replace(
            &format!("<paragraph dot=\"{a}\">x</paragraph>"),
            &format!("<unknown dot=\"{a}\"/>"),
        );
        assert_ne!(target, xml, "the target must differ from the source");
        assert!(target.contains(&format!("dot=\"{b}\"")));

        let err = rejected(edit(state, &from_xml(&target).unwrap()));
        assert!(matches!(
            *err.detail,
            XmlErrorDetail::DotTypeIncompatible { ref new_type, .. } if new_type == "unknown"
        ));
        assert_eq!(err.dot, Some(a.to_string()));
    }

    #[test]
    fn a_target_the_diff_cannot_reach_fails_the_post_condition() {
        let (state, _p) = state! {
            doc { root { p: paragraph { text("x") } } }
            selection: (p, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let mut target = from_xml(&xml).unwrap();
        let nested = {
            let XmlChild::Block(para) = &target.root.children[0] else {
                panic!("the first child is a block");
            };
            let mut nested = para.clone();
            nested.dot = None;
            nested.children.clear();
            nested
        };
        let XmlChild::Block(para) = &mut target.root.children[0] else {
            unreachable!()
        };
        para.children.push(XmlChild::Block(nested));

        let before = state.to_plain();
        let err = rejected(edit(state.clone(), &target));
        assert!(matches!(*err.detail, XmlErrorDetail::Internal { .. }));
        assert!(
            err.message.contains("post-condition"),
            "unexpected message: {}",
            err.message
        );
        assert_eq!(state.to_plain(), before, "the caller's state is untouched");
    }

    fn trailing_scaffold(state: &State) -> Dot {
        let dot = state
            .view()
            .root()
            .expect("root")
            .child_blocks()
            .map(|b| b.id())
            .last()
            .expect("the normalizer appends a trailing paragraph");
        assert!(dot.is_synthetic(), "the fixture must end in a scaffold");
        dot
    }

    fn atom_document() -> (State, Dot, Dot, Dot, Dot) {
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
        (state, p1, image, rule, p2)
    }

    fn tag_line(xml: &str, tag: &str) -> String {
        xml.lines()
            .find(|line| line.trim_start().starts_with(tag))
            .map(|line| format!("{line}\n"))
            .expect("the element has a line of its own")
    }

    fn unwrapped(xml: &str, tags: &[&str]) -> String {
        let mut out = xml.to_string();
        for tag in tags {
            out = out.replace(&tag_line(xml, tag), "");
        }
        assert_ne!(&out, xml, "the target must differ from the source");
        out
    }

    fn leaf_dots(state: &State, block: Dot) -> Vec<Dot> {
        let view = state.view();
        let Some(nv) = view.node(block) else {
            return Vec::new();
        };
        nv.children()
            .filter_map(|child| match child {
                editor_model::ChildView::Leaf(leaf) => Some(leaf.dot()),
                editor_model::ChildView::Block(_) => None,
            })
            .collect()
    }

    fn survivor(state: &State, dot: Dot) -> Dot {
        let view = state.view();
        view.alias_classes()
            .resolve_with(dot, |d| view.node(d).is_some() || view.leaf(d).is_some())
    }

    fn atom_line(xml: &str, dot: Dot) -> String {
        xml.lines()
            .find(|line| line.contains(&format!("dot=\"{dot}\"")))
            .map(|line| format!("{line}\n"))
            .expect("the atom has a line of its own")
    }

    #[test]
    fn deleting_a_block_atom_line_removes_only_that_atom() {
        let (state, p1, image, rule, p2) = atom_document();
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = xml.replace(&atom_line(&xml, image), "");
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_deleted: 1,
                ..Default::default()
            }
        );
        let view = out.state.view();
        assert!(view.leaf(image).is_none(), "the image dot is gone");
        assert!(view.leaf(rule).is_some(), "the rule keeps its dot");
        assert!(view.node(p1).is_some() && view.node(p2).is_some());
    }

    #[test]
    fn inserting_a_block_atom_line_creates_it() {
        let (state, _p1, _image, rule, _p2) = atom_document();
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let rule_line = atom_line(&xml, rule);
        let target = xml.replace(
            &rule_line,
            &format!("{rule_line}  <horizontal_rule attr:variant=\"dashed_line\"/>\n"),
        );

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 1,
                ..Default::default()
            }
        );
        let types: Vec<NodeType> = out
            .state
            .to_plain()
            .root
            .children
            .iter()
            .map(|c| c.node.as_type())
            .collect();
        assert_eq!(
            types,
            vec![
                NodeType::Paragraph,
                NodeType::Image,
                NodeType::HorizontalRule,
                NodeType::HorizontalRule,
                NodeType::Paragraph,
            ]
        );
    }

    #[test]
    fn changing_a_block_atom_attribute_keeps_its_dot() {
        let (state, _p1, image, ..) = atom_document();
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = xml.replace("attr:proportion=\"100\"", "attr:proportion=\"50\"");
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_updated: 1,
                ..Default::default()
            }
        );
        assert_eq!(
            out.state.to_plain().root.children[1].node,
            PlainNode::Image(editor_model::PlainImageNode {
                id: Some("IMG1".to_string()),
                proportion: 50,
            })
        );
        assert!(out.state.view().leaf(image).is_some(), "the dot survives");
    }

    #[test]
    fn changing_a_block_atom_modifier_writes_the_store_to_plain_reads() {
        use editor_model::{Alignment, Modifier, ModifierType};

        let (state, _p1, image, ..) = atom_document();
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = xml.replace("mod:alignment=\"center\"", "mod:alignment=\"right\"");
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_updated: 1,
                ..Default::default()
            }
        );
        assert_eq!(
            out.state.to_plain().root.children[1]
                .modifiers
                .get(&ModifierType::Alignment),
            Some(&Modifier::Alignment {
                value: Alignment::Right
            })
        );
        assert_eq!(
            out.state
                .projected
                .projected()
                .block_modifiers
                .get(&image)
                .map(|m| m.len()),
            None,
            "an atom's modifiers never go to the block-modifier store"
        );
        assert!(out.state.view().leaf(image).is_some(), "the dot survives");
    }

    #[test]
    fn moving_a_block_atom_inside_its_container_keeps_its_entry() {
        let (state, _p1, image, rule, _p2) = atom_document();
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let image_line = atom_line(&xml, image);
        let rule_line = atom_line(&xml, rule);
        let target = xml.replace(
            &format!("{image_line}{rule_line}"),
            &format!("{rule_line}{image_line}"),
        );
        assert_ne!(target, xml, "the target must differ from the source");

        let before = state.to_plain();
        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(out.changed.blocks_moved, 1);
        assert_eq!(out.changed.blocks_deleted, 0);
        assert_eq!(out.changed.blocks_inserted, 0);
        assert_eq!(
            out.changed.blocks_updated, 1,
            "the image is the moved child: the re-published atom loses its span modifier and the shape pass restores it"
        );

        let plain = out.state.to_plain();
        assert_eq!(
            plain.root.children[2], before.root.children[1],
            "the image entry survives the move unchanged"
        );
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    horizontal_rule
                    image(id: Some("IMG1".to_string())) [alignment(Alignment::Center)]
                    paragraph { text("b") }
                }
            }
            selection: none
        };
        assert_eq!(plain, expected.to_plain());
    }

    #[test]
    fn a_modifier_the_writer_omits_does_not_fail_the_post_condition() {
        let (state, p) = state! {
            doc { root { p: paragraph [font_size(99)] { text("a") } } }
            selection: (p, 0)
        };
        assert!(
            state
                .projected
                .projected()
                .block_modifiers
                .get(&p)
                .is_some_and(|m| m.contains_key(&ModifierType::FontSize)),
            "the fixture must actually hold the out-of-range block modifier"
        );
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        assert!(!xml.contains("font_size=\"99\""));

        let out = edit(state, &from_xml(&xml).unwrap()).unwrap();

        assert_eq!(out.changed, ChangeCounts::default());
        assert_eq!(
            out.state
                .projected
                .projected()
                .block_modifiers
                .get(&p)
                .and_then(|m| m.get(&ModifierType::FontSize)),
            Some(&Modifier::FontSize { value: 99 }),
            "a modifier the file cannot carry is left untouched"
        );
    }

    #[test]
    fn moving_a_block_atom_out_of_a_container_keeps_its_content_and_modifiers() {
        use editor_model::{Alignment, Modifier, ModifierType};

        let (state, _title, image, tail) = state! {
            doc {
                root {
                    fold {
                        title: fold_title { }
                        fold_content {
                            image: image(id: Some("IMG1".to_string())) [alignment(Alignment::Center)]
                            paragraph { text("x") }
                        }
                    }
                    tail: paragraph { }
                }
            }
            selection: (title, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let image_line = atom_line(&xml, image);
        let tail_line = format!("  <paragraph dot=\"{tail}\"></paragraph>\n");
        assert!(xml.contains(&tail_line), "unexpected xml: {xml}");
        let target = xml
            .replace(&image_line, "")
            .replace(&tail_line, &format!("{image_line}{tail_line}"));
        assert_ne!(target, xml, "the target must differ from the source");

        let before = state.to_plain();
        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_moved: 1,
                blocks_updated: 1,
                ..Default::default()
            },
            "the re-published atom loses its span modifiers and the shape pass restores them"
        );
        let plain = out.state.to_plain();
        let moved = &plain.root.children[1];
        assert_eq!(moved, &before.root.children[0].children[1].children[0]);
        assert_eq!(
            moved.modifiers.get(&ModifierType::Alignment),
            Some(&Modifier::Alignment {
                value: Alignment::Center
            })
        );
    }

    #[test]
    fn changing_an_asset_id_is_rejected_as_opaque() {
        let (state, ..) = atom_document();
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = xml.replace("attr:id=\"IMG1\"", "attr:id=\"IMG2\"");
        assert_ne!(target, xml, "the target must differ from the source");

        let err = rejected(edit(state, &from_xml(&target).unwrap()));
        assert!(matches!(
            *err.detail,
            XmlErrorDetail::OpaqueIdChanged { ref element, .. } if element == "image"
        ));
    }

    #[test]
    fn a_block_atom_cannot_become_a_block() {
        let (state, _p1, image, ..) = atom_document();
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let image_line = atom_line(&xml, image);
        let target = xml.replace(
            &image_line,
            &format!("  <paragraph dot=\"{image}\">z</paragraph>\n"),
        );

        let err = rejected(edit(state, &from_xml(&target).unwrap()));
        assert!(matches!(
            *err.detail,
            XmlErrorDetail::DotTypeIncompatible { ref new_type, .. } if new_type == "paragraph"
        ));
    }

    #[test]
    fn the_divergence_report_never_carries_document_text() {
        let (with_link, _a) = state! {
            doc { root { a: paragraph { text("qqq") [link(href: "https://example.test/zzz".to_string())] } } }
            selection: (a, 0)
        };
        let (plain_text, _b) = state! {
            doc { root { b: paragraph { text("qqq") } } }
            selection: (b, 0)
        };
        let (other_text, _c) = state! {
            doc { root { c: paragraph { text("www") } } }
            selection: (c, 0)
        };

        let secrets = ["qqq", "www", "zzz", "example.test"];
        for (left, right) in [
            (&with_link, &plain_text),
            (&plain_text, &other_text),
            (&other_text, &with_link),
        ] {
            let message = first_divergence(&left.to_plain(), &right.to_plain());
            assert_ne!(message, "equal");
            for secret in secrets {
                assert!(
                    !message.contains(secret),
                    "`{secret}` leaked into: {message}"
                );
            }
        }

        assert_eq!(
            first_divergence(&with_link.to_plain(), &plain_text.to_plain()),
            "at [0, 0]: modifiers [\"link\"] vs []"
        );
        assert_eq!(
            first_divergence(&plain_text.to_plain(), &other_text.to_plain()),
            "at [0, 0]: text differs (3 vs 3 chars)"
        );
    }

    /// An opaque block whose store carries a modifier the schema does not place
    /// on it: `Transaction::add_modifier` never reads `ModifierSpec.context`,
    /// so a peer or an older build can leave one behind.
    fn opaque_block_with_a_stray_modifier() -> (State, Dot) {
        use editor_crdt::ListOp;
        use editor_model::{EditOp, SeqItem};

        let (mut state, _p) = state! {
            doc { root { p: paragraph { text("ab") } } }
            selection: (p, 0)
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
        let mut tr = Transaction::new(&state);
        tr.add_modifier(unknown, Modifier::LineHeight { value: 160 })
            .expect("the step does not read the modifier's context");
        let (state, ..) = tr.commit();
        (state, unknown)
    }

    fn stray_modifier(state: &State, dot: Dot) -> Option<BTreeMap<ModifierType, Modifier>> {
        state
            .projected
            .projected()
            .block_modifiers
            .get(&dot)
            .cloned()
    }

    #[test]
    fn a_modifier_the_schema_places_elsewhere_stays_out_of_the_file() {
        let (state, unknown) = opaque_block_with_a_stray_modifier();
        assert_eq!(
            stray_modifier(&state, unknown),
            Some(BTreeMap::from([(
                ModifierType::LineHeight,
                Modifier::LineHeight { value: 160 }
            )])),
            "the store holds the modifier the file must not carry"
        );

        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        assert!(
            xml.contains(&format!("<unknown dot=\"{unknown}\"/>")),
            "{xml}"
        );
    }

    #[test]
    fn a_file_the_stray_modifier_never_reached_reads_back_as_no_change() {
        let (state, unknown) = opaque_block_with_a_stray_modifier();
        let xml = to_xml(&state, &live_heads(&state)).unwrap();

        let out = edit(state, &from_xml(&xml).unwrap()).unwrap();

        assert_eq!(out.changed, ChangeCounts::default());
        assert_eq!(
            stray_modifier(&out.state, unknown),
            Some(BTreeMap::from([(
                ModifierType::LineHeight,
                Modifier::LineHeight { value: 160 }
            )])),
            "a value the file cannot carry is not a removal"
        );
    }

    #[test]
    fn an_unknown_block_atom_is_written_and_left_alone() {
        use editor_crdt::ListOp;
        use editor_model::{AtomLeaf, EditOp, SeqItem, UnknownNode};

        let (mut state, _p) = state! {
            doc { root { p: paragraph { text("ab") } } }
            selection: (p, 0)
        };
        let unknown = state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: SeqItem::BlockAtom {
                    leaf: AtomLeaf::Unknown(UnknownNode),
                    parents: vec![Dot::ROOT],
                },
            }))
            .unwrap()
            .id;
        assert!(state.view().leaf(unknown).is_some());

        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        assert!(
            xml.contains(&format!("<unknown dot=\"{unknown}\"/>")),
            "unexpected xml: {xml}"
        );

        let out = edit(state, &from_xml(&xml).unwrap()).unwrap();

        assert_eq!(out.changed, ChangeCounts::default());
        assert!(out.state.view().leaf(unknown).is_some());
    }

    #[test]
    fn an_opaque_block_with_hidden_children_survives_the_post_condition() {
        use editor_crdt::ListOp;
        use editor_model::{EditOp, SeqItem};

        let (mut state, _p) = state! {
            doc { root { p: paragraph { text("ab") } } }
            selection: (p, 0)
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
        state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 4,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, unknown],
                    attrs: vec![],
                },
            }))
            .unwrap();
        let hidden = editor_state::to_plain_subtree(&state, unknown).expect("opaque subtree");
        assert_eq!(hidden.children.len(), 1);

        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        assert!(xml.contains(&format!("<unknown dot=\"{unknown}\"/>")));

        let out = edit(state, &from_xml(&xml).unwrap()).unwrap();

        assert_eq!(out.changed, ChangeCounts::default());
        assert_eq!(
            editor_state::to_plain_subtree(&out.state, unknown),
            Some(hidden)
        );
    }

    #[test]
    fn appending_a_paragraph_past_the_trailing_scaffold_drops_the_scaffold() {
        let (state, _p) = state! {
            doc { root { p: paragraph { text("a") } horizontal_rule } }
            selection: (p, 0)
        };
        let scaffold = trailing_scaffold(&state);

        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        assert!(xml.contains(&format!("<paragraph dot=\"{scaffold}\"></paragraph>")));
        let target = xml.replace("</root>", "  <paragraph>hi</paragraph>\n</root>");
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 1,
                chars_inserted: 2,
                ..Default::default()
            }
        );
        let (expected, ..) = state! {
            doc { root { paragraph { text("a") } horizontal_rule paragraph { text("hi") } } }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        assert_eq!(out.state.to_plain().root.children.len(), 3);
        assert!(out.state.view().node(scaffold).is_none());
    }
    #[test]
    fn writing_into_the_trailing_scaffold_and_appending_keeps_both_paragraphs() {
        let (state, ..) = state! {
            doc { root { horizontal_rule } }
            selection: none
        };
        let scaffold = trailing_scaffold(&state);
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = xml
            .replace(
                &format!("<paragraph dot=\"{scaffold}\"></paragraph>"),
                &format!("<paragraph dot=\"{scaffold}\">hello</paragraph>"),
            )
            .replace("</root>", "  <paragraph>hi</paragraph>\n</root>");
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 1,
                chars_inserted: 7,
                ..Default::default()
            }
        );
        let (expected, ..) = state! {
            doc { root { horizontal_rule paragraph { text("hello") } paragraph { text("hi") } } }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        assert_eq!(out.state.to_plain().root.children.len(), 3);
        assert!(out.state.view().node(scaffold).is_none());
    }
    #[test]
    fn reordering_a_written_scaffold_behind_a_new_paragraph_keeps_its_text() {
        let (state, ..) = state! {
            doc { root { horizontal_rule } }
            selection: none
        };
        let scaffold = trailing_scaffold(&state);
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let line = format!("  <paragraph dot=\"{scaffold}\"></paragraph>\n");
        assert!(xml.contains(&line), "unexpected xml: {xml}");
        let target = xml.replace(
            &line,
            &format!(
                "  <paragraph>hi</paragraph>\n  <paragraph dot=\"{scaffold}\">hello</paragraph>\n"
            ),
        );

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 2,
                chars_inserted: 7,
                ..Default::default()
            },
            "the scaffold was already gone, so its text is carried into a new block"
        );
        let (expected, ..) = state! {
            doc { root { horizontal_rule paragraph { text("hi") } paragraph { text("hello") } } }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        assert_eq!(out.state.to_plain().root.children.len(), 3);
    }

    #[test]
    fn unwrapping_a_blockquote_moves_its_paragraph_out_instead_of_deleting_it() {
        let (state, _p1, p2, _p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    blockquote { p2: paragraph { text("bb") } }
                    p3: paragraph { text("c") }
                }
            }
            selection: (p1, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = unwrapped(&xml, &["<blockquote", "</blockquote>"]);

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_deleted: 1,
                blocks_moved: 1,
                ..Default::default()
            }
        );
        let (expected, ..) = state! {
            doc { root { paragraph { text("a") } paragraph { text("bb") } paragraph { text("c") } } }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        let moved = survivor(&out.state, p2);
        assert!(
            out.state.view().node(moved).is_some(),
            "the unwrapped paragraph keeps its identity"
        );
    }

    #[test]
    fn unwrapping_a_list_lifts_the_item_paragraph_to_the_root() {
        let (state, _p1, p2, _p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    bullet_list { list_item { p2: paragraph { text("bb") } } }
                    p3: paragraph { text("c") }
                }
            }
            selection: (p1, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = unwrapped(
            &xml,
            &[
                "<bullet_list",
                "<list_item",
                "</list_item>",
                "</bullet_list>",
            ],
        );

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_deleted: 1,
                blocks_moved: 1,
                ..Default::default()
            },
            "the list and its item leave as one subtree"
        );
        let (expected, ..) = state! {
            doc { root { paragraph { text("a") } paragraph { text("bb") } paragraph { text("c") } } }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        let moved = survivor(&out.state, p2);
        assert!(out.state.view().node(moved).is_some());
    }

    fn survivors(state: &State, dots: &[Dot]) -> Vec<Dot> {
        dots.iter().map(|d| survivor(state, *d)).collect()
    }

    #[test]
    fn wrapping_a_paragraph_in_a_new_blockquote_moves_it_instead_of_copying_it() {
        let (state, _p1, p2, _p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("bb") }
                    p3: paragraph { text("c") }
                }
            }
            selection: (p1, 0)
        };
        let before = leaf_dots(&state, p2);
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let line = atom_line(&xml, p2);
        let target = xml.replace(&line, &format!("  <blockquote>\n  {line}  </blockquote>\n"));
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 1,
                blocks_moved: 1,
                ..Default::default()
            }
        );
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    blockquote { paragraph { text("bb") } }
                    paragraph { text("c") }
                }
            }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        let moved = survivor(&out.state, p2);
        assert!(
            out.state.view().node(moved).is_some(),
            "the wrapped paragraph keeps its identity"
        );
        assert_eq!(
            leaf_dots(&out.state, moved),
            survivors(&out.state, &before),
            "and so does every leaf it had"
        );
    }

    #[test]
    fn wrapping_two_paragraphs_in_a_new_list_moves_both_into_their_items() {
        let (state, p1, p2, _p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("bb") }
                    p3: paragraph { text("c") }
                }
            }
            selection: (p1, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let first = atom_line(&xml, p1);
        let second = atom_line(&xml, p2);
        let target = xml.replace(
            &format!("{first}{second}"),
            &format!(
                "  <bullet_list>\n    <list_item>\n    {first}    </list_item>\n\
                 \x20   <list_item>\n    {second}    </list_item>\n  </bullet_list>\n"
            ),
        );
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 1,
                blocks_moved: 2,
                ..Default::default()
            },
            "the list and both of its items arrive as one subtree"
        );
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("a") } }
                        list_item { paragraph { text("bb") } }
                    }
                    paragraph { text("c") }
                }
            }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        for dot in [p1, p2] {
            assert!(
                out.state.view().node(survivor(&out.state, dot)).is_some(),
                "the wrapped paragraph keeps its identity"
            );
        }
    }

    #[test]
    fn wrapping_an_empty_paragraph_is_not_mistaken_for_the_scaffold_beside_it() {
        let (state, p, _tail) = state! {
            doc {
                root {
                    blockquote { p: paragraph { } }
                    tail: paragraph { text("z") }
                }
            }
            selection: (p, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let line = atom_line(&xml, p);
        let target = xml.replace(
            &line,
            &format!(
                "    <bullet_list>\n      <list_item>\n      {line}      </list_item>\n\
                 \x20   </bullet_list>\n"
            ),
        );
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 1,
                blocks_moved: 1,
                ..Default::default()
            }
        );
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote { bullet_list { list_item { paragraph { } } } }
                    paragraph { text("z") }
                }
            }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        assert!(
            out.state.view().node(survivor(&out.state, p)).is_some(),
            "the wrapped paragraph keeps its identity"
        );
    }

    #[test]
    fn wrapping_a_paragraph_in_a_new_fold_fills_the_fixed_slots_once() {
        let (state, _p1, p2, _p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("bb") }
                    p3: paragraph { text("c") }
                }
            }
            selection: (p1, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let line = atom_line(&xml, p2);
        let target = xml.replace(
            &line,
            &format!(
                "  <fold>\n    <fold_title>t</fold_title>\n    <fold_content>\n    {line}\
                 \x20   </fold_content>\n  </fold>\n"
            ),
        );
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 1,
                blocks_moved: 1,
                chars_inserted: 1,
                ..Default::default()
            },
            "the fold arrives with its title and content, not a second pair"
        );
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    fold {
                        fold_title { text("t") }
                        fold_content { paragraph { text("bb") } }
                    }
                    paragraph { text("c") }
                }
            }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        assert!(
            out.state.view().node(survivor(&out.state, p2)).is_some(),
            "the wrapped paragraph keeps its identity"
        );
    }

    #[test]
    fn wrapping_a_paragraph_while_editing_its_text_does_both() {
        let (state, _p1, p2, _p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("bb") }
                    p3: paragraph { text("c") }
                }
            }
            selection: (p1, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let line = atom_line(&xml, p2);
        let edited = line.replace("bb</paragraph>", "bc</paragraph>");
        let target = xml.replace(&line, &format!("  <callout>\n  {edited}  </callout>\n"));
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 1,
                blocks_moved: 1,
                chars_inserted: 1,
                chars_deleted: 1,
                ..Default::default()
            }
        );
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    callout { paragraph { text("bc") } }
                    paragraph { text("c") }
                }
            }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        assert!(
            out.state.view().node(survivor(&out.state, p2)).is_some(),
            "the wrapped paragraph keeps its identity"
        );
    }

    #[test]
    fn wrapping_a_paragraph_below_a_new_line_that_holds_an_atom() {
        let (state, _p1, p2, _p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("bb") }
                    p3: paragraph { text("c") }
                }
            }
            selection: (p1, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let line = atom_line(&xml, p2);
        let target = xml.replace(
            &line,
            &format!(
                "  <blockquote>\n    <paragraph>x<hard_break/></paragraph>\n  {line}  </blockquote>\n"
            ),
        );
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 1,
                blocks_moved: 1,
                chars_inserted: 1,
                ..Default::default()
            },
            "the break is an atom, not a character"
        );
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    blockquote {
                        paragraph { text("x") hard_break }
                        paragraph { text("bb") }
                    }
                    paragraph { text("c") }
                }
            }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        assert!(
            out.state.view().node(survivor(&out.state, p2)).is_some(),
            "the wrapped paragraph keeps its identity"
        );
    }

    #[test]
    fn a_file_that_drops_the_scaffold_after_a_page_break_is_refused() {
        let (state, _p1, _p2) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("b") page_break }
                }
            }
            selection: (p1, 0)
        };
        let scaffold = state
            .view()
            .root()
            .expect("root")
            .children()
            .filter_map(|child| match child {
                editor_model::ChildView::Block(block) => Some(block.id()),
                editor_model::ChildView::Leaf(_) => None,
            })
            .last()
            .expect("the projection completes the root with a paragraph");
        assert!(scaffold.is_synthetic());
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        edit(state, &from_xml(&xml).unwrap()).expect("the document writes a file it can hold");

        let target = xml.replace(&atom_line(&xml, scaffold), "");
        assert_ne!(target, xml, "the target must differ from the source");

        let err = from_xml(&target).unwrap_err();
        assert_eq!(*err.detail, XmlErrorDetail::TrailingPageBreak);
    }

    #[test]
    fn a_file_that_drops_a_table_cell_is_refused() {
        let (state, _p1, c4) = state! {
            doc { root { table {
                table_row {
                    table_cell { p1: paragraph { text("a") } }
                    table_cell { paragraph { text("b") } }
                }
                table_row {
                    table_cell { paragraph { text("c") } }
                    c4: table_cell { paragraph { text("d") } }
                }
            } } }
            selection: (p1, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let lines: Vec<&str> = xml.lines().collect();
        let start = lines
            .iter()
            .position(|l| l.contains(&format!("dot=\"{c4}\"")))
            .expect("the cell opens on a line of its own");
        let end = start
            + lines[start..]
                .iter()
                .position(|l| l.contains("</table_cell>"))
                .expect("and closes on a later one");
        let cut: String = lines[start..=end]
            .iter()
            .map(|l| format!("{l}\n"))
            .collect();
        let target = xml.replace(&cut, "");
        assert_ne!(target, xml, "the target must differ from the source");

        let err = from_xml(&target).unwrap_err();
        assert_eq!(
            *err.detail,
            XmlErrorDetail::TableNotRectangular {
                expected: 2,
                got: 1
            }
        );
        assert_eq!(err.pos.map(|p| p.line), Some(11), "the short row's line");
    }

    #[test]
    fn wrapping_a_paragraph_in_a_new_fold_that_also_holds_a_new_rule() {
        let (state, _p1, p2, _p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("bb") }
                    p3: paragraph { text("c") }
                }
            }
            selection: (p1, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let line = atom_line(&xml, p2);
        let target = xml.replace(
            &line,
            &format!(
                "  <fold>\n    <fold_title>t</fold_title>\n    <fold_content>\n\
                 \x20     <horizontal_rule/>\n  {line}    </fold_content>\n  </fold>\n"
            ),
        );
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 1,
                blocks_moved: 1,
                chars_inserted: 1,
                ..Default::default()
            },
            "the rule rides in with the fold and pairs with its own minted dot"
        );
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    fold {
                        fold_title { text("t") }
                        fold_content {
                            horizontal_rule
                            paragraph { text("bb") }
                        }
                    }
                    paragraph { text("c") }
                }
            }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        assert!(
            out.state.view().node(survivor(&out.state, p2)).is_some(),
            "the wrapped paragraph keeps its identity"
        );
    }

    #[test]
    fn a_reorder_past_the_block_bound_lands_the_same_document() {
        let (state, p1, p2, p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("b") }
                    p3: paragraph { text("c") }
                }
            }
            selection: (p1, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let lines: Vec<String> = [p1, p2, p3].iter().map(|d| atom_line(&xml, *d)).collect();
        let target = xml.replace(
            &lines.concat(),
            &format!("{}{}{}", lines[1], lines[2], lines[0]),
        );
        assert_ne!(target, xml, "the target must differ from the source");
        let tree = from_xml(&target).unwrap();

        let searched = edit_bounded(state.clone(), &tree, MAX_EDIT_DISTANCE).unwrap();
        let fallback = edit_bounded(state, &tree, 0).unwrap();

        assert_eq!(fallback.state.to_plain(), searched.state.to_plain());
        assert_eq!(
            (searched.changed.blocks_moved, fallback.changed.blocks_moved),
            (1, 2),
            "with no anchors a child the search would have left alone is moved too"
        );
        assert_eq!(
            ChangeCounts {
                blocks_moved: 0,
                ..fallback.changed
            },
            ChangeCounts {
                blocks_moved: 0,
                ..searched.changed
            },
            "nothing but the moves differs"
        );
        for dot in [p1, p2, p3] {
            for out in [&searched, &fallback] {
                let alive = survivor(&out.state, dot);
                assert!(
                    out.state.view().node(alive).is_some(),
                    "the reordered block keeps its identity"
                );
            }
        }
    }

    #[test]
    fn deleting_a_container_the_target_drops_entirely_still_counts_its_characters() {
        let (state, _p1, p2, _p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    blockquote { p2: paragraph { text("bb") } }
                    p3: paragraph { text("c") }
                }
            }
            selection: (p1, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = xml
            .replace(&tag_line(&xml, "<blockquote"), "")
            .replace(&atom_line(&xml, p2), "")
            .replace(&tag_line(&xml, "</blockquote>"), "");
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_deleted: 1,
                chars_deleted: 2,
                ..Default::default()
            }
        );
        let (expected, ..) = state! {
            doc { root { paragraph { text("a") } paragraph { text("c") } } }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
    }

    #[test]
    fn splitting_a_paragraph_keeps_the_first_half_where_it_is() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("abcd") } } }
            selection: (p1, 0)
        };
        let before = leaf_dots(&state, p1);
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = xml.replace(
            "abcd</paragraph>",
            "ab</paragraph>\n  <paragraph>cd</paragraph>",
        );
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_inserted: 1,
                chars_inserted: 2,
                chars_deleted: 2,
                ..Default::default()
            }
        );
        let (expected, ..) = state! {
            doc { root { paragraph { text("ab") } paragraph { text("cd") } } }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        assert_eq!(
            leaf_dots(&out.state, p1),
            before[..2],
            "the first half keeps the leaves it had"
        );
    }

    #[test]
    fn merging_two_paragraphs_keeps_the_leaves_of_the_first() {
        let (state, p1, p2) = state! {
            doc { root { p1: paragraph { text("ab") } p2: paragraph { text("cd") } } }
            selection: (p1, 0)
        };
        let before = leaf_dots(&state, p1);
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = xml
            .replace("ab</paragraph>", "abcd</paragraph>")
            .replace(&atom_line(&xml, p2), "");
        assert_ne!(target, xml, "the target must differ from the source");

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        assert_eq!(
            out.changed,
            ChangeCounts {
                blocks_deleted: 1,
                chars_inserted: 2,
                chars_deleted: 2,
                ..Default::default()
            }
        );
        let (expected, ..) = state! {
            doc { root { paragraph { text("abcd") } } }
            selection: none
        };
        assert_eq!(out.state.to_plain(), expected.to_plain());
        assert_eq!(
            leaf_dots(&out.state, p1)[..2],
            before[..],
            "the surviving paragraph keeps every leaf it had"
        );
    }

    fn paragraph_subtree(text: &str) -> editor_model::Subtree {
        editor_model::Subtree {
            node: editor_model::PlainNode::Paragraph(editor_model::PlainParagraphNode {}),
            modifiers: Vec::new(),
            carry: Vec::new(),
            children: vec![editor_model::Subtree {
                node: editor_model::PlainNode::Text(editor_model::PlainTextNode {
                    text: text.into(),
                }),
                modifiers: Vec::new(),
                carry: Vec::new(),
                children: Vec::new(),
                source_dots: Vec::new(),
            }],
            source_dots: Vec::new(),
        }
    }

    fn paragraph_texts(state: &State) -> Vec<String> {
        fn walk(entry: &editor_model::PlainNodeEntry, out: &mut Vec<String>) {
            if matches!(entry.node, editor_model::PlainNode::Paragraph(_)) {
                let text: String = entry
                    .children
                    .iter()
                    .filter_map(|c| match &c.node {
                        editor_model::PlainNode::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect();
                out.push(text);
            }
            for c in &entry.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(&state.to_plain().root, &mut out);
        out
    }

    #[test]
    fn swapping_two_runs_of_paragraphs_moves_them_without_duplicating() {
        let (state, _z) = state! {
            doc { root { z: paragraph { text("Z") } } }
            selection: (z, 0)
        };
        let root = state.view().root().unwrap().id();
        let mut tr = editor_transaction::Transaction::new(&state);
        let labels: Vec<String> = (1..=16)
            .map(|i| format!("A{i:02}"))
            .chain((1..=16).map(|i| format!("B{i:02}")))
            .collect();
        for (i, label) in labels.iter().enumerate() {
            tr.insert_subtree(root, i, paragraph_subtree(label))
                .unwrap();
        }
        let state = tr.commit().0;
        assert_eq!(paragraph_texts(&state).len(), 33);

        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let lines: Vec<&str> = xml.lines().collect();
        assert_eq!(lines.len(), 35, "root + 33 paragraphs + /root");
        let mut swapped: Vec<&str> = vec![lines[0]];
        swapped.extend_from_slice(&lines[17..33]);
        swapped.extend_from_slice(&lines[1..17]);
        swapped.extend_from_slice(&lines[33..]);
        let target = swapped.join("\n") + "\n";

        let out = edit(state, &from_xml(&target).unwrap()).unwrap();

        let expected: Vec<String> = (1..=16)
            .map(|i| format!("B{i:02}"))
            .chain((1..=16).map(|i| format!("A{i:02}")))
            .chain(std::iter::once("Z".to_string()))
            .collect();
        assert_eq!(paragraph_texts(&out.state), expected);
        assert_eq!(out.changed.blocks_inserted, 0);
        assert_eq!(out.changed.blocks_deleted, 0);
    }

    fn swap_document() -> (State, String) {
        let (state, _z) = state! {
            doc { root { z: paragraph { text("Z") } } }
            selection: (z, 0)
        };
        let root = state.view().root().unwrap().id();
        let mut tr = editor_transaction::Transaction::new(&state);
        let labels: Vec<String> = (1..=16)
            .map(|i| format!("A{i:02}"))
            .chain((1..=16).map(|i| format!("B{i:02}")))
            .collect();
        for (i, label) in labels.iter().enumerate() {
            tr.insert_subtree(root, i, paragraph_subtree(label))
                .unwrap();
        }
        let state = tr.commit().0;
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let lines: Vec<&str> = xml.lines().collect();
        let mut swapped: Vec<&str> = vec![lines[0]];
        swapped.extend_from_slice(&lines[17..33]);
        swapped.extend_from_slice(&lines[1..17]);
        swapped.extend_from_slice(&lines[33..]);
        (state, swapped.join("\n") + "\n")
    }

    fn swapped_expected() -> Vec<String> {
        (1..=16)
            .map(|i| format!("B{i:02}"))
            .chain((1..=16).map(|i| format!("A{i:02}")))
            .chain(std::iter::once("Z".to_string()))
            .collect()
    }

    #[test]
    fn swapping_two_runs_past_the_anchor_bound_moves_every_child_once() {
        let (state, target) = swap_document();
        let out = edit_bounded(state, &from_xml(&target).unwrap(), 1).unwrap();
        assert_eq!(paragraph_texts(&out.state), swapped_expected());
        assert_eq!(out.changed.blocks_inserted, 0);
        assert_eq!(out.changed.blocks_deleted, 0);
    }

    #[test]
    fn swapping_two_runs_merges_with_a_concurrent_insert_without_duplicating() {
        let (base, target) = swap_document();
        let heads: hashbrown::HashSet<Dot> = base.graph().current_heads().copied().collect();
        let server = edit(base.clone(), &from_xml(&target).unwrap())
            .unwrap()
            .state;
        let server_css = server.graph().local_changesets_since(&heads).unwrap();

        let peer = State::from_changesets(base.graph().changesets_as_vec(), None).unwrap();
        let root = peer.view().root().unwrap().id();
        let mut tr = editor_transaction::Transaction::new(&peer);
        tr.insert_subtree(root, 5, paragraph_subtree("PEER"))
            .unwrap();
        let peer = tr.commit().0;
        let peer_css = peer.graph().local_changesets_since(&heads).unwrap();

        let mut merged = peer.graph().clone();
        for cs in server_css {
            merged = merged.receive_changeset(cs).unwrap();
        }
        let mut other = server.graph().clone();
        for cs in peer_css {
            other = other.receive_changeset(cs).unwrap();
        }
        let a = paragraph_texts(&State::from_changesets(merged.changesets_as_vec(), None).unwrap());
        let b = paragraph_texts(&State::from_changesets(other.changesets_as_vec(), None).unwrap());
        assert_eq!(a, b, "both merge orders agree");
        for label in swapped_expected() {
            let n = a.iter().filter(|t| **t == label).count();
            assert_eq!(n, 1, "{label} appears {n} times in {a:?}");
        }
        assert_eq!(a.iter().filter(|t| **t == "PEER").count(), 1);
    }

    fn rebuilt(state: &State) -> State {
        State::from_changesets(state.graph().changesets_as_vec(), None).unwrap()
    }

    fn assert_each_once(texts: &[String], labels: &[String]) {
        for label in labels {
            let n = texts.iter().filter(|t| *t == label).count();
            assert_eq!(n, 1, "{label} appears {n} times in {texts:?}");
        }
    }

    #[test]
    fn swapping_two_runs_survives_a_replay_from_changesets() {
        let (state, target) = swap_document();
        let out = edit(state, &from_xml(&target).unwrap()).unwrap();
        let expected = swapped_expected();
        assert_eq!(paragraph_texts(&out.state), expected, "transaction state");
        let replayed = paragraph_texts(&rebuilt(&out.state));
        assert_eq!(replayed, expected, "replayed from changesets");
    }

    #[test]
    fn swapping_two_runs_authored_by_two_actors_survives_a_replay() {
        let (seed, _z) = state! {
            doc { root { z: paragraph { text("Z") } } }
            selection: (z, 0)
        };
        let heads: hashbrown::HashSet<Dot> = seed.graph().current_heads().copied().collect();
        let root = seed.view().root().unwrap().id();

        let actor_a = State::from_changesets(seed.graph().changesets_as_vec(), None).unwrap();
        let mut tr = editor_transaction::Transaction::new(&actor_a);
        for i in 1..=16 {
            tr.insert_subtree(root, i - 1, paragraph_subtree(&format!("A{i:02}")))
                .unwrap();
        }
        let actor_a = tr.commit().0;
        let css_a = actor_a.graph().local_changesets_since(&heads).unwrap();

        let actor_b = State::from_changesets(seed.graph().changesets_as_vec(), None).unwrap();
        let mut tr = editor_transaction::Transaction::new(&actor_b);
        for i in 1..=16 {
            tr.insert_subtree(root, i - 1, paragraph_subtree(&format!("B{i:02}")))
                .unwrap();
        }
        let actor_b = tr.commit().0;
        let css_b = actor_b.graph().local_changesets_since(&heads).unwrap();

        let mut graph = seed.graph().clone();
        for cs in css_a.into_iter().chain(css_b) {
            graph = graph.receive_changeset(cs).unwrap();
        }
        let base = State::from_changesets(graph.changesets_as_vec(), None).unwrap();
        let texts = paragraph_texts(&base);
        assert_eq!(texts.len(), 33, "{texts:?}");

        let xml = to_xml(&base, &live_heads(&base)).unwrap();
        let lines: Vec<&str> = xml.lines().collect();
        assert_eq!(lines.len(), 35);
        let mut swapped: Vec<&str> = vec![lines[0]];
        swapped.extend_from_slice(&lines[17..33]);
        swapped.extend_from_slice(&lines[1..17]);
        swapped.extend_from_slice(&lines[33..]);
        let target = swapped.join("\n") + "\n";
        let want: Vec<String> = lines[17..33]
            .iter()
            .chain(lines[1..17].iter())
            .chain(lines[33..34].iter())
            .map(|l| {
                let s = l.find('>').unwrap() + 1;
                let e = l.rfind("</").unwrap();
                l[s..e].to_string()
            })
            .collect();

        let out = edit(base, &from_xml(&target).unwrap()).unwrap();
        assert_eq!(paragraph_texts(&out.state), want, "transaction state");
        let replayed = paragraph_texts(&rebuilt(&out.state));
        assert_each_once(&replayed, &want);
        assert_eq!(replayed, want, "replayed from changesets");
    }

    #[test]
    fn swapping_two_runs_twice_survives_a_replay() {
        let (state, target) = swap_document();
        let once = edit(state, &from_xml(&target).unwrap()).unwrap().state;
        let once = rebuilt(&once);
        let xml = to_xml(&once, &live_heads(&once)).unwrap();
        let lines: Vec<&str> = xml.lines().collect();
        assert_eq!(lines.len(), 35);
        let mut swapped: Vec<&str> = vec![lines[0]];
        swapped.extend_from_slice(&lines[17..33]);
        swapped.extend_from_slice(&lines[1..17]);
        swapped.extend_from_slice(&lines[33..]);
        let target2 = swapped.join("\n") + "\n";
        let out = edit(once, &from_xml(&target2).unwrap()).unwrap();
        let expected: Vec<String> = (1..=16)
            .map(|i| format!("A{i:02}"))
            .chain((1..=16).map(|i| format!("B{i:02}")))
            .chain(std::iter::once("Z".to_string()))
            .collect();
        assert_eq!(paragraph_texts(&out.state), expected, "transaction state");
        let replayed = paragraph_texts(&rebuilt(&out.state));
        assert_each_once(&replayed, &expected);
        assert_eq!(replayed, expected, "replayed from changesets");
    }
}
