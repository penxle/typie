use std::sync::Arc;

use editor_crdt::Dot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageBreakPolicy {
    #[default]
    Auto,
    Avoid,
}
use crate::measure::text::measure::MeasuredLine;
use crate::style::BoxStyle;

#[derive(Debug)]
pub(crate) struct MeasuredTree {
    pub root: MeasuredNode,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MeasuredChildren {
    tree: editor_common::SumTree<Arc<MeasuredNode>, f32>,
}

impl MeasuredChildren {
    pub fn from_blocks(blocks: Vec<Arc<MeasuredNode>>) -> Self {
        let items = blocks
            .into_iter()
            .map(|b| {
                let height = b.height;
                (b, height)
            })
            .collect();
        Self {
            tree: editor_common::SumTree::from_items(items),
        }
    }

    pub fn total_height(&self) -> f32 {
        self.tree.total_size()
    }
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.tree.len()
    }
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }
    #[cfg(test)]
    pub fn get(&self, index: usize) -> Option<&Arc<MeasuredNode>> {
        self.tree.get(index)
    }
    pub fn iter(&self) -> editor_common::Iter<'_, Arc<MeasuredNode>, f32> {
        self.tree.iter()
    }
    pub fn set(&mut self, index: usize, node: Arc<MeasuredNode>) -> bool {
        let height = node.height;
        self.tree.set(index, node, height)
    }
}

impl std::ops::Index<usize> for MeasuredChildren {
    type Output = Arc<MeasuredNode>;
    fn index(&self, index: usize) -> &Arc<MeasuredNode> {
        self.tree
            .get(index)
            .expect("MeasuredChildren: index out of bounds")
    }
}

impl<'a> IntoIterator for &'a MeasuredChildren {
    type Item = &'a Arc<MeasuredNode>;
    type IntoIter = editor_common::Iter<'a, Arc<MeasuredNode>, f32>;
    fn into_iter(self) -> Self::IntoIter {
        self.tree.iter()
    }
}

impl FromIterator<Arc<MeasuredNode>> for MeasuredChildren {
    fn from_iter<I: IntoIterator<Item = Arc<MeasuredNode>>>(iter: I) -> Self {
        Self::from_blocks(iter.into_iter().collect())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MeasuredNode {
    pub width: f32,
    pub height: f32,
    pub content: MeasuredContent,
}

#[derive(Debug, Clone)]
pub(crate) enum MeasuredContent {
    Box(MeasuredBox),
    Line(Arc<MeasuredLine>),
    Atom(MeasuredAtom),
    Spacing(f32),
    PageBreak,
}

impl MeasuredNode {
    /// The sole line-wrap path (H1): `height` is taken from the line, `width`
    /// is the wrapping layout width (the line payload carries no width).
    pub fn from_line(width: f32, line: MeasuredLine) -> Self {
        let height = line.height;
        let node = Self {
            width,
            height,
            content: MeasuredContent::Line(Arc::new(line)),
        };
        debug_assert_eq!(node.height, height);
        node
    }

    pub(crate) fn page_break_policy(&self) -> PageBreakPolicy {
        match &self.content {
            MeasuredContent::Box(b) => b.page_break_policy,
            MeasuredContent::Line(_) | MeasuredContent::Atom(_) => PageBreakPolicy::Avoid,
            MeasuredContent::Spacing(_) | MeasuredContent::PageBreak => PageBreakPolicy::Auto,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MeasuredBox {
    pub node: Dot,
    pub style: BoxStyle,
    pub children: MeasuredChildren,
    pub page_break_policy: PageBreakPolicy,
}

#[derive(Debug, Clone)]
pub(crate) struct MeasuredAtom {
    pub node: Dot,
}

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;

    use super::*;

    fn atom_node(n: u64, height: f32) -> Arc<MeasuredNode> {
        Arc::new(MeasuredNode {
            width: 100.0,
            height,
            content: MeasuredContent::Atom(MeasuredAtom {
                node: Dot::new(1, n),
            }),
        })
    }

    #[test]
    fn from_blocks_aggregates_height_and_indexes() {
        let c = MeasuredChildren::from_blocks(vec![
            atom_node(1, 10.0),
            atom_node(2, 20.0),
            atom_node(3, 30.0),
        ]);
        assert_eq!(c.len(), 3);
        assert_eq!(c.total_height(), 60.0);
        assert_eq!(c[1].height, 20.0);
        assert_eq!(c.iter().count(), 3);
    }
}
