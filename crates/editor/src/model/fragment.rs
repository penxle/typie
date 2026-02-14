use crate::font::get_available_fonts;
use crate::model::{
    Doc, FontWeightStyle, Node, NodeId, NodeRef, ParagraphNode, Style, Text, TextNode, TextSegment,
};
use crate::schema::Schema;
use crate::state::position_helpers::find_child_at_offset;
use crate::state::{Position, Selection, StructureSelectionInfo, compute_structure_selection};
use crate::types::Affinity;
use anyhow::{Context, Result};
use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentNode {
    data: Node,
    parent: Option<NodeId>,
}

impl FragmentNode {
    pub fn new(data: Node, parent: Option<NodeId>) -> Self {
        Self { data, parent }
    }

    pub fn data(&self) -> &Node {
        &self.data
    }

    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    pub fn with_parent(mut self, parent: Option<NodeId>) -> Self {
        self.parent = parent;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fragment {
    pub nodes: IndexMap<NodeId, FragmentNode>,
    pub(crate) open_start: usize,
    pub(crate) open_end: usize,
}

#[derive(Debug)]
pub struct FragmentBuilder {
    nodes: IndexMap<NodeId, FragmentNode>,
    open_start: usize,
    open_end: usize,
}

impl FragmentBuilder {
    pub fn new() -> Self {
        Self {
            nodes: IndexMap::new(),
            open_start: 0,
            open_end: 0,
        }
    }

    pub fn add(mut self, node: (NodeId, FragmentNode)) -> Self {
        self.nodes.insert(node.0, node.1);
        self
    }

    pub fn open_start(mut self, depth: usize) -> Self {
        self.open_start = depth;
        self
    }

    pub fn open_end(mut self, depth: usize) -> Self {
        self.open_end = depth;
        self
    }

    pub fn build(self) -> Fragment {
        Fragment {
            nodes: self.nodes,
            open_start: self.open_start,
            open_end: self.open_end,
        }
    }
}

impl Default for FragmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Fragment {
    pub fn builder() -> FragmentBuilder {
        FragmentBuilder::new()
    }

    pub fn from_doc(doc: &Doc) -> Result<Self> {
        let root = doc.node(NodeId::ROOT).context("Root not found")?;
        let child_count = root.children().count();

        if child_count == 0 {
            return Ok(Self::empty());
        }

        let selection = Selection::new(
            Position::new(NodeId::ROOT, 0, Affinity::Downstream),
            Position::new(NodeId::ROOT, child_count, Affinity::Upstream),
        );

        Self::new_from_selection(doc, &selection)
    }

    fn extract_rectangular_cells(
        doc: &Doc,
        table_id: NodeId,
        range: ((usize, usize), (usize, usize)),
    ) -> Result<Self> {
        let table = doc.node(table_id).context("Table not found")?;

        let ((r_start, r_end), (c_start, c_end)) = range;

        let mut builder = Self::builder();

        let table_frag_node = FragmentNode::new(table.node().clone(), None);
        builder = builder.add((table_id, table_frag_node));

        let row_ids: Vec<_> = table.children().map(|c| c.node_id()).collect();

        for r in r_start..=r_end {
            if let Some(&row_id) = row_ids.get(r) {
                let row = doc.node(row_id).context("Row not found")?;
                let row_frag_node = FragmentNode::new(row.node().clone(), Some(table_id));
                builder = builder.add((row_id, row_frag_node));

                let cell_ids: Vec<_> = row.children().map(|c| c.node_id()).collect();

                for c in c_start..=c_end {
                    if let Some(&cell_id) = cell_ids.get(c) {
                        let cell = doc.node(cell_id).context("Cell not found")?;
                        let cell_frag_node = FragmentNode::new(cell.node().clone(), Some(row_id));
                        builder = builder.add((cell_id, cell_frag_node));

                        Self::collect_descendants(doc, cell_id, &mut builder)?;
                    }
                }
            }
        }

        Ok(builder.open_start(0).open_end(0).build())
    }

    fn collect_descendants(
        doc: &Doc,
        parent_id: NodeId,
        builder: &mut FragmentBuilder,
    ) -> Result<()> {
        if let Some(node) = doc.node(parent_id) {
            for child in node.children() {
                let child_id = child.node_id();
                let frag_node = FragmentNode::new(child.node().clone(), Some(parent_id));

                builder.nodes.insert(child_id, frag_node);
                Self::collect_descendants(doc, child_id, builder)?;
            }
        }
        Ok(())
    }

    pub fn from_text(text: &str, styles: &[Style]) -> Self {
        if text.is_empty() {
            return Self::empty();
        }

        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let lines: Vec<&str> = normalized.split('\n').collect();

        if lines.len() == 1 {
            let node_id = NodeId::new();
            let text_obj = Text::from(lines[0]);
            let len = text_obj.char_len();
            for style in styles {
                let _ = text_obj.apply_style(0..len, style);
            }
            let fragment_node = FragmentNode::new(
                Node::Text(TextNode {
                    text: text_obj,
                    ..Default::default()
                }),
                None,
            );
            return Self::builder()
                .add((node_id, fragment_node))
                .open_start(0)
                .open_end(0)
                .build();
        }

        let mut builder = Self::builder();

        for line in lines {
            let para_id = NodeId::new();
            let para_node = FragmentNode::new(Node::Paragraph(Default::default()), None);
            builder = builder.add((para_id, para_node));

            if !line.is_empty() {
                let text_id = NodeId::new();
                let text_obj = Text::from(line);
                let len = text_obj.char_len();
                for style in styles {
                    let _ = text_obj.apply_style(0..len, style);
                }
                let text_node = FragmentNode::new(
                    Node::Text(TextNode {
                        text: text_obj,
                        ..Default::default()
                    }),
                    Some(para_id),
                );
                builder = builder.add((text_id, text_node));
            }
        }

        builder.open_start(1).open_end(1).build()
    }

    pub fn new_from_selection(doc: &Doc, selection: &Selection) -> Result<Self> {
        if selection.is_collapsed() {
            return Ok(Self::empty());
        }

        let cell_info = compute_structure_selection(doc, selection);
        if let StructureSelectionInfo::Rectangular { table_id, range } = cell_info {
            return Self::extract_rectangular_cells(doc, table_id, range);
        }

        if let StructureSelectionInfo::Structural(ref block_ids) = cell_info {
            let (mut f, mut t) = selection.as_sorted(doc)?;

            for &block_id in block_ids {
                if let Some(block) = doc.node(block_id) {
                    if let (Some(parent), Some(idx)) = (block.parent(), block.index()) {
                        let from_node = doc.node(f.node_id).context("From node not found")?;
                        if f.node_id == block_id
                            || from_node.ancestors().any(|n| n.node_id() == block_id)
                        {
                            f = Position::new(parent.node_id(), idx, Affinity::Downstream);
                        }

                        let to_node = doc.node(t.node_id).context("To node not found")?;
                        if t.node_id == block_id
                            || to_node.ancestors().any(|n| n.node_id() == block_id)
                        {
                            t = Position::new(parent.node_id(), idx + 1, Affinity::Upstream);
                        }
                    }
                }
            }
            return Self::extract_range(doc, f, t);
        }

        let (from, to) = selection.as_sorted(doc)?;

        // 여러 블록에 걸친 선택
        if from.node_id != to.node_id {
            return Self::extract_range(doc, from, to);
        }

        // 같은 블록 내 선택
        let block = doc.node(from.node_id).context("Block not found")?;
        let Some((from_child_id, from_local)) = find_child_at_offset(&block, from.offset) else {
            return Self::extract_range(doc, from, to);
        };

        let from_child = doc
            .node(from_child_id)
            .context("new_from_selection: From child not found")?;

        // 단일 atomic 노드 선택 (Image, HorizontalRule, etc.)
        if Self::is_atomic_node_selection(&from_child, from_local, from.offset, to.offset) {
            return Ok(Self::extract_atomic_node(from_child_id, &from_child));
        }

        // 단일 텍스트 노드 부분 선택
        let Some((to_child_id, to_local)) = find_child_at_offset(&block, to.offset) else {
            return Self::extract_range(doc, from, to);
        };

        if from_child_id == to_child_id && matches!(from_child.node(), Node::Text(_)) {
            return Self::extract_single_text_node(doc, from_child_id, from_local, to_local);
        }

        Self::extract_range(doc, from, to)
    }

    fn is_atomic_node_selection(
        node: &NodeRef<'_>,
        local_offset: usize,
        from_offset: usize,
        to_offset: usize,
    ) -> bool {
        if matches!(node.node(), Node::Text(_)) {
            return false;
        }

        if local_offset != 0 || to_offset != from_offset + 1 {
            return false;
        }

        node.spec().content.is_leaf()
    }

    fn extract_atomic_node(node_id: NodeId, node: &NodeRef<'_>) -> Self {
        let parent_id = node.parent().map(|n| n.node_id());
        let fragment_node = FragmentNode::new(node.node().clone(), parent_id);
        Self::builder()
            .add((node_id, fragment_node))
            .open_start(0)
            .open_end(0)
            .build()
    }

    pub fn empty() -> Self {
        Self {
            nodes: IndexMap::new(),
            open_start: 0,
            open_end: 0,
        }
    }

    pub fn closed(self) -> Self {
        Self {
            open_start: 0,
            open_end: 0,
            ..self
        }
    }

    pub fn into_blocks(self, schema: &Schema) -> Self {
        let top_levels = self.top_level_node_ids();
        let has_inline_top_level = top_levels.iter().any(|&id| self.is_inline_node(id, schema));

        if !has_inline_top_level {
            return self.closed();
        }

        self.wrap_inline_nodes_in_paragraphs(schema)
    }

    pub fn split_at_page_breaks(self, schema: &Schema) -> Self {
        let ends_with_page_break = self
            .nodes
            .values()
            .last()
            .map(|n| matches!(n.data(), Node::PageBreak(_)))
            .unwrap_or(false);

        if !ends_with_page_break {
            return self;
        }

        let all_ids = self.collect_all_ids();
        let mut new_nodes = IndexMap::with_capacity(self.nodes.len() + 2);
        let mut current_para: Option<NodeId> = None;

        for (id, node) in &self.nodes {
            if !self.is_top_level(node, &all_ids) {
                new_nodes.insert(*id, node.clone());
                continue;
            }

            if self.is_inline_node(*id, schema) {
                let pid = *current_para.get_or_insert_with(|| {
                    let pid = NodeId::new();
                    new_nodes.insert(
                        pid,
                        FragmentNode::new(Node::Paragraph(ParagraphNode::default()), None),
                    );
                    pid
                });
                new_nodes.insert(*id, node.clone().with_parent(Some(pid)));
            } else {
                current_para = None;
                new_nodes.insert(*id, node.clone());
            }
        }

        new_nodes.insert(
            NodeId::new(),
            FragmentNode::new(Node::Paragraph(ParagraphNode::default()), None),
        );

        Self {
            nodes: new_nodes,
            open_start: 1,
            open_end: 1,
        }
    }

    fn is_inline_node(&self, id: NodeId, schema: &Schema) -> bool {
        self.node(id)
            .map(|n| schema.node_spec(n.data().as_type()).inline)
            .unwrap_or(false)
    }

    fn wrap_inline_nodes_in_paragraphs(self, schema: &Schema) -> Self {
        let all_ids = self.collect_all_ids();
        let mut new_nodes = IndexMap::with_capacity(self.nodes.len());
        let mut current_para: Option<NodeId> = None;

        for (id, node) in &self.nodes {
            if !self.is_top_level(node, &all_ids) {
                new_nodes.insert(*id, node.clone());
                continue;
            }

            if self.is_inline_node(*id, schema) {
                let pid = *current_para.get_or_insert_with(|| {
                    let pid = NodeId::new();
                    new_nodes.insert(
                        pid,
                        FragmentNode::new(Node::Paragraph(ParagraphNode::default()), None),
                    );
                    pid
                });
                new_nodes.insert(*id, node.clone().with_parent(Some(pid)));
            } else {
                current_para = None;
                new_nodes.insert(*id, node.clone());
            }
        }

        Self {
            nodes: new_nodes,
            open_start: self.open_start.min(1),
            open_end: self.open_end.min(1),
        }
    }

    // 부모와 같은 타입의 최상위 노드를 unwrap해서 중첩 방지
    pub fn flatten_for_merge(
        self,
        parent_discriminant: Option<std::mem::Discriminant<Node>>,
    ) -> Self {
        if !self.is_open() {
            return self;
        }

        let Some(parent_disc) = parent_discriminant else {
            return self;
        };

        let nodes_to_unwrap: Vec<NodeId> = self
            .top_level_node_ids()
            .into_iter()
            .filter(|id| {
                self.nodes
                    .get(id)
                    .map_or(false, |n| std::mem::discriminant(n.data()) == parent_disc)
            })
            .collect();

        if nodes_to_unwrap.is_empty() {
            return self;
        }

        let mut builder = Fragment::builder();
        for (id, node) in &self.nodes {
            if nodes_to_unwrap.contains(id) {
                continue;
            }
            let new_node = if node
                .parent()
                .map_or(false, |pid| nodes_to_unwrap.contains(&pid))
            {
                node.clone().with_parent(None)
            } else {
                node.clone()
            };
            builder = builder.add((*id, new_node));
        }

        builder
            .open_start(self.open_start)
            .open_end(self.open_end)
            .build()
    }

    pub fn flatten_for_merge_at(self, doc: &Doc, insert_pos: Position) -> Self {
        let parent_disc = doc
            .node(insert_pos.node_id)
            .and_then(|n| n.parent().map(|p| std::mem::discriminant(p.node())));
        self.flatten_for_merge(parent_disc)
    }

    #[allow(dead_code)]
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize fragment to JSON")
    }

    #[allow(dead_code)]
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to deserialize fragment from JSON")
    }

