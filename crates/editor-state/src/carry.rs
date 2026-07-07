use editor_crdt::Dot;
use editor_model::{ChildView, DocView, ModifierType, NodeType, Schema};

use crate::position::Position;
use crate::selection::ResolvedSelection;
use crate::traversal::{blocks_in_range, leaves_in_block_range};

const CHARLIKE_SLOTS: [NodeType; 3] = [NodeType::Text, NodeType::Tab, NodeType::HardBreak];

pub fn block_accepts_carry_kind(view: &DocView, block: Dot, ty: ModifierType) -> bool {
    let Some(node) = view.node(block) else {
        return false;
    };
    if !node.spec().is_textblock() {
        return false;
    }
    let mut base: Vec<NodeType> = node.ancestors().map(|a| a.node_type()).collect();
    base.reverse();
    let target = &Schema::modifier_spec(ty).target;
    CHARLIKE_SLOTS.iter().any(|&slot| {
        let mut path = base.clone();
        path.push(slot);
        target.matches(&path)
    })
}

pub fn end_touched_textblocks(view: &DocView, rs: &ResolvedSelection) -> Vec<Dot> {
    if let Some(rect) = rs.as_cell_rect() {
        let mut out = Vec::new();
        for cell in rect.cells() {
            for d in cell.descendants() {
                if let ChildView::Block(b) = d
                    && b.spec().is_textblock()
                {
                    out.push(b.id());
                }
            }
        }
        return out;
    }

    let mut out = Vec::new();
    let to_path = rs.to().path();
    for block in blocks_in_range(rs) {
        if !block.spec().is_textblock() {
            continue;
        }
        let last_charlike = block
            .children()
            .enumerate()
            .filter_map(|(i, c)| match c {
                ChildView::Leaf(l) if l.is_charlike() => Some(i),
                _ => None,
            })
            .last();
        match last_charlike {
            Some(slot) => {
                if leaves_in_block_range(rs, &block)
                    .iter()
                    .any(|(s, _)| *s == slot)
                {
                    out.push(block.id());
                }
            }
            None => {
                if let Some(start) = Position::new(block.id(), 0).resolve(view)
                    && to_path > start.path()
                {
                    out.push(block.id());
                }
            }
        }
    }
    out
}
