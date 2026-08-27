use std::collections::{HashMap, HashSet};

use editor_crdt::Dot;
use editor_model::NodeType;
use editor_transaction::{MovedNode, Transaction};

use crate::error::XmlError;
use crate::tree::{XmlNode, XmlTree};

pub mod blocks;
pub mod inline;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChangeCounts {
    pub blocks_inserted: u32,
    pub blocks_deleted: u32,
    pub blocks_moved: u32,
    pub blocks_updated: u32,
    pub chars_inserted: u32,
    pub chars_deleted: u32,
}

pub struct Diff<'a> {
    pub(crate) tr: &'a mut Transaction,
    pub(crate) target_dots: HashSet<Dot>,
    remap: HashMap<Dot, Dot>,
    vanished: HashSet<Dot>,
    doomed: Vec<Dot>,
    kept_scaffolds: Vec<Dot>,
    pub(crate) block_lcs_bound: usize,
    pub(crate) counts: ChangeCounts,
}

/// What `finish` hands back: the counts the reconcile accumulated, and the
/// target dots it skipped as vanished scaffolds — the target describes them,
/// the document cannot hold them, and the post-condition must read the target
/// without them.
pub struct DiffOutcome {
    pub counts: ChangeCounts,
    pub vanished: HashSet<Dot>,
}

impl<'a> Diff<'a> {
    pub fn new(tr: &'a mut Transaction, target: &XmlTree) -> Self {
        Self::bounded(tr, target, crate::lcs::MAX_EDIT_DISTANCE)
    }

    /// As [`Diff::new`], with the bound on the block reorder search the
    /// anchors come from. Past it nothing is anchored and every child is
    /// placed by a move.
    pub fn bounded(tr: &'a mut Transaction, target: &XmlTree, block_lcs_bound: usize) -> Self {
        let mut target_dots = HashSet::new();
        collect_dots(&target.root, &mut target_dots);
        Self {
            tr,
            target_dots,
            remap: HashMap::new(),
            vanished: HashSet::new(),
            doomed: Vec::new(),
            kept_scaffolds: Vec::new(),
            block_lcs_bound,
            counts: ChangeCounts::default(),
        }
    }

    pub fn resolve(&self, dot: Dot) -> Dot {
        let mut d = dot;
        let mut hops = 0;
        while let Some(next) = self.remap.get(&d) {
            d = *next;
            hops += 1;
            if hops > 64 {
                break;
            }
        }
        let view = self.tr.view();
        view.alias_classes()
            .resolve_with(d, |x| view.node(x).is_some() || view.leaf(x).is_some())
    }

    /// The target-space dot `current` stands for — itself, or the member of its
    /// alias class the target names.
    pub(crate) fn target_identity(&self, current: Dot) -> Option<Dot> {
        if self.target_dots.contains(&current) {
            return Some(current);
        }
        let view = self.tr.view();
        let members = view.alias_classes().members_of(current)?;
        members
            .iter()
            .copied()
            .find(|m| self.target_dots.contains(m))
    }

    pub(crate) fn record_moved(&mut self, moved: &MovedNode) {
        for (old, new) in &moved.pairs {
            if old != new {
                self.remap.insert(*old, *new);
            }
        }
    }

    /// A projection-owned scaffold the target still names but the document no
    /// longer holds: filling the slot it stood in makes it stop existing, so a
    /// target that names it describes a node that is already gone.
    pub(crate) fn is_vanished_scaffold(&self, dot: Dot) -> bool {
        let resolved = self.resolve(dot);
        if !resolved.is_synthetic() {
            return false;
        }
        let view = self.tr.view();
        view.node(resolved).is_none() && view.leaf(resolved).is_none()
    }

    /// Records that the target names `dot` but the reconcile left it out. The
    /// post-condition then reads the target without it, so only a scaffold
    /// whose target carries nothing may go this way — anything else would be
    /// dropped from the document and from the oracle at once.
    pub(crate) fn skip_vanished(&mut self, dot: Dot, target: &XmlNode) -> Result<(), XmlError> {
        if !blocks::carries_nothing(target) {
            return Err(XmlError::internal("vanished scaffold carries content"));
        }
        self.vanished.insert(dot);
        Ok(())
    }

    /// Marks a base child the target names nowhere. The removal waits for
    /// `finish`: a descendant the target names somewhere else is a move, and
    /// removing the container now would take it along.
    pub(crate) fn doom(&mut self, dot: Dot) {
        self.doomed.push(dot);
    }

    /// Watches a scaffold the reconcile left projection-owned: a removal in
    /// `finish` can still fill its slot and make it stop existing. Only a
    /// scaffold whose target carries nothing is watched, since `finish` skips
    /// what is gone by then.
    pub(crate) fn keep_scaffold(&mut self, dot: Dot, target: &XmlNode) {
        if blocks::carries_nothing(target) {
            self.kept_scaffolds.push(dot);
        }
    }