    pub fn to_plain_text(&self) -> String {
        let mut result = String::new();
        let mut last_was_block = false;

        for (_, node) in &self.nodes {
            match node.data() {
                Node::Text(text_node) => {
                    result.push_str(&text_node.text.as_str());
                    last_was_block = false;
                }
                Node::HardBreak(_) => {
                    result.push('\n');
                    last_was_block = false;
                }
                Node::Paragraph(_)
                | Node::Blockquote(_)
                | Node::BulletList(_)
                | Node::OrderedList(_)
                | Node::ListItem(_) => {
                    if !result.is_empty() && !last_was_block {
                        result.push('\n');
                    }
                    last_was_block = true;
                }
                _ => {}
            }
        }

        result
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[allow(dead_code)]
    pub fn child_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&NodeId, &FragmentNode)> {
        self.nodes.iter()
    }

    pub fn open_start(&self) -> usize {
        self.open_start
    }

    pub fn open_end(&self) -> usize {
        self.open_end
    }

    pub fn collect_all_ids(&self) -> FxHashSet<NodeId> {
        self.nodes.keys().copied().collect()
    }

    pub fn remap_ids(&self, id_map: &FxHashMap<NodeId, NodeId>) -> Self {
        let new_nodes = self
            .nodes
            .iter()
            .map(|(id, node)| {
                let new_id = id_map.get(id).copied().unwrap_or(*id);
                let new_parent = node.parent.map(|id| id_map.get(&id).copied().unwrap_or(id));
                let new_node = FragmentNode::new(node.data.clone(), new_parent);
                (new_id, new_node)
            })
            .collect();

        Self {
            nodes: new_nodes,
            open_start: self.open_start,
            open_end: self.open_end,
        }
    }

