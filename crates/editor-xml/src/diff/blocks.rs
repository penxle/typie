use std::borrow::Cow;
use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{
    ChildView, Modifier, ModifierType, NodeType, PlainNode, PlainNodeEntry, PlainTextNode, Subtree,
};
use editor_transaction::StepError;

use super::Diff;
use crate::error::{XmlError, XmlErrorDetail};
use crate::lcs::lcs_pairs_bounded;
use crate::names::{element_name, is_block_atom, is_textblock, writable_modifiers};
use crate::tree::{InlineLeaf, XmlChild, XmlNode};

pub(crate) fn reconcile_shape(
    diff: &mut Diff<'_>,
    dot: Dot,
    target: &XmlNode,
    path: &[NodeType],
) -> Result<NodeType, XmlError> {
    let target_type = target.node.as_type();
    let base_type = diff.node_type_of(dot)?;
    let mut updated = false;

    if base_type != target_type {
        if is_block_atom(base_type)
            || is_block_atom(target_type)
            || diff.tr.view().leaf(dot).is_some()
        {
            return Err(XmlError::at(
                target.pos,
                XmlErrorDetail::DotTypeIncompatible {
                    dot: dot.to_string(),
                    new_type: type_name(target_type),
                },
            )
            .with_dot(dot));
        }
        if !is_replaceable(base_type) || !is_replaceable(target_type) {
            return Err(XmlError::internal(format!(
                "block type <{}> cannot be replaced with <{}>",
                type_name(base_type),
                type_name(target_type)
            )));
        }
        diff.tr
            .replace_block_type(dot, target_type)
            .map_err(|e| match e {
                StepError::IncompatibleBlockTypeReplacement { .. } => XmlError::at(
                    target.pos,
                    XmlErrorDetail::DotTypeIncompatible {
                        dot: dot.to_string(),
                        new_type: type_name(target_type),
                    },
                )
                .with_dot(dot),
                other => XmlError::internal(format!("replace block type: {other}")),
            })?;
        updated = true;
    }

    let dot = diff.resolve(dot);
    let atom = diff.tr.view().leaf(dot).is_some();

    let base_plain = current_node(diff, dot)?;
    if base_plain != target.node {
        let base_attrs = base_plain.to_attrs();
        for attr in target.node.to_attrs() {
            let same = base_attrs.iter().find(|b| b.same_field(&attr));
            if same != Some(&attr) {
                diff.tr
                    .set_node_attr(dot, attr)
                    .map_err(|e| XmlError::internal(format!("set node attr: {e}")))?;
                updated = true;
            }
        }
    }

    let base_mods = if atom {
        writable_own_modifiers(diff, dot, path)
    } else {
        writable_block_modifiers(diff, dot, path)
    };
    if base_mods != target.modifiers {
        for (ty, m) in &base_mods {
            if target.modifiers.get(ty) != Some(m) {
                if atom {
                    diff.tr.remove_span_modifier(dot, dot, m.clone())
                } else {
                    diff.tr.remove_modifier(dot, m.clone())
                }
                .map_err(|e| XmlError::internal(format!("remove modifier: {e}")))?;
            }
        }
        for (ty, m) in &target.modifiers {
            if base_mods.get(ty) != Some(m) {
                if atom {
                    diff.tr.add_span_modifier(dot, dot, m.clone())
                } else {
                    diff.tr.add_modifier(dot, m.clone())
                }
                .map_err(|e| XmlError::internal(format!("add modifier: {e}")))?;
            }
        }
        updated = true;
    }

    if is_textblock(target_type) {
        let base_carry = diff.tr.state().projected.projected().carry_modifiers(dot);
        if base_carry != target.carry {
            diff.tr
                .replace_carry(dot, target.carry.values().cloned().collect())
                .map_err(|e| XmlError::internal(format!("replace carry: {e}")))?;
            updated = true;
        }
    }

    if updated {
        diff.counts.blocks_updated += 1;
    }
    Ok(target_type)
}