    /// Closes the reconcile: removes what it doomed, outermost first, and
    /// counts what is left inside — a descendant that moved out is gone by now
    /// and is no longer a loss — then re-reads the scaffolds it left, since a
    /// removal can be what finally fills their slot.
    pub fn finish(mut self) -> Result<DiffOutcome, XmlError> {
        for dot in std::mem::take(&mut self.doomed) {
            let dot = self.resolve(dot);
            let present = {
                let view = self.tr.view();
                view.node(dot).is_some() || view.leaf(dot).is_some()
            };
            if !present {
                continue;
            }
            let lost = editor_state::to_plain_subtree(self.tr.state(), dot)
                .map(|entry| blocks::count_plain_chars(&entry))
                .unwrap_or_default();
            self.tr
                .remove_subtree(dot)
                .map_err(|e| XmlError::internal(format!("remove subtree: {e}")))?;
            self.counts.blocks_deleted += 1;
            self.counts.chars_deleted += lost;
        }
        for dot in std::mem::take(&mut self.kept_scaffolds) {
            if self.is_vanished_scaffold(dot) {
                self.vanished.insert(dot);
            }
        }
        Ok(DiffOutcome {
            counts: self.counts,
            vanished: self.vanished,
        })
    }

    pub(crate) fn ensure_real(&mut self, dot: Dot) -> Result<Dot, XmlError> {
        let dot = self.resolve(dot);
        if !dot.is_synthetic() {
            return Ok(dot);
        }
        let real = editor_commands::materialize_block(self.tr, dot)
            .map_err(|e| XmlError::internal(format!("materialize block {dot}: {e}")))?;
        if real != dot {
            self.remap.insert(dot, real);
        }
        Ok(real)
    }

    /// `path` runs from the root down to `target`'s parent — the schema reads
    /// it to tell which modifiers may sit on the node the file describes.
    pub fn reconcile_node(
        &mut self,
        dot: Dot,
        target: &XmlNode,
        path: &[NodeType],
    ) -> Result<(), XmlError> {
        if self.is_vanished_scaffold(dot) {
            return self.skip_vanished(dot, target);
        }
        let mut here = path.to_vec();
        here.push(target.node.as_type());
        let dot = self.resolve(dot);
        if self.subtree_equals(dot, target, &here) && self.named_dots_in_place(dot, target) {
            return Ok(());
        }
        let dot = self.ensure_real(dot)?;
        let node_type = blocks::reconcile_shape(self, dot, target, &here)?;
        if crate::names::is_opaque(node_type) || crate::names::is_block_atom(node_type) {
            return Ok(());
        }
        let dot = self.resolve(dot);
        if crate::names::is_textblock(node_type) {
            inline::reconcile_textblock(self, dot, target)
        } else {
            blocks::reconcile_container(self, dot, target, &here)
        }
    }

    /// Every dot the target names inside this subtree already stands where the
    /// target puts it. Reading the same content is not enough to leave a
    /// subtree alone: a scaffold reads exactly like the block the target names,
    /// so a target that names a block parked somewhere else reads as equal
    /// while a move is still owed. Textblocks and leaf-content blocks answer
    /// yes — the file gives their children no dots to name.
    fn named_dots_in_place(&self, dot: Dot, target: &XmlNode) -> bool {
        let node_type = target.node.as_type();
        if crate::names::is_textblock(node_type)
            || crate::names::is_opaque(node_type)
            || crate::names::is_block_atom(node_type)
        {
            return true;
        }
        let mut base = blocks::current_children(self, dot).into_iter();
        for child in target.block_children() {
            let Some(b) = base.next() else {
                return false;
            };
            if child.dot.is_some_and(|d| self.resolve(d) != b) {
                return false;
            }
            if !self.named_dots_in_place(b, child) {
                return false;
            }
        }
        base.next().is_none()
    }

    fn subtree_equals(&self, dot: Dot, target: &XmlNode, path: &[NodeType]) -> bool {
        let Some(base) = editor_state::to_plain_subtree(self.tr.state(), dot) else {
            return false;
        };
        let wanted = target.to_plain_entry();
        // An opaque block's children never reach the file, so the target can
        // never describe them and they can never be a difference; neither can a
        // modifier the file cannot carry.
        if crate::names::is_opaque(base.node.as_type())
            || crate::names::is_opaque(wanted.node.as_type())
        {
            return base.node == wanted.node
                && crate::names::writable_modifiers(&base.modifiers, path) == wanted.modifiers
                && base.carry == wanted.carry;
        }
        base == wanted
    }

    pub(crate) fn node_type_of(&self, dot: Dot) -> Result<NodeType, XmlError> {
        let view = self.tr.view();
        view.node(dot)
            .map(|n| n.node_type())
            .or_else(|| view.leaf(dot).map(|l| l.node_type()))
            .ok_or_else(|| XmlError::internal(format!("block not found: {dot}")))
    }
}

fn collect_dots(node: &XmlNode, out: &mut HashSet<Dot>) {
    if let Some(d) = node.dot {
        out.insert(d);
    }
    for child in node.block_children() {
        collect_dots(child, out);
    }
}