    pub fn with_fresh_ids_for_doc(&self, doc: &Doc) -> Self {
        let mut id_map = FxHashMap::default();
        for old_id in self.collect_all_ids() {
            if doc.node(old_id).is_some() {
                id_map.insert(old_id, NodeId::new());
            }
        }
        if id_map.is_empty() {
            return self.clone();
        }
        self.remap_ids(&id_map)
    }

    fn is_top_level(&self, node: &FragmentNode, all_ids: &FxHashSet<NodeId>) -> bool {
        node.parent().map_or(true, |pid| !all_ids.contains(&pid))
    }

    pub fn top_level_node_ids(&self) -> Vec<NodeId> {
        let all_ids = self.collect_all_ids();
        self.nodes
            .iter()
            .filter(|(_, item)| self.is_top_level(item, &all_ids))
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn content_node_ids(&self, schema: &Schema) -> Vec<NodeId> {
        if self.open_start == 0 {
            return self.top_level_node_ids();
        }

        let mut current_level = self.top_level_node_ids();

        for _ in 0..self.open_start {
            let mut next_level = Vec::new();
            for node_id in &current_level {
                if let Some(node) = self.nodes.get(node_id) {
                    let spec = schema.node_spec(node.data().as_type());
                    if spec.content.is_leaf() {
                        next_level.push(*node_id);
                        continue;
                    }

                    let mut pushed_child = false;
                    for (child_id, child_node) in &self.nodes {
                        if child_node.parent() == Some(*node_id) {
                            next_level.push(*child_id);
                            pushed_child = true;
                        }
                    }

                    if !pushed_child {
                        next_level.push(*node_id);
                    }
                }
            }
            current_level = next_level;
        }

        current_level
    }

    pub fn node(&self, id: NodeId) -> Option<&FragmentNode> {
        self.nodes.get(&id)
    }

    #[allow(dead_code)]
    pub fn text_segments_of_node(&self, node_id: NodeId) -> Vec<TextSegment> {
        if let Some(frag_node) = self.node(node_id) {
            if let Node::Text(text_node) = frag_node.data() {
                return text_node.text.get_segments();
            }
        }

        let mut segments = Vec::new();
        for (child_id, frag_node) in &self.nodes {
            if frag_node.parent() == Some(node_id) {
                segments.extend(self.text_segments_of_node(*child_id));
            }
        }
        segments
    }

    pub fn children_of_node(&self, node_id: NodeId) -> Vec<(NodeId, &FragmentNode)> {
        self.nodes
            .iter()
            .filter(|(_, frag_node)| frag_node.parent() == Some(node_id))
            .map(|(id, node)| (*id, node))
            .collect()
    }

    pub fn split_segments_at(
        text: &crate::model::Text,
        offset: usize,
    ) -> (Vec<TextSegment>, Vec<TextSegment>) {
        let mut left = Vec::new();
        let mut right = Vec::new();
        let mut consumed = 0;

        for seg in text.get_segments() {
            let seg_len = seg.text.chars().count();
            if consumed + seg_len <= offset {
                left.push(seg);
            } else if consumed >= offset {
                right.push(seg);
            } else {
                let split_point = offset - consumed;
                let chars: Vec<char> = seg.text.chars().collect();
                let left_text: String = chars[..split_point].iter().collect();
                let right_text: String = chars[split_point..].iter().collect();
                left.push(TextSegment {
                    text: left_text,
                    styles: seg.styles.clone(),
                    annotations: seg.annotations.clone(),
                });
                right.push(TextSegment {
                    text: right_text,
                    styles: seg.styles,
                    annotations: seg.annotations,
                });
            }
            consumed += seg_len;
        }

        (left, right)
    }

    pub fn normalize_font_weights(self) -> Self {
        let available = get_available_fonts();
        if available.is_empty() {
            return self;
        }

        let mut new_nodes = IndexMap::with_capacity(self.nodes.len());

        for (id, node) in &self.nodes {
            match node.data() {
                Node::Text(text_node) => {
                    let segments = text_node.text.get_segments();
                    let mut modified = false;
                    let mut new_segments = Vec::with_capacity(segments.len());

                    for seg in &segments {
                        let family = seg
                            .styles
                            .iter()
                            .find_map(|s| {
                                if let Style::FontFamily(ff) = s {
                                    Some(ff.family.as_str())
                                } else {
                                    None
                                }
                            })
                            .expect("segment must have FontFamily style");

                        let weight_idx = seg
                            .styles
                            .iter()
                            .position(|s| matches!(s, Style::FontWeight(_)));

                        if let Some(idx) = weight_idx {
                            if let Style::FontWeight(fw) = &seg.styles[idx] {
                                if let Some(weights) = available.get(family) {
                                    if !weights.contains(&fw.weight) {
                                        let nearest = nearest_weight(fw.weight, weights);
                                        let mut new_styles = seg.styles.clone();
                                        new_styles[idx] =
                                            Style::FontWeight(FontWeightStyle { weight: nearest });
                                        new_segments.push(TextSegment {
                                            text: seg.text.clone(),
                                            styles: new_styles,
                                            annotations: seg.annotations.clone(),
                                        });
                                        modified = true;
                                        continue;
                                    }
                                }
                            }
                        }

                        new_segments.push(seg.clone());
                    }

                    if modified {
                        let new_text = Text::from_segments(&new_segments);
                        new_nodes.insert(
                            *id,
                            FragmentNode::new(
                                Node::Text(TextNode { text: new_text }),
                                node.parent(),
                            ),
                        );
                    } else {
                        new_nodes.insert(*id, node.clone());
                    }
                }
                _ => {
                    new_nodes.insert(*id, node.clone());
                }
            }
        }

        Self {
            nodes: new_nodes,
            open_start: self.open_start,
            open_end: self.open_end,
        }
    }

    pub fn merge_adjacent_text_nodes(self) -> Self {
        let mut parent_to_children: rustc_hash::FxHashMap<Option<NodeId>, Vec<NodeId>> =
            rustc_hash::FxHashMap::default();
        for (id, node) in &self.nodes {
            parent_to_children
                .entry(node.parent())
                .or_default()
                .push(*id);
        }

        let mut next_nodes = self.nodes.clone();
        let mut removed_ids = rustc_hash::FxHashSet::default();

        for (_parent_id, siblings) in parent_to_children {
            let siblings_nodes = siblings.iter().map(|id| (*id, self.nodes[id].data()));
            let plans = Node::plan_consecutive_text_merges(siblings_nodes);

            for (keep_id, remove_ids, segments) in plans {
                let merged_text = Text::from_segments(&segments);
                let parent_id = self.nodes[&keep_id].parent();

                next_nodes.insert(
                    keep_id,
                    FragmentNode::new(Node::Text(TextNode { text: merged_text }), parent_id),
                );

                for rid in remove_ids {
                    removed_ids.insert(rid);
                }
            }
        }

        if !removed_ids.is_empty() {
            let mut final_nodes = IndexMap::new();
            for (id, node) in next_nodes {
                if !removed_ids.contains(&id) {
                    final_nodes.insert(id, node);
                }
            }
            Self {
                nodes: final_nodes,
                open_start: self.open_start,
                open_end: self.open_end,
            }
        } else {
            Self {
                nodes: next_nodes,
                open_start: self.open_start,
                open_end: self.open_end,
            }
        }
    }

    pub fn inline_len(&self, schema: &Schema) -> usize {
        self.nodes
            .iter()
            .filter(|(_, n)| schema.node_spec(n.data().as_type()).inline)
            .map(|(_, n)| n.data().len())
            .sum()
    }

    pub fn last_top_level_inline_len(&self, schema: &Schema) -> usize {
        let top_levels = self.top_level_node_ids();
        let last_id = match top_levels.last() {
            Some(id) => *id,
            None => return 0,
        };

        self.nodes
            .iter()
            .filter(|(_, n)| {
                n.parent() == Some(last_id) && schema.node_spec(n.data().as_type()).inline
            })
            .map(|(_, n)| n.data().len())
            .sum()
    }

    pub fn has_leaf_block(&self, schema: &Schema) -> bool {
        self.top_level_node_ids().iter().any(|id| {
            self.node(*id).map_or(false, |n| {
                let spec = schema.node_spec(n.data().as_type());
                !spec.inline && spec.content.is_leaf()
            })
        })
    }

    pub fn has_open_start(&self) -> bool {
        self.open_start > 0
    }

    pub fn has_open_end(&self) -> bool {
        self.open_end > 0
    }

    pub fn is_open(&self) -> bool {
        self.has_open_start() || self.has_open_end()
    }

    pub fn find_last_leaf_block(&self, root_id: NodeId) -> Option<NodeId> {
        let node = self.node(root_id)?;
        if matches!(node.data(), Node::Paragraph(_)) {
            return Some(root_id);
        }

        self.nodes
            .iter()
            .rev()
            .filter(|(_, n)| n.parent() == Some(root_id))
            .find_map(|(id, _)| self.find_last_leaf_block(*id))
    }

    fn extract_single_text_node(
        doc: &Doc,
        node_id: NodeId,
        from_offset: usize,
        to_offset: usize,
    ) -> Result<Self> {
        let node = doc.node(node_id).context("Node not found")?;
        let Node::Text(text_node) = node.node() else {
            anyhow::bail!("Expected text node");
        };

        let sliced_text = text_node.text.slice(from_offset, to_offset);

        let parent_id = node.parent().map(|n| n.node_id());
        let fragment_node =
            FragmentNode::new(Node::Text(TextNode { text: sliced_text }), parent_id);

        Ok(Self {
            nodes: IndexMap::from_iter([(node_id, fragment_node)]),
            open_start: 0,
            open_end: 0,
        })
    }

    fn extract_range(doc: &Doc, from: Position, to: Position) -> Result<Self> {
        let from_node_id = from.node_id;
        let to_node_id = to.node_id;

        let from_node = doc.node(from_node_id).context("From node not found")?;
        let to_node = doc.node(to_node_id).context("To node not found")?;

        let from_path = from_node.path();
        let to_path = to_node.path();

        let common_depth = from_path
            .iter()
            .zip(to_path.iter())
            .take_while(|(a, b)| a == b)
            .count();

        let open_start = from_path.len() - common_depth;
        let open_end = to_path.len() - common_depth;

        let mut collected_nodes = IndexMap::new();
        let mut visited = FxHashSet::default();

        Self::collect_nodes_in_range(doc, from, to, &mut collected_nodes, &mut visited)?;

        Ok(Self {
            nodes: collected_nodes,
            open_start,
            open_end,
        })
    }

    fn collect_nodes_in_range(
        doc: &Doc,
        from: Position,
        to: Position,
        collected: &mut IndexMap<NodeId, FragmentNode>,
        visited: &mut FxHashSet<NodeId>,
    ) -> Result<()> {
        let from_node_id = from.node_id;
        let to_node_id = to.node_id;

        if from_node_id != to_node_id
            && from_node_id != NodeId::ROOT
            && !visited.contains(&from_node_id)
        {
            let from_node = doc.node(from_node_id).context("From node not found")?;

            let from_ancestor_ids = Self::collect_ancestor_ids(doc, from_node_id);
            let to_ancestor_ids: FxHashSet<NodeId> = Self::collect_ancestor_ids(doc, to_node_id)
                .into_iter()
                .collect();

            let mut ancestors_to_add = Vec::new();
            for ancestor_id in from_ancestor_ids {
                if ancestor_id == NodeId::ROOT {
                    continue;
                }
                if to_ancestor_ids.contains(&ancestor_id) {
                    break;
                }
                if !visited.contains(&ancestor_id) {
                    ancestors_to_add.push(ancestor_id);
                }
            }

            for ancestor_id in ancestors_to_add.into_iter().rev() {
                let ancestor = doc.node(ancestor_id).context("Ancestor not found")?;
                let ancestor_parent_id = ancestor.parent().map(|n| n.node_id());
                let fragment_node = FragmentNode::new(ancestor.node().clone(), ancestor_parent_id);
                collected.insert(ancestor_id, fragment_node);
                visited.insert(ancestor_id);
            }

            let parent_id = from_node.parent().map(|n| n.node_id());
            let fragment_node = FragmentNode::new(from_node.node().clone(), parent_id);
            collected.insert(from_node_id, fragment_node);
            visited.insert(from_node_id);
        }

        if let Some((from_child_id, from_local)) = find_child_at_offset(
            &doc.node(from_node_id).context("From block not found")?,
            from.offset,
        ) {
            let node = doc
                .node(from_child_id)
                .context("collect_nodes_in_range: From child not found")?;
            let parent_id = node.parent().map(|n| n.node_id());

            match node.node() {
                Node::Text(text_node) => {
                    let text = &text_node.text;
                    let char_count = text.char_len();

                    if from_local < char_count {
                        let sliced_text = text.slice(from_local, char_count);
                        let fragment_node = FragmentNode::new(
                            Node::Text(TextNode { text: sliced_text }),
                            parent_id,
                        );
                        collected.insert(from_child_id, fragment_node);
                        visited.insert(from_child_id);
                    }
                }
                _ => {
                    if from_local == 0 {
                        let fragment_node = FragmentNode::new(node.node().clone(), parent_id);
                        collected.insert(from_child_id, fragment_node);
                        visited.insert(from_child_id);
                    }
                }
            }
        }

        let start_traversal_from = if let Some((child_id, _)) = find_child_at_offset(
            &doc.node(from_node_id).context("From block not found")?,
            from.offset,
        ) {
            child_id
        } else {
            from_node_id
        };

        let mut current_id = start_traversal_from;

        while let Some(next_id) = Self::next_in_dfs_order(doc, current_id) {
            if next_id == to_node_id {
                break;
            }

            if let Some((to_child_id, _)) = find_child_at_offset(
                &doc.node(to_node_id)
                    .context("To node not found in traversal")?,
                to.offset,
            ) {
                if next_id == to_child_id {
                    break;
                }
            }

            if !visited.contains(&next_id) {
                let node = doc.node(next_id).context("Node not found")?;
                let parent_id = node.parent().map(|n| n.node_id());
                let fragment_node = FragmentNode::new(node.node().clone(), parent_id);

                collected.insert(next_id, fragment_node);
                visited.insert(next_id);
            }

            current_id = next_id;
        }

        if from_node_id != to_node_id
            && to_node_id != NodeId::ROOT
            && !visited.contains(&to_node_id)
        {
            let to_node = doc.node(to_node_id).context("To node not found")?;
            let parent_id = to_node.parent().map(|n| n.node_id());
            let fragment_node = FragmentNode::new(to_node.node().clone(), parent_id);
            collected.insert(to_node_id, fragment_node);
            visited.insert(to_node_id);
        }

        if let Some((to_child_id, to_local)) = find_child_at_offset(
            &doc.node(to_node_id).context("To block not found")?,
            to.offset,
        ) {
            if !visited.contains(&to_child_id) {
                let node = doc.node(to_child_id).context("To child not found")?;
                let parent_id = node.parent().map(|n| n.node_id());

                match node.node() {
                    Node::Text(text_node) => {
                        let text = &text_node.text;
                        if to_local > 0 {
                            let sliced_text = text.slice(0, to_local);
                            let fragment_node = FragmentNode::new(
                                Node::Text(TextNode { text: sliced_text }),
                                parent_id,
                            );
                            collected.insert(to_child_id, fragment_node);
                        }
                    }
                    _ => {
                        if to_local > 0 {
                            let fragment_node = FragmentNode::new(node.node().clone(), parent_id);
                            collected.insert(to_child_id, fragment_node);
                            visited.insert(to_child_id);
                            Self::collect_subtree(doc, to_child_id, collected, visited)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn collect_subtree(
        doc: &Doc,
        node_id: NodeId,
        collected: &mut IndexMap<NodeId, FragmentNode>,
        visited: &mut FxHashSet<NodeId>,
    ) -> Result<()> {
        let node = doc.node(node_id).context("Node not found")?;
        for child in node.children() {
            let child_id = child.node_id();
            if !visited.contains(&child_id) {
                let fragment_node = FragmentNode::new(child.node().clone(), Some(node_id));
                collected.insert(child_id, fragment_node);
                visited.insert(child_id);
                Self::collect_subtree(doc, child_id, collected, visited)?;
            }
        }
        Ok(())
    }

    fn next_in_dfs_order(doc: &Doc, node_id: NodeId) -> Option<NodeId> {
        let node = doc.node(node_id)?;

        if let Some(first_child) = node.first_child() {
            return Some(first_child.node_id());
        }

        if let Some(next_sibling) = node.next_sibling() {
            return Some(next_sibling.node_id());
        }

        let mut current_id = node_id;
        loop {
            let current = doc.node(current_id)?;
            let parent = current.parent()?;
            if let Some(next_sibling) = parent.next_sibling() {
                return Some(next_sibling.node_id());
            }
            current_id = parent.node_id();
        }
    }

    // TODO: nodeRef.ancestors()를 쓰면 되지 않나
    fn collect_ancestor_ids(doc: &Doc, node_id: NodeId) -> Vec<NodeId> {
        let mut ancestors = Vec::new();
        let Some(node) = doc.node(node_id) else {
            return ancestors;
        };

        let mut current = node;
        while let Some(parent) = current.parent() {
            let parent_id = parent.node_id();
            if parent_id == NodeId::ROOT {
                break;
            }
            ancestors.push(parent_id);
            let Some(p) = doc.node(parent_id) else {
                break;
            };
            current = p;
        }

        ancestors
    }
}

fn nearest_weight(target: u16, weights: &[u16]) -> u16 {
    weights
        .iter()
        .copied()
        .min_by_key(|&w| (w as i32 - target as i32).abs())
        .unwrap_or(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn test_empty_fragment_from_collapsed_selection() {
        let doc = Rc::new(Doc::new());
        let selection = Selection::collapsed(Position::new(NodeId::ROOT, 0, Affinity::Downstream));

        let fragment = selection.extract_fragment(&doc).unwrap();

        assert!(fragment.is_empty());
        assert_eq!(fragment.child_count(), 0);
        assert_eq!(fragment.open_start(), 0);
        assert_eq!(fragment.open_end(), 0);
    }

    #[test]
    fn test_extract_partial_text_single_node() {
        let mut n = id!();

        let state = state! {
            doc {
                @n paragraph {
                    text { "Hello world" }
                }
            }
            selection {
                (n, 2) -> (n, 7)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();

        assert!(!fragment.is_empty());
        assert_eq!(fragment.child_count(), 1);
        assert_eq!(fragment.open_start(), 0);
        assert_eq!(fragment.open_end(), 0);

        let (_id, item) = &fragment.nodes.get_index(0).unwrap();
        if let Node::Text(text_node) = item.data() {
            assert_eq!(text_node.text.as_str(), "llo w");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_extract_full_text_node() {
        let mut n = id!();

        let state = state! {
            doc {
                @n paragraph {
                    text { "Hello" }
                }
            }
            selection {
                (n, 0) -> (n, 5)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();

        assert_eq!(fragment.child_count(), 1);
        assert_eq!(fragment.open_start(), 0);
        assert_eq!(fragment.open_end(), 0);

        let (_id, item) = &fragment.nodes.get_index(0).unwrap();
        if let Node::Text(text_node) = item.data() {
            assert_eq!(text_node.text.as_str(), "Hello");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_extract_multiple_paragraphs_partial() {
        let mut n1 = id!();
        let mut n2 = id!();

        let state = state! {
            doc {
                @n1 paragraph {
                    text { "First paragraph" }
                }
                paragraph {
                    text { "Second paragraph" }
                }
                @n2 paragraph {
                    text { "Third paragraph" }
                }
            }
            selection {
                (n1, 6) -> (n2, 5)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();

        assert!(!fragment.is_empty());
        assert!(fragment.child_count() > 0);

        let doc = &state.doc;
        let from_node = doc.node(n1).unwrap();
        let to_node = doc.node(n2).unwrap();
        let from_path = from_node.path();
        let to_path = to_node.path();

        let common_depth = from_path
            .iter()
            .zip(to_path.iter())
            .take_while(|(a, b)| a == b)
            .count();

        let expected_open_start = from_path.len() - common_depth;
        let expected_open_end = to_path.len() - common_depth;

        assert_eq!(fragment.open_start(), expected_open_start);
        assert_eq!(fragment.open_end(), expected_open_end);
    }

    #[test]
    fn test_extract_preserves_node_ids() {
        let mut n = id!();

        let state = state! {
            doc {
                @n paragraph {
                    text { "Hello world" }
                }
            }
            selection {
                (n, 0) -> (n, 5)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();

        assert_eq!(fragment.child_count(), 1);
        let (_extracted_id, item) = &fragment.nodes.get_index(0).unwrap();
        if let Node::Text(text_node) = item.data() {
            assert_eq!(text_node.text.as_str(), "Hello");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_extract_across_paragraph_boundary() {
        let mut n1 = id!();
        let mut n2 = id!();

        let state = state! {
            doc {
                @n1 paragraph {
                    text { "First" }
                }
                @n2 paragraph {
                    text { "Second" }
                }
            }
            selection {
                (n1, 2) -> (n2, 4)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();

        assert!(!fragment.is_empty());
        assert!(fragment.child_count() >= 2);

        assert_eq!(fragment.open_start(), 1);
        assert_eq!(fragment.open_end(), 1);
    }

    #[test]
    fn test_extract_with_offset_at_start() {
        let mut n = id!();

        let state = state! {
            doc {
                @n paragraph {
                    text { "Hello" }
                }
            }
            selection {
                (n, 0) -> (n, 3)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();

        let (_id, item) = &fragment.nodes.get_index(0).unwrap();
        if let Node::Text(text_node) = item.data() {
            assert_eq!(text_node.text.as_str(), "Hel");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_extract_with_offset_at_end() {
        let mut n = id!();

        let state = state! {
            doc {
                @n paragraph {
                    text { "Hello" }
                }
            }
            selection {
                (n, 2) -> (n, 5)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();

        let (_id, item) = &fragment.nodes.get_index(0).unwrap();
        if let Node::Text(text_node) = item.data() {
            assert_eq!(text_node.text.as_str(), "llo");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_extract_middle_portion() {
        let mut n = id!();

        let state = state! {
            doc {
                @n paragraph {
                    text { "Hello world from tests" }
                }
            }
            selection {
                (n, 6) -> (n, 17)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();

        let (_id, item) = &fragment.nodes.get_index(0).unwrap();
        if let Node::Text(text_node) = item.data() {
            assert_eq!(text_node.text.as_str(), "world from ");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_extract_single_character() {
        let mut n = id!();

        let state = state! {
            doc {
                @n paragraph {
                    text { "Hello" }
                }
            }
            selection {
                (n, 2) -> (n, 3)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();

        let (_id, item) = &fragment.nodes.get_index(0).unwrap();
        if let Node::Text(text_node) = item.data() {
            assert_eq!(text_node.text.as_str(), "l");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_fragment_collect_all_ids() {
        let mut n1 = id!();
        let mut n2 = id!();

        let state = state! {
            doc {
                @n1 paragraph {
                    text { "First" }
                }
                @n2 paragraph {
                    text { "Second" }
                }
            }
            selection {
                (n1, 0) -> (n2, 6)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();
        let all_ids = fragment.collect_all_ids();

        assert!(!all_ids.is_empty(), "Fragment should have node IDs");

        for (id, _) in fragment.iter().collect::<Vec<_>>() {
            assert!(
                all_ids.contains(id),
                "All fragment node IDs should be collected"
            );
        }
    }

    #[test]
    fn test_fragment_remap_ids() {
        let mut n = id!();

        let state = state! {
            doc {
                @n paragraph {
                    text { "Hello" }
                }
            }
            selection {
                (n, 0) -> (n, 5)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();

        let (original_id, _) = fragment.nodes.get_index(0).unwrap();
        let original_id = *original_id; // Copy to avoid reference lifetime issues
        let new_id = NodeId::new();
        let mut id_map = FxHashMap::default();
        id_map.insert(original_id, new_id);

        let remapped_fragment = fragment.remap_ids(&id_map);

        let (remapped_id, _) = remapped_fragment.nodes.get_index(0).unwrap();
        assert_eq!(*remapped_id, new_id);
        assert_ne!(*remapped_id, original_id);
    }

    #[test]
    fn test_extract_utf8_text() {
        let mut n = id!();

        let state = state! {
            doc {
                @n paragraph {
                    text { "안녕하세요" }
                }
            }
            selection {
                (n, 1) -> (n, 3)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();

        let (_id, item) = &fragment.nodes.get_index(0).unwrap();
        if let Node::Text(text_node) = item.data() {
            assert_eq!(text_node.text.as_str(), "녕하");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_extract_emoji() {
        let mut n = id!();

        let state = state! {
            doc {
                @n paragraph {
                    text { "Hello 👋 World 🌍" }
                }
            }
            selection {
                (n, 6) -> (n, 13)
            }
        };

        let fragment = state.selection.extract_fragment(&state.doc).unwrap();

        let (_id, item) = &fragment.nodes.get_index(0).unwrap();
        if let Node::Text(text_node) = item.data() {
            assert_eq!(text_node.text.as_str(), "👋 World");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_fragment_builder_empty() {
        let fragment = Fragment::builder().build();

        assert!(fragment.is_empty());
        assert_eq!(fragment.child_count(), 0);
        assert_eq!(fragment.open_start(), 0);
        assert_eq!(fragment.open_end(), 0);
    }

    #[test]
    fn test_fragment_builder_with_single_text_node() {
        let node_id = NodeId::new();
        let fragment_node = FragmentNode::new(
            Node::Text(TextNode {
                text: Text::from("Hello"),
                ..Default::default()
            }),
            None,
        );

        let fragment = Fragment::builder().add((node_id, fragment_node)).build();

        assert!(!fragment.is_empty());
        assert_eq!(fragment.child_count(), 1);
        assert_eq!(fragment.open_start(), 0);
        assert_eq!(fragment.open_end(), 0);

        let (_, node_item) = &fragment.nodes.get_index(0).unwrap();
        if let Node::Text(text_node) = node_item.data() {
            assert_eq!(text_node.text.as_str(), "Hello");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_fragment_builder_with_open_start_and_end() {
        let node_id = NodeId::new();
        let fragment_node = FragmentNode::new(
            Node::Text(TextNode {
                text: Text::from("Hello"),
                ..Default::default()
            }),
            None,
        );

        let fragment = Fragment::builder()
            .add((node_id, fragment_node))
            .open_start(1)
            .open_end(2)
            .build();

        assert_eq!(fragment.open_start(), 1);
        assert_eq!(fragment.open_end(), 2);
    }

    #[test]
    fn test_fragment_builder_with_multiple_nodes() {
        let node1_id = NodeId::new();
        let fragment_node1 = FragmentNode::new(
            Node::Text(TextNode {
                text: Text::from("Hello"),
                ..Default::default()
            }),
            None,
        );

        let node2_id = NodeId::new();
        let fragment_node2 = FragmentNode::new(
            Node::Text(TextNode {
                text: Text::from(" World"),
                ..Default::default()
            }),
            None,
        );

        let fragment = Fragment::builder()
            .add((node1_id, fragment_node1))
            .add((node2_id, fragment_node2))
            .build();

        assert_eq!(fragment.child_count(), 2);

        let nodes = fragment.iter().collect::<Vec<_>>();

        if let Node::Text(text_node) = nodes[0].1.data() {
            assert_eq!(text_node.text.as_str(), "Hello");
        } else {
            panic!("Expected text node");
        }

        if let Node::Text(text_node) = nodes[1].1.data() {
            assert_eq!(text_node.text.as_str(), " World");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_fragment_builder_add_simple() {
        let node_id = NodeId::new();
        let fragment_node = FragmentNode::new(
            Node::Text(TextNode {
                text: Text::from("Hello"),
                ..Default::default()
            }),
            None,
        );

        let fragment = Fragment::builder().add((node_id, fragment_node)).build();

        assert!(!fragment.is_empty());
        assert_eq!(fragment.child_count(), 1);

        let (_, node_item) = &fragment.nodes.get_index(0).unwrap();
        if let Node::Text(text_node) = node_item.data() {
            assert_eq!(text_node.text.as_str(), "Hello");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_fragment_builder_add_with_id() {
        let node_id = NodeId::new();
        let fragment_node = FragmentNode::new(
            Node::Text(TextNode {
                text: Text::from("Hello"),
                ..Default::default()
            }),
            None,
        );

        let fragment = Fragment::builder().add((node_id, fragment_node)).build();

        assert_eq!(fragment.child_count(), 1);
        let (id, _) = fragment.nodes.get_index(0).unwrap();
        assert_eq!(*id, node_id);
    }

    #[test]
    fn test_fragment_builder_add_multiple_simple() {
        let node1_id = NodeId::new();
        let fragment_node1 = FragmentNode::new(
            Node::Text(TextNode {
                text: Text::from("Hello"),
                ..Default::default()
            }),
            None,
        );

        let node2_id = NodeId::new();
        let fragment_node2 = FragmentNode::new(
            Node::Text(TextNode {
                text: Text::from(" World"),
                ..Default::default()
            }),
            None,
        );

        let fragment = Fragment::builder()
            .add((node1_id, fragment_node1))
            .add((node2_id, fragment_node2))
            .build();

        assert_eq!(fragment.child_count(), 2);
    }

    #[test]
    fn test_fragment_builder_with_parent_child_relationship() {
        let para_id = NodeId::new();
        let para_node = FragmentNode::new(Node::Paragraph(Default::default()), None);

        let text_id = NodeId::new();
        let text_node = FragmentNode::new(
            Node::Text(TextNode {
                text: Text::from("Hello"),
                ..Default::default()
            }),
            Some(para_id),
        );

        let fragment = Fragment::builder()
            .add((para_id, para_node))
            .add((text_id, text_node))
            .build();

        assert_eq!(fragment.child_count(), 2);

        let para_node = fragment.nodes.get_index(0).unwrap();
        let text_node = fragment.nodes.get_index(1).unwrap();

        assert_eq!(text_node.1.parent(), Some(*para_node.0));
    }

    #[test]
    fn test_fragment_from_selection() {
        let mut p = id!();
        let mut p2 = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "italic" }
                    text { "normal" }
                }
                @p2 paragraph {
                    text { "normal" }
                }
            }
            selection { (p, 8) -> (p2, 4) }
        };

        let fragment = Fragment::new_from_selection(&state.doc, &state.selection)
            .expect("fragment extraction failed");

        assert!(!fragment.is_empty());
        assert_eq!(fragment.child_count(), 4); // paragraph(p), text("rmal"), paragraph(p2), text("norm")

        assert_eq!(fragment.open_start(), 1);
        assert_eq!(fragment.open_end(), 1);

        let nodes = fragment.iter().collect::<Vec<_>>();

        let (_, item0) = &nodes[0];
        assert!(
            matches!(item0.data(), Node::Paragraph(_)),
            "First node should be paragraph p"
        );

        let (_, item1) = &nodes[1];
        if let Node::Text(t1) = item1.data() {
            assert_eq!(t1.text.as_str(), "rmal");
        } else {
            panic!("Expected second fragment node to be text 'rmal'");
        }

        let (_, item_last) = &nodes[nodes.len() - 1];
        if let Node::Text(t2) = item_last.data() {
            assert_eq!(t2.text.as_str(), "norm");
        } else {
            panic!("Expected last fragment text");
        }
    }

    #[test]
    fn test_fragment_nested_structure() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut bq = id!();

        let state = state! {
            doc {
                @p1 paragraph {
                    text { "Start" }
                }
                @bq blockquote {
                    @p2 paragraph {
                        text { "Nested" }
                    }
                }
            }
            selection {
                (p1, 2) -> (p2, 3)
            }
        };

        let fragment = Fragment::new_from_selection(&state.doc, &state.selection).unwrap();

        let expected = fragment! {
            open_start: 1,
            open_end: 2,

            paragraph {
                text { "art" }
            }
            blockquote {
                paragraph {
                    text { "Nes" }
                }
            }
        };

        assert_fragment_eq!(fragment, expected);
    }

    #[test]
    fn test_fragment_nested_structure_reverse() {
        let mut bq_p = id!();
        let mut p2 = id!();
        let mut bq = id!();

        let state = state! {
            doc {
                @bq blockquote {
                    @bq_p paragraph {
                        text { "AA" }
                    }
                }
                @p2 paragraph {
                    text { "BB" }
                }
            }
            selection {
                (bq_p, 0) -> (p2, 2)
            }
        };

        let fragment = Fragment::new_from_selection(&state.doc, &state.selection).unwrap();

        let expected = fragment! {
            open_start: 2,
            open_end: 1,

            blockquote {
                paragraph {
                    text { "AA" }
                }
            }
            paragraph {
                text { "BB" }
            }
        };

        assert_fragment_eq!(fragment, expected);
    }

    #[test]
    fn test_new_from_selection_rectangular_cells() {
        let mut p1 = id!(); // Row 0, Col 0
        let mut p2 = id!(); // Row 1, Col 1

        let state = state! {
            doc {
                table {
                    // Row 0
                    table_row {
                        table_cell { @p1 paragraph { text { "0-0" } } }
                        table_cell { paragraph { text { "0-1" } } }
                        table_cell { paragraph { text { "0-2" } } }
                    }
                    // Row 1
                    table_row {
                        table_cell { paragraph { text { "1-0" } } }
                        table_cell { @p2 paragraph { text { "1-1" } } }
                        table_cell { paragraph { text { "1-2" } } }
                    }
                }
            }
            selection { (p1, 0) -> (p2, 3) }
        };

        let fragment = Fragment::new_from_selection(&state.doc, &state.selection).unwrap();

        assert_eq!(fragment.top_level_node_ids().len(), 1);
        let (_, table_node) = fragment.nodes.get_index(0).unwrap();
        assert!(matches!(table_node.data(), Node::Table(_)));

        let table_id = *fragment.nodes.keys().next().unwrap();
        let rows = fragment.children_of_node(table_id);
        assert_eq!(rows.len(), 2, "Should extract 2 rows");

        let (r0_id, _) = rows[0];
        let r0_cells = fragment.children_of_node(r0_id);
        assert_eq!(r0_cells.len(), 2, "Row 0 should have 2 cells (0-0, 0-1)");

        let (c00_id, _) = r0_cells[0];
        let c00_content = fragment.children_of_node(c00_id);
        assert_eq!(c00_content.len(), 1);
        let (p00_id, _) = c00_content[0];
        let p00_text = fragment.text_segments_of_node(p00_id);
        assert_eq!(p00_text[0].text, "0-0");

        let (r1_id, _) = rows[1];
        let r1_cells = fragment.children_of_node(r1_id);
        assert_eq!(r1_cells.len(), 2, "Row 1 should have 2 cells (1-0, 1-1)");

        let (c11_id, _) = r1_cells[1];
        let c11_content = fragment.children_of_node(c11_id);
        let (p11_id, _) = c11_content[0];
        let p11_text = fragment.text_segments_of_node(p11_id);
        assert_eq!(p11_text[0].text, "1-1");
    }
    #[test]
    fn test_new_from_selection_full_tables_mixed() {
        let mut p1 = id!();
        let mut t1 = id!();
        let mut last_cell_p = id!();
        let mut p2 = id!();

        let state = state! {
            doc {
                @p1 paragraph { text { "Before" } }
                @t1 table {
                    table_row {
                        table_cell { paragraph { text { "Row 0 Cell 0" } } }
                        table_cell { paragraph { text { "Row 0 Cell 1" } } }
                    }
                    table_row {
                        table_cell { paragraph { text { "Row 1 Cell 0" } } }
                        table_cell { @last_cell_p paragraph { text { "Row 1 Cell 1" } } }
                    }
                }
                @p2 paragraph { text { "After" } }
            }
            selection { (last_cell_p, 0) -> (p2, 2) } // Select from LAST cell to "Af" of "After"
        };

        let fragment = Fragment::new_from_selection(&state.doc, &state.selection).unwrap();

        // Should return Table + Paragraph
        assert_eq!(
            fragment.top_level_node_ids().len(),
            2,
            "Should have 2 top level nodes"
        );

        // Check Table
        let table_node_entry = fragment
            .nodes
            .iter()
            .find(|(_, n)| matches!(n.data(), Node::Table(_)));
        assert!(table_node_entry.is_some(), "Should contain table");
        let (table_id, _) = table_node_entry.unwrap();

        let rows = fragment.children_of_node(*table_id);

        // Critical Assertion:
        // Visual selection highlights the WHOLE table because it crosses boundary.
        // User expects to copy the WHOLE table.
        // Current implementation likely only copies from the start point (Row 1 Cell 1) onwards.
        // So Row 0 will be missing.
        assert_eq!(rows.len(), 2, "Should contain ALL rows of the table");

        // Verify Row 0 content
        let (r0_id, _) = rows[0];
        let r0_cells = fragment.children_of_node(r0_id);
        assert_eq!(r0_cells.len(), 2, "Row 0 should have all cells");
    }

    #[test]
    fn with_fresh_ids_for_doc_preserves_ids_without_conflict() {
        let mut p = id!();

        let doc_state = state! {
            doc {
                @p paragraph { text { "Existing" } }
            }
            selection { (p, 0) }
        };

        let mut frag_p = id!();
        let fragment = fragment! {
            open_start: 1, open_end: 1,
            @frag_p paragraph { text { "New" } }
        };

        let result = fragment.with_fresh_ids_for_doc(&doc_state.doc);

        assert!(
            result.node(frag_p).is_some(),
            "Non-conflicting ID should be preserved"
        );
    }

    #[test]
    fn with_fresh_ids_for_doc_remaps_conflicting_ids() {
        let mut p = id!();

        let doc_state = state! {
            doc {
                @p paragraph { text { "Existing" } }
            }
            selection { (p, 0) }
        };

        let fragment = Fragment::new_from_selection(
            &doc_state.doc,
            &Selection::new(
                Position::new(p, 0, Affinity::Downstream),
                Position::new(p, 8, Affinity::Downstream),
            ),
        )
        .unwrap();

        let original_ids: Vec<NodeId> = fragment.nodes.keys().copied().collect();

        let result = fragment.with_fresh_ids_for_doc(&doc_state.doc);

        for old_id in &original_ids {
            if doc_state.doc.node(*old_id).is_some() {
                assert!(
                    result.node(*old_id).is_none(),
                    "Conflicting ID {old_id:?} should be remapped"
                );
            }
        }
        assert_eq!(result.nodes.len(), fragment.nodes.len());
    }

    #[test]
    fn with_fresh_ids_for_doc_remaps_only_conflicting_ids() {
        let mut p1 = id!();
        let mut p2 = id!();

        let source = state! {
            doc {
                @p1 paragraph { text { "First" } }
                @p2 paragraph { text { "Second" } }
            }
            selection { (p1, 0) -> (p2, 6) }
        };

        let fragment = source.selection.extract_fragment(&source.doc).unwrap();
        let fragment_ids = fragment.collect_all_ids();

        let mut target_p = id!();
        let target = state! {
            doc {
                @target_p paragraph { text { "Target" } }
            }
            selection { (target_p, 0) }
        };

        let has_conflict = fragment_ids.iter().any(|id| target.doc.node(*id).is_some());
        assert!(!has_conflict, "No fragment IDs should exist in target doc");

        let result = fragment.with_fresh_ids_for_doc(&target.doc);

        for id in &fragment_ids {
            assert!(
                result.node(*id).is_some(),
                "Non-conflicting ID {id:?} should be preserved"
            );
        }
    }
}