pub(crate) fn reconcile_container(
    diff: &mut Diff<'_>,
    container: Dot,
    target: &XmlNode,
    path: &[NodeType],
) -> Result<(), XmlError> {
    let container = diff.resolve(container);

    for child in current_children(diff, container) {
        // A scaffold the target does not name is projection-owned, not authored
        // content: it disappears on its own once real content fills the slot.
        if !child.is_synthetic() && diff.target_identity(child).is_none() {
            diff.doom(child);
        }
    }

    let target_children: Vec<&XmlNode> = target.block_children().collect();
    let target_order: Vec<Dot> = target_children.iter().filter_map(|c| c.dot).collect();
    let base_order: Vec<Dot> = current_children(diff, container)
        .into_iter()
        .filter_map(|d| diff.target_identity(d))
        .filter(|d| target_order.contains(d))
        .collect();
    let anchors = block_anchors(&base_order, &target_order, diff.block_lcs_bound);

    let mut settled: Vec<Dot> = Vec::new();
    let mut placed: Vec<(Dot, Cow<'_, XmlNode>)> = Vec::new();
    for child in &target_children {
        let mut dot = child.dot;
        if let Some(d) = dot
            && diff.resolve(d).is_synthetic()
        {
            dot = match scaffold_plan(diff, d, child, path) {
                Scaffold::Skip => {
                    diff.skip_vanished(d, child)?;
                    continue;
                }
                Scaffold::Insert => None,
                Scaffold::Materialize => Some(diff.ensure_real(d)?),
                Scaffold::Leave => {
                    diff.keep_scaffold(d, child);
                    Some(d)
                }
            };
        }
        match dot {
            Some(d) if anchors.contains(&d) => {
                let d = diff.resolve(d);
                settled.push(d);
                placed.push((d, Cow::Borrowed(*child)));
            }
            Some(d) => {
                let d = diff.ensure_real(d)?;
                let index = slot_after(diff, container, &settled)?;
                let current = position_of(diff, d);
                if !already_in_place(current, container, index) {
                    let moved = diff
                        .tr
                        .move_node(d, container, move_index(current, container, index))
                        .map_err(|e| XmlError::internal(format!("move node: {e}")))?;
                    diff.record_moved(&moved);
                    diff.counts.blocks_moved += 1;
                }
                let d = diff.resolve(d);
                settled.push(d);
                placed.push((d, Cow::Borrowed(*child)));
            }
            None if holds_dotted_block(child) => {
                let index = slot_after(diff, container, &settled)?;
                let new = insert_new(diff, container, index, &without_dotted_blocks(child))?;
                let mut adopted = (*child).clone();
                adopt_insert(diff, new, &mut adopted)?;
                settled.push(new);
                placed.push((new, Cow::Owned(adopted)));
            }
            None => {
                let index = slot_after(diff, container, &settled)?;
                let new = insert_new(diff, container, index, child)?;
                settled.push(new);
            }
        }
    }

    for (d, child) in placed {
        diff.reconcile_node(d, &child, path)?;
    }
    Ok(())
}

fn insert_new(
    diff: &mut Diff<'_>,
    container: Dot,
    index: usize,
    node: &XmlNode,
) -> Result<Dot, XmlError> {
    let new = diff
        .tr
        .insert_subtree(container, index, subtree_of(node))
        .map_err(|e| XmlError::internal(format!("insert subtree: {e}")))?
        .ok_or_else(|| XmlError::internal("inserted block has no dot"))?;
    diff.counts.blocks_inserted += 1;
    diff.counts.chars_inserted += count_chars(node);
    Ok(new)
}

/// A target child the file gives no dot but that holds dots below it: a new
/// container wrapped around blocks the document already has. Inserting its
/// subtree whole would copy those blocks and leave the originals behind, so
/// the insert carries only what is new.
fn holds_dotted_block(node: &XmlNode) -> bool {
    node.block_children()
        .any(|child| child.dot.is_some() || holds_dotted_block(child))
}

fn without_dotted_blocks(node: &XmlNode) -> XmlNode {
    let mut out = node.clone();
    out.children = node
        .children
        .iter()
        .filter_map(|child| match child {
            XmlChild::Block(block) if block.dot.is_some() => None,
            XmlChild::Block(block) => Some(XmlChild::Block(without_dotted_blocks(block))),
            XmlChild::Inline(_) => Some(child.clone()),
        })
        .collect();
    out
}

/// Names the blocks the insert just minted, container and carried descendants
/// alike, so the reconcile that follows reads them as blocks the target
/// already names and moves the dotted blocks the insert left out into the
/// holes they left. The insert emits every carried block and nothing else, so
/// the minted blocks pair in order with the dot-less children; a scaffold the
/// projection added to complete a hole is not one of them. A textblock stops
/// the walk: the file gives its inline atoms no dots, so there is nothing
/// below it to name.
fn adopt_insert(diff: &mut Diff<'_>, dot: Dot, target: &mut XmlNode) -> Result<(), XmlError> {
    target.dot = Some(dot);
    diff.target_dots.insert(dot);
    if is_textblock(target.node.as_type()) {
        return Ok(());
    }
    let mut minted = current_children(diff, dot)
        .into_iter()
        .filter(|d| !d.is_synthetic());
    for child in &mut target.children {
        let XmlChild::Block(block) = child else {
            continue;
        };
        if block.dot.is_some() {
            continue;
        }
        let next = minted
            .next()
            .ok_or_else(|| XmlError::internal("inserted container is missing a child"))?;
        adopt_insert(diff, next, block)?;
    }
    match minted.next() {
        Some(_) => Err(XmlError::internal("inserted container has an extra child")),
        None => Ok(()),
    }
}

/// The base children that keep their place. Past the bound the search gives up
/// and nothing is anchored, so every child is placed by a move.
fn block_anchors(base_order: &[Dot], target_order: &[Dot], max_d: usize) -> Vec<Dot> {
    lcs_pairs_bounded(base_order, target_order, max_d)
        .map(|pairs| pairs.into_iter().map(|(i, _)| base_order[i]).collect())
        .unwrap_or_default()
}

/// What to do with a target child whose dot names a projection-owned scaffold.
enum Scaffold {
    /// Already gone and carrying nothing: leave it out of the document.
    Skip,
    /// Already gone but the target wrote content into it: keep the content by
    /// creating a new block for it.
    Insert,
    /// Still here and changed: give it a real dot before anything can fill the
    /// slot it stands in, so the authored content cannot vanish with it.
    Materialize,
    /// Still here and unchanged: leave it projection-owned.
    Leave,
}

fn scaffold_plan(diff: &Diff<'_>, dot: Dot, target: &XmlNode, path: &[NodeType]) -> Scaffold {
    if diff.is_vanished_scaffold(dot) {
        return if carries_nothing(target) {
            Scaffold::Skip
        } else {
            Scaffold::Insert
        };
    }
    let mut here = path.to_vec();
    here.push(target.node.as_type());
    if diff.subtree_equals(diff.resolve(dot), target, &here) {
        Scaffold::Leave
    } else {
        Scaffold::Materialize
    }
}

pub(crate) fn carries_nothing(target: &XmlNode) -> bool {
    target.children.is_empty() && target.modifiers.is_empty() && target.carry.is_empty()
}

fn is_replaceable(t: NodeType) -> bool {
    !matches!(t, NodeType::Root | NodeType::Unknown)
}

fn type_name(t: NodeType) -> String {
    element_name(t).unwrap_or("?").to_string()
}

/// The node a dot stands for now, whether it projects as a block or as an atom
/// leaf.
fn current_node(diff: &Diff<'_>, dot: Dot) -> Result<PlainNode, XmlError> {
    let view = diff.tr.view();
    view.node(dot)
        .map(|nv| nv.node().to_plain())
        .or_else(|| view.leaf(dot).and_then(|l| l.node()).map(|n| n.to_plain()))
        .ok_or_else(|| XmlError::internal(format!("block not found: {dot}")))
}

/// An atom's modifiers live in its leaf span state — the store `to_plain` reads
/// for an atom. Block modifiers set on an atom dot never reach it.
fn writable_own_modifiers(
    diff: &Diff<'_>,
    dot: Dot,
    path: &[NodeType],
) -> BTreeMap<ModifierType, Modifier> {
    let view = diff.tr.view();
    let Some(state) = view.leaf_state_by_dot_slow(dot) else {
        return BTreeMap::new();
    };
    let own: BTreeMap<ModifierType, Modifier> = state
        .own
        .iter()
        .map(|(ty, o)| (*ty, o.value.clone()))
        .collect();
    writable_modifiers(&own, path)
}

fn writable_block_modifiers(
    diff: &Diff<'_>,
    dot: Dot,
    path: &[NodeType],
) -> BTreeMap<ModifierType, Modifier> {
    diff.tr
        .state()
        .projected
        .projected()
        .block_modifiers
        .get(&dot)
        .map(|mods| writable_modifiers(mods, path))
        .unwrap_or_default()
}

/// `index` is a slot of `container` as it stands now, the node itself included.
/// A same-parent `move_node` vacates the node's own slot before inserting, so
/// slot `i` and slot `i + 1` are the same place, and a target slot past `i` is
/// one lower in the index domain `move_node` reads.
fn already_in_place(current: Option<(Dot, usize)>, container: Dot, index: usize) -> bool {
    match current {
        Some((parent, i)) => parent == container && (i == index || i + 1 == index),
        None => false,
    }
}

fn move_index(current: Option<(Dot, usize)>, container: Dot, index: usize) -> usize {
    match current {
        Some((parent, i)) if parent == container && i < index => index - 1,
        _ => index,
    }
}

fn block_slots(diff: &Diff<'_>, container: Dot) -> Vec<(usize, Dot)> {
    let view = diff.tr.view();
    let Some(nv) = view.node(container) else {
        return Vec::new();
    };
    nv.children()
        .enumerate()
        .filter_map(|(slot, child)| match child {
            ChildView::Block(b) => Some((slot, b.id())),
            ChildView::Leaf(l) => l.node().is_some().then(|| (slot, l.dot())),
        })
        .collect()
}

pub(crate) fn current_children(diff: &Diff<'_>, container: Dot) -> Vec<Dot> {
    block_slots(diff, container)
        .into_iter()
        .map(|(_, dot)| dot)
        .collect()
}

fn position_of(diff: &Diff<'_>, dot: Dot) -> Option<(Dot, usize)> {
    let view = diff.tr.view();
    let parent = match view.node(dot) {
        Some(nv) => nv.parent()?,
        None => view.leaf(dot)?.parent()?,
    };
    let index = parent.children().position(|c| c.id() == dot)?;
    Some((parent.id(), index))
}

/// The slot right after the last settled child still present in `container`.
/// A settled scaffold may have stopped existing once a later child filled its
/// slot; that vanish is expected, so the search falls back to the child before
/// it. A real block that vanished is still an internal error.
fn slot_after(diff: &Diff<'_>, container: Dot, settled: &[Dot]) -> Result<usize, XmlError> {
    let slots = block_slots(diff, container);
    for last in settled.iter().rev() {
        let last = diff.resolve(*last);
        if let Some((slot, _)) = slots.iter().find(|(_, d)| *d == last) {
            return Ok(slot + 1);
        }
        if !last.is_synthetic() {
            return Err(XmlError::internal(format!(
                "settled block vanished: {last}"
            )));
        }
    }
    Ok(0)
}

pub(crate) fn subtree_of(node: &XmlNode) -> Subtree {
    let mut children: Vec<Subtree> = Vec::new();
    let mut run: Option<(String, BTreeMap<ModifierType, Modifier>)> = None;
    for child in &node.children {
        match child {
            XmlChild::Block(b) => {
                flush_run(&mut run, &mut children);
                children.push(subtree_of(b));
            }
            XmlChild::Inline(item) => match &item.leaf {
                InlineLeaf::Char(ch) => match &mut run {
                    Some((text, own)) if *own == item.own => text.push(*ch),
                    _ => {
                        flush_run(&mut run, &mut children);
                        run = Some((ch.to_string(), item.own.clone()));
                    }
                },
                InlineLeaf::Atom(atom) => {
                    flush_run(&mut run, &mut children);
                    children.push(Subtree {
                        node: atom.clone(),
                        modifiers: item.own.values().cloned().collect(),
                        carry: Vec::new(),
                        children: Vec::new(),
                        source_dots: Vec::new(),
                    });
                }
            },
        }
    }
    flush_run(&mut run, &mut children);
    Subtree {
        node: node.node.clone(),
        modifiers: node.modifiers.values().cloned().collect(),
        carry: node.carry.values().cloned().collect(),
        children,
        source_dots: Vec::new(),
    }
}

fn flush_run(
    run: &mut Option<(String, BTreeMap<ModifierType, Modifier>)>,
    children: &mut Vec<Subtree>,
) {
    if let Some((text, own)) = run.take() {
        children.push(Subtree {
            node: PlainNode::Text(PlainTextNode { text }),
            modifiers: own.into_values().collect(),
            carry: Vec::new(),
            children: Vec::new(),
            source_dots: Vec::new(),
        });
    }
}

pub(crate) fn count_plain_chars(entry: &PlainNodeEntry) -> u32 {
    let own = match &entry.node {
        PlainNode::Text(t) => t.text.chars().count() as u32,
        _ => 0,
    };
    own + entry.children.iter().map(count_plain_chars).sum::<u32>()
}

fn count_chars(node: &XmlNode) -> u32 {
    let own = node
        .inline_items()
        .filter(|i| matches!(i.leaf, InlineLeaf::Char(_)))
        .count() as u32;
    own + node.block_children().map(count_chars).sum::<u32>()
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_transaction::Transaction;

    use super::super::ChangeCounts;
    use super::*;
    use crate::lcs::MAX_EDIT_DISTANCE;
    use crate::reader::from_xml;
    use crate::test_support::live_heads;
    use crate::writer::to_xml;

    fn apply(
        state: &editor_state::State,
        xml: &str,
        probes: &[Dot],
    ) -> (editor_state::State, ChangeCounts, Vec<Dot>) {
        let tree = from_xml(xml).expect("target parses");
        let mut tr = Transaction::new(state);
        let root = state.view().root().expect("root").id();
        let (counts, resolved) = {
            let mut diff = Diff::new(&mut tr, &tree);
            diff.reconcile_node(root, &tree.root, &[])
                .expect("reconcile");
            let resolved: Vec<Dot> = probes.iter().map(|d| diff.resolve(*d)).collect();
            (diff.finish().expect("finish").counts, resolved)
        };
        let (next, ..) = tr.commit();
        (next, counts, resolved)
    }

    #[test]
    fn anchors_are_dropped_when_the_reorder_exceeds_the_bound() {
        let base: Vec<Dot> = (1..=4).map(|i| Dot::new(1, i)).collect();
        let reversed: Vec<Dot> = base.iter().rev().copied().collect();

        assert_eq!(block_anchors(&base, &reversed, MAX_EDIT_DISTANCE).len(), 1);
        assert!(block_anchors(&base, &reversed, 1).is_empty());
        assert_eq!(block_anchors(&base, &base, 0), base);
    }

    #[test]
    fn slot_arithmetic_of_an_anchored_move() {
        let container = Dot::new(1, 1);
        let other = Dot::new(1, 2);

        assert!(!already_in_place(Some((other, 0)), container, 0));
        assert!(already_in_place(Some((container, 2)), container, 2));
        assert!(already_in_place(Some((container, 2)), container, 3));
        assert!(!already_in_place(Some((container, 5)), container, 2));
        assert!(!already_in_place(None, container, 0));

        assert_eq!(move_index(Some((other, 0)), container, 2), 2);
        assert_eq!(move_index(Some((container, 5)), container, 2), 2);
        assert_eq!(move_index(Some((container, 0)), container, 2), 1);
        assert_eq!(move_index(None, container, 2), 2);
    }

    #[test]
    fn insert_delete_and_reorder_keep_anchor_dots() {
        let (state, a, b, c) = state! {
            doc { root { a: paragraph { text("A") } b: paragraph { text("B") } c: paragraph { text("C") } } }
            selection: (a, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let a_line = format!("  <paragraph dot=\"{a}\">A</paragraph>\n");
        let b_line = format!("  <paragraph dot=\"{b}\">B</paragraph>\n");
        let c_line = format!("  <paragraph dot=\"{c}\">C</paragraph>\n");
        let target = xml.replace(
            &format!("{a_line}{b_line}{c_line}"),
            &format!("{c_line}{a_line}  <paragraph>N</paragraph>\n"),
        );
        assert_ne!(target, xml, "the target must differ from the source");

        let (next, counts, resolved) = apply(&state, &target, &[a, b, c]);
        let (expected, ..) = state! {
            doc { root { paragraph { text("C") } paragraph { text("A") } paragraph { text("N") } } }
            selection: none
        };
        assert_eq!(next.to_plain(), expected.to_plain());
        assert_eq!(
            (
                counts.blocks_inserted,
                counts.blocks_deleted,
                counts.blocks_moved,
                counts.blocks_updated,
                counts.chars_inserted,
                counts.chars_deleted,
            ),
            (1, 1, 1, 0, 1, 1)
        );

        let out = to_xml(&next, &live_heads(&next)).unwrap();
        assert_eq!(resolved[2], c, "the anchor keeps its dot");
        assert!(out.contains(&format!("dot=\"{c}\"")));
        assert_ne!(
            resolved[0], a,
            "a move re-publishes the subtree under new dots"
        );
        assert!(out.contains(&format!("dot=\"{}\"", resolved[0])));
        assert!(!out.contains(&format!("dot=\"{a}\"")));
        assert!(!out.contains(&format!("dot=\"{b}\"")));
    }

    #[test]
    fn attrs_modifiers_carry_and_type_change_in_place() {
        let (state, p) = state! {
            doc { root { blockquote(variant: BlockquoteVariant::LeftLine) { p: paragraph { text("q") } } } }
            selection: (p, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = xml
            .replace("<blockquote", "<callout")
            .replace("</blockquote>", "</callout>")
            .replace("attr:variant=\"left_line\"", "attr:variant=\"warning\"")
            .replace(
                &format!("<paragraph dot=\"{p}\">"),
                &format!("<paragraph dot=\"{p}\" mod:alignment=\"right\" carry:bold=\"\">"),
            );

        let (next, counts, resolved) = apply(&state, &target, &[p]);
        let (expected, ..) = state! {
            doc { root { callout(variant: CalloutVariant::Warning) { paragraph [alignment(Alignment::Right)] carry([bold]) { text("q") } } } }
            selection: none
        };
        assert_eq!(next.to_plain(), expected.to_plain());
        assert_eq!(
            (
                counts.blocks_inserted,
                counts.blocks_deleted,
                counts.blocks_moved,
                counts.blocks_updated,
            ),
            (0, 0, 0, 2)
        );

        let out = to_xml(&next, &live_heads(&next)).unwrap();
        assert_ne!(
            resolved[0], p,
            "a type change re-publishes the whole subtree, children included"
        );
        assert!(out.contains(&format!("dot=\"{}\"", resolved[0])));
        assert!(!out.contains(&format!("dot=\"{p}\"")));
    }

    #[test]
    fn writable_modifiers_drops_invalid_valueless_and_out_of_context_kinds() {
        let mods = BTreeMap::from([
            (ModifierType::Bold, Modifier::Bold),
            (ModifierType::FontSize, Modifier::FontSize { value: 99 }),
            (
                ModifierType::FontWeight,
                Modifier::FontWeight { value: 700 },
            ),
        ]);
        assert!(
            Modifier::Bold.is_valid(),
            "Bold is valid but carries no value"
        );
        assert!(!Modifier::FontSize { value: 99 }.is_valid());
        assert_eq!(
            writable_modifiers(&mods, &[NodeType::Root]),
            BTreeMap::from([(
                ModifierType::FontWeight,
                Modifier::FontWeight { value: 700 }
            )])
        );

        let line_height = BTreeMap::from([(
            ModifierType::LineHeight,
            Modifier::LineHeight { value: 160 },
        )]);
        assert_eq!(
            writable_modifiers(&line_height, &[NodeType::Root, NodeType::Paragraph]),
            line_height
        );
        assert_eq!(
            writable_modifiers(&line_height, &[NodeType::Root, NodeType::Archived]),
            BTreeMap::new(),
            "an opaque element takes no block modifier the schema does not place there"
        );
    }

    #[test]
    fn a_scaffold_is_materialized_before_it_is_changed() {
        let (state, ..) = state! {
            doc { root { fold paragraph {} } }
            selection: none
        };
        let title = {
            let view = state.view();
            view.root()
                .expect("root")
                .child_blocks()
                .find(|b| b.node_type() == NodeType::Fold)
                .expect("fold")
                .fold_title()
                .expect("fold title")
                .id()
        };
        assert!(title.is_synthetic());

        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let target = xml.replace(
            &format!("<fold_title dot=\"{title}\">"),
            &format!("<fold_title dot=\"{title}\" carry:bold=\"\">"),
        );
        assert_ne!(target, xml, "the target must differ from the source");

        let (next, counts, resolved) = apply(&state, &target, &[title]);
        assert_ne!(resolved[0], title);
        assert!(!resolved[0].is_synthetic());
        assert_eq!(counts.blocks_updated, 1);
        assert_eq!(
            next.projected
                .projected()
                .carry_modifiers(resolved[0])
                .get(&ModifierType::Bold),
            Some(&Modifier::Bold)
        );
    }

    #[test]
    fn an_unchanged_opaque_block_keeps_its_hidden_children() {
        use editor_crdt::ListOp;
        use editor_model::{EditOp, SeqItem};

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
        let before = editor_state::to_plain_subtree(&state, unknown).expect("opaque subtree");
        assert_eq!(before.children.len(), 1);

        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        assert!(xml.contains(&format!("<unknown dot=\"{unknown}\"/>")));

        let (next, counts, ..) = apply(&state, &xml, &[]);
        assert_eq!(counts, ChangeCounts::default());
        assert_eq!(
            editor_state::to_plain_subtree(&next, unknown),
            Some(before),
            "an opaque block the target repeats verbatim is never touched"
        );
    }

    #[test]
    fn move_across_containers_preserves_identity() {
        let (state, p, q) = state! {
            doc { root { blockquote { p: paragraph { text("p") } } q: paragraph { text("q") } } }
            selection: (p, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let p_line = format!("    <paragraph dot=\"{p}\">p</paragraph>\n");
        let q_line = format!("  <paragraph dot=\"{q}\">q</paragraph>\n");
        let target = xml.replace(&p_line, "").replace(
            &q_line,
            &format!("{q_line}  <paragraph dot=\"{p}\">p</paragraph>\n"),
        );
        let target = target.replace(
            "attr:variant=\"left_line\">\n  </blockquote>",
            "attr:variant=\"left_line\"><paragraph>x</paragraph></blockquote>",
        );

        let (next, counts, resolved) = apply(&state, &target, &[p, q]);
        assert_eq!(
            (
                counts.blocks_inserted,
                counts.blocks_deleted,
                counts.blocks_moved,
                counts.blocks_updated,
                counts.chars_inserted,
                counts.chars_deleted,
            ),
            (1, 0, 1, 0, 1, 0)
        );

        let plain = next.to_plain();
        assert_eq!(plain.root.children.len(), 3);
        assert_eq!(plain.root.children[2].children[0].node, text_node("p"));

        let out = to_xml(&next, &live_heads(&next)).unwrap();
        assert_eq!(resolved[1], q, "the untouched sibling keeps its dot");
        assert_ne!(resolved[0], p);
        assert!(out.contains(&format!("<paragraph dot=\"{}\">p</paragraph>", resolved[0])));
    }

    fn text_node(text: &str) -> PlainNode {
        PlainNode::Text(PlainTextNode {
            text: text.to_string(),
        })
    }
}
