use crate::global::get_available_fonts;
use crate::model::*;
use crate::runtime::Effect;
use crate::state::position_helpers::find_child_at_offset;
use crate::state::{Position, block_content_len, calculate_block_offsets, collect_blocks_in_range};
use crate::transaction::Transaction;
use anyhow::{Context, Result};

pub(crate) fn compute_styles_at_cursor(tr: &Transaction, position: &Position) -> Vec<Style> {
    let cascade = tr.resolve_style_cascade(position.node_id);

    let Some(node) = tr.node(position.node_id) else {
        return cascade;
    };

    let Some((child_id, local_offset)) = find_child_at_offset(&node, position.offset) else {
        return cascade;
    };

    let Some(child) = tr.node(child_id) else {
        return cascade;
    };

    if let Node::Text(text_node) = child.node() {
        let segments = text_node.text.get_segments();
        let mut current_offset = 0;

        for segment in segments {
            let segment_len = segment.text.chars().count();
            if local_offset > current_offset && local_offset <= current_offset + segment_len {
                return fill_missing_styles(segment.styles, &cascade);
            }
            if local_offset == 0 && current_offset == 0 {
                return fill_missing_styles(segment.styles, &cascade);
            }
            current_offset += segment_len;
        }
    }

    cascade
}

fn fill_missing_styles(mut styles: Vec<Style>, defaults: &[Style]) -> Vec<Style> {
    for default in defaults {
        if !styles.iter().any(|s| s.as_type() == default.as_type()) {
            styles.push(default.clone());
        }
    }
    styles
}

fn apply_style_to_range(
    tr: &mut Transaction,
    from: Position,
    to: Position,
    style: &Style,
) -> Result<()> {
    let ranges = collect_text_ranges_in_selection(tr, from, to)?;
    let style_type = style.as_type();

    for (text_node_id, start_offset, end_offset) in ranges {
        let allowed = tr.doc().allowed_styles_for(text_node_id);
        anyhow::ensure!(
            allowed.contains(&style_type),
            "Style '{:?}' not allowed at node {}",
            style_type,
            text_node_id,
        );

        let node = tr.node_mut(text_node_id).context("Text node not found")?;
        if let Node::Text(text_node) = node.node() {
            let range = start_offset..end_offset;
            text_node.text.apply_style(range, style)?;
            tr.push_effect(Effect::NodeChanged {
                node_id: text_node_id,
            });
        }
    }

    update_cascade_attrs_on_empty_textblocks_in_range(tr, from, to, &[style.clone()])?;

    Ok(())
}

fn collect_empty_textblocks_in_range(
    tr: &Transaction,
    from: Position,
    to: Position,
) -> Result<Vec<NodeId>> {
    let mut block_ids = collect_blocks_in_range(tr.doc(), from, to)?;

    if to.offset == 0 && from.node_id != to.node_id {
        if let Some(block) = tr.node(to.node_id) {
            if block.spec().is_textblock(tr.doc().schema())
                && block_content_len(&block) == 0
                && !block_ids.contains(&to.node_id)
            {
                block_ids.push(to.node_id);
            }
        }
    }

    block_ids.retain(|&id| {
        tr.node(id)
            .map(|b| b.spec().is_textblock(tr.doc().schema()) && block_content_len(&b) == 0)
            .unwrap_or(false)
    });

    Ok(block_ids)
}

fn update_cascade_attrs_on_empty_textblocks_in_range(
    tr: &mut Transaction,
    from: Position,
    to: Position,
    styles: &[Style],
) -> Result<()> {
    let block_ids = collect_empty_textblocks_in_range(tr, from, to)?;

    for block_id in block_ids {
        let mut attrs = tr
            .node(block_id)
            .and_then(|b| b.cascade_attrs().map(|a| a.to_vec()))
            .unwrap_or_default();

        for style in styles {
            attrs.retain(|a| !matches!(a, Attr::Style(s) if s.as_type() == style.as_type()));
            attrs.push(Attr::Style(style.clone()));
        }

        tr.set_cascade_attrs(block_id, &attrs)?;
    }

    Ok(())
}

fn remove_style_from_range(
    tr: &mut Transaction,
    from: Position,
    to: Position,
    style_type: StyleType,
) -> Result<()> {
    let ranges = collect_text_ranges_in_selection(tr, from, to)?;

    for (text_node_id, start_offset, end_offset) in ranges {
        let node = tr.node_mut(text_node_id).context("Text node not found")?;
        if let Node::Text(text_node) = node.node() {
            let range = start_offset..end_offset;
            text_node.text.remove_style(range, style_type)?;
            tr.push_effect(Effect::NodeChanged {
                node_id: text_node_id,
            });
        }
    }

    remove_cascade_style_on_empty_textblocks_in_range(tr, from, to, style_type)?;

    Ok(())
}

fn remove_cascade_style_on_empty_textblocks_in_range(
    tr: &mut Transaction,
    from: Position,
    to: Position,
    style_type: StyleType,
) -> Result<()> {
    let block_ids = collect_empty_textblocks_in_range(tr, from, to)?;

    for block_id in block_ids {
        let Some(existing) = tr.node(block_id).and_then(|b| b.cascade_attrs()) else {
            continue;
        };

        let mut attrs: Vec<Attr> = existing.to_vec();
        let before_len = attrs.len();
        attrs.retain(|a| !matches!(a, Attr::Style(s) if s.as_type() == style_type));

        if attrs.len() != before_len {
            tr.set_cascade_attrs(block_id, &attrs)?;
        }
    }

    Ok(())
}

fn check_range_has_style(
    tr: &Transaction,
    from: Position,
    to: Position,
    style: &Style,
) -> Result<bool> {
    let ranges = collect_text_ranges_in_selection(tr, from, to)?;

    for (text_node_id, start_offset, end_offset) in ranges {
        let node = tr.node(text_node_id).context("Text node not found")?;
        if let Node::Text(text_node) = node.node() {
            let segments = text_node.text.get_segments();

            let mut current_offset = 0;
            for segment in segments {
                let segment_len = segment.text.chars().count();
                let segment_end = current_offset + segment_len;

                let overlap_start = current_offset.max(start_offset);
                let overlap_end = segment_end.min(end_offset);

                if overlap_start < overlap_end {
                    if !segment.styles.contains(style) {
                        return Ok(false);
                    }
                }

                current_offset = segment_end;
            }
        }
    }

    Ok(true)
}

fn get_common_style_in_range(
    tr: &Transaction,
    from: Position,
    to: Position,
    style_type: StyleType,
) -> Option<Style> {
    let ranges = collect_text_ranges_in_selection(tr, from, to).ok()?;
    let mut common_style: Option<Style> = None;

    for (text_node_id, start_offset, end_offset) in ranges {
        let node = tr.node(text_node_id)?;
        if let Node::Text(text_node) = node.node() {
            let segments = text_node.text.get_segments();
            let mut current_offset = 0;

            for segment in segments {
                let segment_len = segment.text.chars().count();
                let segment_end = current_offset + segment_len;

                let overlap_start = current_offset.max(start_offset);
                let overlap_end = segment_end.min(end_offset);

                if overlap_start < overlap_end {
                    let found = segment.styles.iter().find(|s| s.as_type() == style_type);
                    match (found, &common_style) {
                        (None, _) => return None,
                        (Some(s), None) => common_style = Some(s.clone()),
                        (Some(s), Some(existing)) if existing != s => return None,
                        _ => {}
                    }
                }

                current_offset = segment_end;
            }
        }
    }

    common_style
}

fn collect_text_ranges_in_selection(
    tr: &Transaction,
    from: Position,
    to: Position,
) -> Result<Vec<(NodeId, usize, usize)>> {
    let block_ids = collect_blocks_in_range(tr.doc(), from, to)?;
    let mut ranges = Vec::new();

    for block_id in block_ids {
        let block = tr
            .node(block_id)
            .with_context(|| format!("Block {block_id} not found"))?;

        if !block.spec().is_textblock(tr.doc().schema()) {
            continue;
        }

        let block_len = block_content_len(&block);
        let (start, end) = calculate_block_offsets(block_id, block_len, from, to);

        collect_ranges_in_textblock(&block, start, end, &mut ranges)?;
    }

    Ok(ranges)
}

fn collect_ranges_in_textblock(
    parent: &NodeRef,
    start_offset: usize,
    end_offset: usize,
    result: &mut Vec<(NodeId, usize, usize)>,
) -> Result<()> {
    let mut current_offset = 0;

    for child in parent.children() {
        match child.node() {
            Node::Text(text_node) => {
                let text_len = text_node.text.char_len();
                let child_end = current_offset + text_len;

                let overlap_start = current_offset.max(start_offset);
                let overlap_end = child_end.min(end_offset);

                if overlap_start < overlap_end {
                    let local_start = overlap_start - current_offset;
                    let local_end = overlap_end - current_offset;
                    result.push((child.node_id(), local_start, local_end));
                }

                current_offset = child_end;
            }
            Node::HardBreak(_) => {
                current_offset += 1;
            }
            _ => {}
        }
    }

    Ok(())
}

fn apply_font_style_normalized(
    tr: &mut Transaction,
    from: Position,
    to: Position,
    style: &Style,
) -> Result<()> {
    let ranges = collect_text_ranges_in_selection(tr, from, to)?;
    let available = get_available_fonts();
    let (default_family, _) = tr.resolved_font(from.node_id);
    let style_type = style.as_type();

    let mut actions: Vec<(NodeId, usize, usize, Vec<Style>)> = Vec::new();

    for &(text_node_id, start_offset, end_offset) in &ranges {
        let allowed = tr.doc().allowed_styles_for(text_node_id);
        anyhow::ensure!(
            allowed.contains(&style_type),
            "Style '{:?}' not allowed at node {}",
            style_type,
            text_node_id,
        );

        let node = tr.node(text_node_id).context("Text node not found")?;
        if let Node::Text(text_node) = node.node() {
            let segments = text_node.text.get_segments();
            let mut current_offset = 0;

            for segment in segments {
                let seg_len = segment.text.chars().count();
                let seg_end = current_offset + seg_len;
                let overlap_start = current_offset.max(start_offset);
                let overlap_end = seg_end.min(end_offset);

                if overlap_start < overlap_end {
                    let mut styles = Vec::new();

                    match style {
                        Style::FontWeight(fw) => {
                            let family = segment
                                .styles
                                .iter()
                                .find_map(|s| match s {
                                    Style::FontFamily(f) => Some(f.family.clone()),
                                    _ => None,
                                })
                                .unwrap_or_else(|| default_family.clone());

                            if let Some(w) = available.get(&family).filter(|w| !w.is_empty()) {
                                let weight = nearest_weight(w, fw.weight);
                                styles.push(Style::FontWeight(FontWeightStyle { weight }));
                            }
                        }
                        Style::FontFamily(fm) => {
                            if let Some(family_weights) =
                                available.get(&fm.family).filter(|w| !w.is_empty())
                            {
                                styles.push(style.clone());

                                if let Some(weight) = segment.styles.iter().find_map(|s| match s {
                                    Style::FontWeight(fw) => Some(fw.weight),
                                    _ => None,
                                }) {
                                    let normalized = nearest_weight(family_weights, weight);
                                    if normalized != weight {
                                        styles.push(Style::FontWeight(FontWeightStyle {
                                            weight: normalized,
                                        }));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }

                    actions.push((text_node_id, overlap_start, overlap_end, styles));
                }

                current_offset = seg_end;
            }
        }
    }

    for (text_node_id, start, end, styles) in actions {
        let node = tr.node_mut(text_node_id).context("Text node not found")?;
        if let Node::Text(text_node) = node.node() {
            for s in &styles {
                text_node.text.apply_style(start..end, s)?;
            }
            tr.push_effect(Effect::NodeChanged {
                node_id: text_node_id,
            });
        }
    }

    update_cascade_attrs_on_empty_textblocks_in_range(tr, from, to, &[style.clone()])?;

    Ok(())
}

fn collect_style_codepoints_in_selection(
    tr: &Transaction,
    style_type: StyleType,
) -> Vec<(Style, Vec<u32>)> {
    let selection = tr.selection();

    if selection.is_collapsed() {
        if let Some(style) = tr
            .state
            .pending_styles
            .iter()
            .find(|s| s.as_type() == style_type)
        {
            return vec![(style.clone(), tr.selection_codepoints())];
        }
        return vec![];
    }

    let (from, to) = match selection.as_sorted(tr.doc()) {
        Ok(pair) => pair,
        Err(_) => return vec![],
    };

    let ranges = match collect_text_ranges_in_selection(tr, from, to) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut grouped: rustc_hash::FxHashMap<Style, Vec<u32>> = rustc_hash::FxHashMap::default();

    for (text_node_id, start_offset, end_offset) in ranges {
        let Some(node) = tr.node(text_node_id) else {
            continue;
        };
        if let Node::Text(text_node) = node.node() {
            let segments = text_node.text.get_segments();
            let mut current_offset = 0;

            for segment in segments {
                let segment_len = segment.text.chars().count();
                let segment_end = current_offset + segment_len;

                let overlap_start = current_offset.max(start_offset);
                let overlap_end = segment_end.min(end_offset);

                if overlap_start < overlap_end {
                    if let Some(style) = segment.styles.iter().find(|s| s.as_type() == style_type) {
                        let cps = grouped.entry(style.clone()).or_default();
                        let text_start = overlap_start - current_offset;
                        let text_end = overlap_end - current_offset;
                        for ch in segment
                            .text
                            .chars()
                            .skip(text_start)
                            .take(text_end - text_start)
                        {
                            cps.push(ch as u32);
                        }
                    }
                }

                current_offset = segment_end;
            }
        }
    }

    grouped.into_iter().collect()
}

impl Transaction {
    fn update_cascade_attrs_if_empty_textblock(&self) {
        let head = self.selection().head;
        if !self.cursor_has_text_segment(head.node_id, head.offset) {
            let _ = self
                .set_cascade_attrs(head.node_id, &Attr::from_styles(&self.state.pending_styles));
        }
    }

    pub fn recompute_pending_styles(&mut self) {
        let new_styles = compute_styles_at_cursor(self, &self.selection().head);
        if self.state.pending_styles != new_styles {
            self.state.pending_styles = new_styles;
            self.push_effect(Effect::PendingStylesChanged);
        }
    }

    pub fn set_style(&mut self, style: Style) -> Result<bool> {
        let selection = self.selection().clone();

        match &style {
            Style::FontFamily(fm) => {
                let available = get_available_fonts();
                let family_name = fm.family.clone();
                let family_weights = available.get(&family_name);

                if selection.is_collapsed() {
                    if let Some(w) = family_weights.filter(|w| !w.is_empty()) {
                        let current_weight =
                            self.state.pending_styles.iter().find_map(|s| match s {
                                Style::FontWeight(fw) => Some(fw.weight),
                                _ => None,
                            });

                        self.state
                            .pending_styles
                            .retain(|s| s.as_type() != StyleType::FontFamily);
                        self.state.pending_styles.push(style);

                        if let Some(weight) = current_weight {
                            let normalized = nearest_weight(w, weight);
                            if normalized != weight {
                                self.state
                                    .pending_styles
                                    .retain(|s| s.as_type() != StyleType::FontWeight);
                                self.state.pending_styles.push(Style::FontWeight(
                                    FontWeightStyle { weight: normalized },
                                ));
                            }
                        }

                        self.push_effect(Effect::PendingStylesChanged);
                        self.update_cascade_attrs_if_empty_textblock();
                    }
                } else if let Some(w) = family_weights.filter(|w| !w.is_empty()) {
                    let mut grouped: rustc_hash::FxHashMap<u16, Vec<u32>> =
                        rustc_hash::FxHashMap::default();
                    for (s, codepoints) in
                        collect_style_codepoints_in_selection(self, StyleType::FontWeight)
                    {
                        let Style::FontWeight(fw) = s else {
                            continue;
                        };
                        let weight = nearest_weight(w, fw.weight);
                        grouped.entry(weight).or_default().extend(codepoints);
                    }
                    for (weight, codepoints) in grouped {
                        self.push_effect(Effect::FontDetected {
                            family: family_name.clone(),
                            weight,
                            codepoints,
                        });
                    }

                    let (from, to) = selection.as_sorted(self.doc())?;
                    apply_font_style_normalized(self, from, to, &style)?;
                }

                Ok(true)
            }
            Style::FontWeight(fw) => {
                let available = get_available_fonts();

                if selection.is_collapsed() {
                    let current_family = self.state.pending_styles.iter().find_map(|s| match s {
                        Style::FontFamily(f) => Some(f.family.clone()),
                        _ => None,
                    });

                    let default_family;
                    let family_for_lookup = match current_family.as_deref() {
                        Some(f) => f,
                        None => {
                            default_family = self.doc().default_attrs().font_family().to_string();
                            &default_family
                        }
                    };

                    if let Some(w) = available.get(family_for_lookup).filter(|w| !w.is_empty()) {
                        let normalized_weight = nearest_weight(w, fw.weight);

                        self.state
                            .pending_styles
                            .retain(|s| s.as_type() != StyleType::FontWeight);
                        self.state
                            .pending_styles
                            .push(Style::FontWeight(FontWeightStyle {
                                weight: normalized_weight,
                            }));
                        self.push_effect(Effect::PendingStylesChanged);
                        self.update_cascade_attrs_if_empty_textblock();
                    }
                } else {
                    let mut grouped: rustc_hash::FxHashMap<(String, u16), Vec<u32>> =
                        rustc_hash::FxHashMap::default();
                    for (s, codepoints) in
                        collect_style_codepoints_in_selection(self, StyleType::FontFamily)
                    {
                        let Style::FontFamily(fm) = s else {
                            continue;
                        };
                        if let Some(w) = available.get(&fm.family).filter(|w| !w.is_empty()) {
                            let weight = nearest_weight(w, fw.weight);
                            grouped
                                .entry((fm.family, weight))
                                .or_default()
                                .extend(codepoints);
                        }
                    }
                    for ((family, weight), codepoints) in grouped {
                        self.push_effect(Effect::FontDetected {
                            family,
                            weight,
                            codepoints,
                        });
                    }

                    let (from, to) = selection.as_sorted(self.doc())?;
                    apply_font_style_normalized(self, from, to, &style)?;
                }

                Ok(true)
            }
            _ => {
                if selection.is_collapsed() {
                    let style_type = style.as_type();
                    self.state
                        .pending_styles
                        .retain(|s| s.as_type() != style_type);
                    self.state.pending_styles.push(style);
                    self.push_effect(Effect::PendingStylesChanged);
                    self.update_cascade_attrs_if_empty_textblock();
                    return Ok(true);
                }

                let (from, to) = selection.as_sorted(self.doc())?;
                apply_style_to_range(self, from, to, &style)?;
                Ok(true)
            }
        }
    }

    pub fn toggle_style(&mut self, style: Style) -> Result<bool> {
        anyhow::ensure!(
            matches!(
                style,
                Style::Italic(_) | Style::Strikethrough(_) | Style::Underline(_)
            ),
            "toggle_style only supports Italic, Strikethrough, and Underline"
        );

        let selection = self.selection().clone();

        if selection.is_collapsed() {
            let style_type = style.as_type();
            let has_exact_style = self.state.pending_styles.contains(&style);

            self.state
                .pending_styles
                .retain(|s| s.as_type() != style_type);

            if !has_exact_style {
                self.state.pending_styles.push(style);
            }

            self.push_effect(Effect::PendingStylesChanged);
            self.update_cascade_attrs_if_empty_textblock();

            return Ok(true);
        }

        let (from, to) = selection.as_sorted(self.doc())?;
        let all_have_style = check_range_has_style(self, from.clone(), to.clone(), &style)?;

        if all_have_style {
            let style_type = style.as_type();
            remove_style_from_range(self, from, to, style_type)?;
        } else {
            apply_style_to_range(self, from, to, &style)?;
        }

        Ok(true)
    }

    pub fn reset_all_styles(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        let default_styles = self.resolve_style_cascade(NodeId::ROOT);

        if selection.is_collapsed() {
            let allowed = self
                .doc()
                .allowed_styles_for_content(selection.head.node_id);
            self.state.pending_styles = default_styles
                .into_iter()
                .filter(|s| allowed.contains(&s.as_type()))
                .collect();
            self.push_effect(Effect::PendingStylesChanged);
            self.update_cascade_attrs_if_empty_textblock();
        } else {
            let (from, to) = selection.as_sorted(self.doc())?;
            let text_ranges = collect_text_ranges_in_selection(self, from.clone(), to.clone())?;

            for (text_node_id, start_offset, end_offset) in text_ranges {
                let allowed = self.doc().allowed_styles_for(text_node_id);
                if allowed.is_empty() {
                    continue;
                }
                let range = start_offset..end_offset;

                for style in &default_styles {
                    if allowed.contains(&style.as_type()) {
                        let node = self.node_mut(text_node_id).context("Text node not found")?;
                        if let Node::Text(text_node) = node.node() {
                            text_node.text.apply_style(range.clone(), style)?;
                        }
                    }
                }

                for &style_type in StyleType::all() {
                    if allowed.contains(&style_type)
                        && !default_styles.iter().any(|s| s.as_type() == style_type)
                    {
                        let node = self.node_mut(text_node_id).context("Text node not found")?;
                        if let Node::Text(text_node) = node.node() {
                            text_node.text.remove_style(range.clone(), style_type)?;
                        }
                    }
                }

                self.push_effect(Effect::NodeChanged {
                    node_id: text_node_id,
                });
            }

            let block_ids = collect_empty_textblocks_in_range(self, from, to)?;
            for block_id in block_ids {
                let allowed = self.doc().allowed_styles_for_content(block_id);
                let filtered: Vec<Style> = default_styles
                    .iter()
                    .filter(|s| allowed.contains(&s.as_type()))
                    .cloned()
                    .collect();
                self.set_cascade_attrs(block_id, &Attr::from_styles(&filtered))?;
            }
        }

        Ok(true)
    }

    pub fn toggle_bold_style(&mut self) -> Result<bool> {
        let current_weight = match self.get_style_value(StyleType::FontWeight) {
            Some(Style::FontWeight(s)) => Some(s.weight),
            _ => None,
        };

        let family_name = match self.get_style_value(StyleType::FontFamily) {
            Some(Style::FontFamily(s)) => s.family.clone(),
            _ => self.resolved_font(self.selection().head.node_id).0,
        };

        let available = get_available_fonts();
        let weights = available.get(&family_name).cloned().unwrap_or_default();

        if weights.is_empty() {
            return Ok(false);
        }

        let normal_weight = nearest_weight(&weights, 400);
        let bold_weight = nearest_weight(&weights, 700);

        if normal_weight == bold_weight {
            return Ok(false);
        }

        let target_weight = if current_weight.unwrap_or(normal_weight) < bold_weight {
            bold_weight
        } else {
            normal_weight
        };

        let style = Style::FontWeight(FontWeightStyle {
            weight: target_weight,
        });
        for (fam_style, codepoints) in
            collect_style_codepoints_in_selection(self, StyleType::FontFamily)
        {
            let Style::FontFamily(fm) = fam_style else {
                continue;
            };
            self.push_effect(Effect::FontDetected {
                family: fm.family,
                weight: target_weight,
                codepoints,
            });
        }
        self.set_style(style)
    }

    pub fn get_style_value(&self, style_type: StyleType) -> Option<Style> {
        let selection = self.selection();

        if selection.is_collapsed() {
            if let Some(style) = self
                .state
                .pending_styles
                .iter()
                .find(|s| s.as_type() == style_type)
            {
                return Some(style.clone());
            }
        } else if let Ok((from, to)) = selection.as_sorted(self.doc()) {
            return get_common_style_in_range(self, from, to, style_type);
        }

        None
    }

    pub(crate) fn resolved_font(&self, node_id: NodeId) -> (String, u16) {
        let cascade = self.resolve_style_cascade(node_id);
        let mut family = cascade
            .iter()
            .find_map(|s| match s {
                Style::FontFamily(f) => Some(f.family.clone()),
                _ => None,
            })
            .unwrap_or_else(|| DefaultAttrs::default().font_family().to_string());
        let mut weight = cascade
            .iter()
            .find_map(|s| match s {
                Style::FontWeight(w) => Some(w.weight),
                _ => None,
            })
            .unwrap_or_else(|| DefaultAttrs::default().font_weight());

        for style in &self.state.pending_styles {
            match style {
                Style::FontFamily(f) => family = f.family.clone(),
                Style::FontWeight(w) => weight = w.weight,
                _ => {}
            }
        }

        if let Some(node_ref) = self.doc().node(node_id) {
            for style in &node_ref.node().style_overrides() {
                match style {
                    Style::FontFamily(f) => family = f.family.clone(),
                    Style::FontWeight(w) => weight = w.weight,
                    _ => {}
                }
            }
        }

        (family, weight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Message;
    use crate::state::Selection;
    use crate::test_utils::ScopedFontRegistration;
    use crate::types::Affinity;

    #[test]
    fn set_style_to_partial_text_node() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .set_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "hello" => [italic()],
                        " world"
                    }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_style_to_full_text_node() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .set_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_style_across_two_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { " world" }
                }
            }
            selection { (p, 2) -> (p, 9) }
        };

        let actual = transact!(initial, |tr| tr
            .set_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "he",
                        "llo" => [italic()]
                    }
                    text {
                        " wor" => [italic()],
                        "ld"
                    }
                }
            }
            selection { (p, 2) -> (p, 9) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_style_across_three_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { " beautiful" }
                    text { " world" }
                }
            }
            selection { (p, 2) -> (p, 19) }
        };

        let actual = transact!(initial, |tr| tr
            .set_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "he",
                        "llo" => [italic()]
                    }
                    text(styles: [italic()]) { " beautiful" }
                    text {
                        " wor" => [italic()],
                        "ld"
                    }
                }
            }
            selection { (p, 2) -> (p, 19) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_style_with_slot_at_start() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text { "hello" }
                }
            }
            selection { (p1, 0) -> (p2, 3) }
        };

        let actual = transact!(initial, |tr| tr
            .set_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text {
                        "hel" => [italic()],
                        "lo"
                    }
                }
            }
            selection { (p1, 0) -> (p2, 3) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_style_with_slot_at_end() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                @p2 paragraph { }
            }
            selection { (p1, 2) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .set_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text {
                        "he",
                        "llo" => [italic()]
                    }
                }
                @p2 paragraph { }
            }
            selection { (p1, 2) -> (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_style_with_slots_at_both_ends() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text { "hello" }
                }
                @p3 paragraph { }
            }
            selection { (p1, 0) -> (p3, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .set_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text(styles: [italic()]) { "hello" }
                }
                @p3 paragraph { }
            }
            selection { (p1, 0) -> (p3, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_style_across_multiple_paragraphs() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                @p2 paragraph {
                    text { "beautiful" }
                }
                @p3 paragraph {
                    text { "world" }
                }
            }
            selection { (p1, 2) -> (p3, 3) }
        };

        let actual = transact!(initial, |tr| tr
            .set_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text {
                        "he",
                        "llo" => [italic()]
                    }
                }
                @p2 paragraph {
                    text(styles: [italic()]) { "beautiful" }
                }
                @p3 paragraph {
                    text {
                        "wor" => [italic()],
                        "ld"
                    }
                }
            }
            selection { (p1, 2) -> (p3, 3) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_style_to_already_styled_text() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .set_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_style_to_multiple_text_nodes_full() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { " beautiful" }
                    text { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        let actual = transact!(initial, |tr| tr
            .set_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                    text(styles: [italic()]) { " beautiful" }
                    text(styles: [italic()]) { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_style_to_all_list_items() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph {
                            text { "A" }
                        }
                    }
                    list_item {
                        @p2 paragraph {
                            text { "B" }
                        }
                    }
                }
                @p2 paragraph {}
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .set_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph {
                            text(styles: [italic()]) { "A" }
                        }
                    }
                    list_item {
                        @p2 paragraph {
                            text(styles: [italic()]) { "B" }
                        }
                    }
                }
                @p2 paragraph {}
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_all_styled_to_unstyled() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_all_unstyled_to_styled() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_partial_styled_adds_to_unstyled() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                    text { " world" }
                }
            }
            selection { (p, 0) -> (p, 11) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                    text(styles: [italic()]) { " world" }
                }
            }
            selection { (p, 0) -> (p, 11) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_partial_selection_all_styled() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello world" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "hello",
                        " world" => [italic()]
                    }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_partial_selection_all_unstyled() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "hello" => [italic()],
                        " world"
                    }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_mixed_styles_across_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                    text { " beautiful" }
                    text(styles: [italic()]) { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                    text(styles: [italic()]) { " beautiful" }
                    text(styles: [italic()]) { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_all_styled_multiple_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                    text(styles: [italic()]) { " beautiful" }
                    text(styles: [italic()]) { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { " beautiful" }
                    text { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_with_slot_positions() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text { "hello" }
                }
                @p3 paragraph { }
            }
            selection { (p1, 0) -> (p3, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text(styles: [italic()]) { "hello" }
                }
                @p3 paragraph { }
            }
            selection { (p1, 0) -> (p3, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_across_multiple_paragraphs_mixed() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text(styles: [italic()]) { "hello" }
                }
                @p2 paragraph {
                    text { "beautiful" }
                }
                @p3 paragraph {
                    text(styles: [italic()]) { "world" }
                }
            }
            selection { (p1, 0) -> (p3, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text(styles: [italic()]) { "hello" }
                }
                @p2 paragraph {
                    text(styles: [italic()]) { "beautiful" }
                }
                @p3 paragraph {
                    text(styles: [italic()]) { "world" }
                }
            }
            selection { (p1, 0) -> (p3, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_partial_with_split() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                    text { " world" }
                }
            }
            selection { (p, 2) -> (p, 9) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                    text {
                        " wor" => [italic()],
                        "ld"
                    }
                }
            }
            selection { (p, 2) -> (p, 9) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_single_text_with_adjacent_text_position() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "wor" }
                    text { "ld" }
                }
            }
            selection { (p, 5) -> (p, 8) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text(styles: [italic()]) { "wor" }
                    text { "ld" }
                }
            }
            selection { (p, 5) -> (p, 8) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_partial_split_text() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "world" }
                }
            }
            selection { (p, 5) -> (p, 8) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text {
                        "wor" => [italic()],
                        "ld"
                    }
                }
            }
            selection { (p, 5) -> (p, 8) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_partial_split_text_multiple() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "world" }
                }
            }
            selection { (p, 2) -> (p, 4) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "he",
                        "ll" => [italic()],
                        "o"
                    }
                    text { "world" }
                }
            }
            selection { (p, 2) -> (p, 4) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_multiple_paragraphs() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                @p2 paragraph {
                    text { "world" }
                }
            }
            selection { (p1, 5) -> (p2, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_style(Style::Italic(ItalicStyle {}))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                @p2 paragraph {
                    text(styles: [italic()]) { "world" }
                }
            }
            selection { (p1, 5) -> (p2, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_style_collapsed_adds_to_pending_styles() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = Transaction::new(&state);
        tr.toggle_style(Style::Italic(ItalicStyle {})).unwrap();
        let (view, _) = tr.commit().unwrap();

        assert!(
            view.pending_styles
                .iter()
                .any(|s| matches!(s, Style::Italic(_)))
        );
    }

    #[test]
    fn toggle_style_collapsed_removes_from_pending_styles() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = Transaction::new(&state);
        tr.toggle_style(Style::Italic(ItalicStyle {})).unwrap();
        let (view, _) = tr.commit().unwrap();

        assert!(
            !view
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::Italic(_)))
        );
    }

    #[test]
    fn set_style_collapsed_sets_pending_styles() {
        let mut p = id!();

        let font_family = DefaultAttrs::default().font_family().to_string();
        let mut fonts = std::collections::HashMap::new();
        fonts.insert(font_family, vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();
        let (view, _) = tr.commit().unwrap();

        assert!(
            view.pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700))
        );
    }

    #[test]
    fn toggle_bold_style() {
        let mut p = id!();

        let font_family = DefaultAttrs::default().font_family().to_string();
        let mut fonts = std::collections::HashMap::new();
        fonts.insert(font_family.clone(), vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let (bold_state, effects) =
            transact_with_effect!(initial, |tr| tr.toggle_bold_style().unwrap());

        assert!(effects.iter().any(|e| matches!(e, Effect::FontDetected { family, weight: 700, .. } if family == &font_family)));

        let expected_bold = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(bold_state, expected_bold);

        let (normal_state, effects) =
            transact_with_effect!(bold_state, |tr| tr.toggle_bold_style().unwrap());

        assert!(effects.iter().any(|e| matches!(e, Effect::FontDetected { family, weight: 400, .. } if family == &font_family)));

        let expected_normal = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(400)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(normal_state, expected_normal);
    }

    #[test]
    fn toggle_bold_style_backward_selection() {
        let mut p = id!();

        let font_family = DefaultAttrs::default().font_family().to_string();
        let mut fonts = std::collections::HashMap::new();
        fonts.insert(font_family.clone(), vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text {
                        "가가가",
                        "가" => [font_weight(700)]
                    }
                }
            }
            selection { (p, 4) -> (p, 3) }
        };

        let (result_state, _) =
            transact_with_effect!(initial, |tr| tr.toggle_bold_style().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "가가가가" }
                }
            }
            selection { (p, 4) -> (p, 3) }
        };

        assert_state_eq!(result_state, expected);
    }

    #[test]
    fn test_style_application_invalidates_layout_cache() {
        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "Hello World" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        rt.layout();

        assert!(
            rt.is_layout_cached(p),
            "precondition: paragraph layout should be cached"
        );

        rt.update(Message::ToggleStyle {
            style: Style::Italic(ItalicStyle {}),
        });

        assert!(
            !rt.is_layout_cached(p),
            "paragraph layout cache should be invalidated after applying style"
        );
    }

    #[test]
    fn test_style_removal_invalidates_layout_cache() {
        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "Hello World" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        rt.layout();

        assert!(
            rt.is_layout_cached(p),
            "precondition: paragraph layout should be cached"
        );

        rt.update(Message::ToggleStyle {
            style: Style::Italic(ItalicStyle {}),
        });

        assert!(
            !rt.is_layout_cached(p),
            "paragraph layout cache should be invalidated after removing style"
        );
    }

    #[test]
    fn toggle_style_with_missing_400_weight() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("ThinFont".to_string(), vec![100]);

        let _guard = ScopedFontRegistration::new(fonts);

        let (_, effects) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontFamily(FontFamilyStyle {
                family: "ThinFont".to_string(),
            }))
            .unwrap()
        });

        let effect = effects
            .iter()
            .find(|e| matches!(e, Effect::FontDetected { .. }))
            .expect("Effect::FontDetected not found");

        if let Effect::FontDetected { family, weight, .. } = effect {
            assert_eq!(family, "ThinFont");
            assert_eq!(*weight, 100);
        } else {
            panic!("Unexpected effect");
        }
    }

    #[test]
    fn toggle_font_weight_collapsed_100_to_900() {
        let mut p = id!();

        let font_family = DefaultAttrs::default().font_family().to_string();
        let mut fonts = std::collections::HashMap::new();
        fonts.insert(font_family, vec![100, 900]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(100)]) { "hello" }
                }
            }
            selection { (p, 2) -> (p, 2) }
        };

        let actual_state = transact!(initial, |tr| {
            tr.set_style(Style::FontWeight(FontWeightStyle { weight: 900 }))
                .unwrap()
        });

        let pending = &actual_state.pending_styles;

        let has_900 = pending.contains(&Style::FontWeight(FontWeightStyle { weight: 900 }));
        assert!(
            has_900,
            "Should have switched to 900, but pending styles are: {:?}",
            pending
        );

        let has_100 = pending.contains(&Style::FontWeight(FontWeightStyle { weight: 100 }));
        assert!(!has_100, "Should have removed 100");
    }

    #[test]
    fn toggle_font_weight_collapsed_900_to_100() {
        let mut p = id!();

        let font_family = DefaultAttrs::default().font_family().to_string();
        let mut fonts = std::collections::HashMap::new();
        fonts.insert(font_family, vec![100, 900]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(900)]) { "hello" }
                }
            }
            selection { (p, 2) -> (p, 2) }
        };

        let actual_state = transact!(initial, |tr| {
            tr.set_style(Style::FontWeight(FontWeightStyle { weight: 100 }))
                .unwrap()
        });

        let pending = &actual_state.pending_styles;

        assert!(
            pending.contains(&Style::FontWeight(FontWeightStyle { weight: 100 })),
            "Should have switched to 100, styles: {:?}",
            pending
        );
        assert!(
            !pending.contains(&Style::FontWeight(FontWeightStyle { weight: 900 })),
            "Should have removed 900"
        );
    }

    #[test]
    fn select_all_and_toggle_italic() {
        let mut p1 = id!();
        let mut rt = runtime! {
          viewport { 800, 600, 1.0 }
          doc {
            @p1 paragraph { text { "hello" } }
            paragraph { text { "world" } }
          }
          selection { (p1, 0) }
        };

        rt.layout();
        rt.update(Message::SelectAll);
        rt.update(Message::ToggleStyle {
            style: Style::Italic(ItalicStyle {}),
        });

        let mut ep1 = id!();
        let mut ep2 = id!();
        let expected = state! {
          doc {
            @ep1 paragraph { text(styles: [italic()]) { "hello" } }
            @ep2 paragraph { text(styles: [italic()]) { "world" } }
          }
          selection { (ep1, 0) -> (ep2, 5, Affinity::Upstream) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn set_font_weight_emits_font_detected_with_codepoints() {
        let font_family = DefaultAttrs::default().font_family().to_string();
        let mut fonts = std::collections::HashMap::new();
        fonts.insert(font_family, vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                paragraph {
                    text { "aazz" }
                }
            }
            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 1) }
        };

        let (_, effects) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
                .unwrap()
        });

        let font_detected = effects
            .iter()
            .find(|e| matches!(e, Effect::FontDetected { weight: 700, .. }))
            .expect("FontDetected effect should be emitted when setting font weight");

        if let Effect::FontDetected { codepoints, .. } = font_detected {
            let expected: Vec<u32> = "aazz".chars().map(|c| c as u32).collect();
            assert_eq!(
                *codepoints, expected,
                "FontDetected should contain codepoints of selected text"
            );
        }
    }

    #[test]
    fn set_font_weight_normalizes_to_closest_available() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("NarrowFont".to_string(), vec![100, 400]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("NarrowFont")]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let (actual, effects) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
                .unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("NarrowFont"), font_weight(400)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(actual, expected);

        assert!(effects.iter().any(
            |e| matches!(e, Effect::FontDetected { family, weight: 400, .. } if family == "NarrowFont")
        ));
    }

    #[test]
    fn set_font_weight_normalizes_per_segment_family() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("FontA".to_string(), vec![400, 700]);
        fonts.insert("FontB".to_string(), vec![100, 300]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("FontA")]) { "aaa" }
                    text(styles: [font_family("FontB")]) { "bbb" }
                }
            }
            selection { (p, 0) -> (p, 6) }
        };

        let (actual, _) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
                .unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("FontA"), font_weight(700)]) { "aaa" }
                    text(styles: [font_family("FontB"), font_weight(300)]) { "bbb" }
                }
            }
            selection { (p, 0) -> (p, 6) }
        };
        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_font_weight_no_normalization_when_weight_available() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("FullFont".to_string(), vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("FullFont")]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let (actual, _) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
                .unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("FullFont"), font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_font_family_normalizes_existing_weight() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("LightFont".to_string(), vec![100, 300]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let (actual, effects) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontFamily(FontFamilyStyle {
                family: "LightFont".to_string(),
            }))
            .unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(300), font_family("LightFont")]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(actual, expected);

        assert!(effects.iter().any(
            |e| matches!(e, Effect::FontDetected { family, weight: 300, .. } if family == "LightFont")
        ));
    }

    #[test]
    fn set_font_family_no_normalization_when_weight_available() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("RichFont".to_string(), vec![400, 700, 900]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let (actual, _) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontFamily(FontFamilyStyle {
                family: "RichFont".to_string(),
            }))
            .unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700), font_family("RichFont")]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_font_family_normalizes_default_weight() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("LightFont".to_string(), vec![100]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let (actual, _) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontFamily(FontFamilyStyle {
                family: "LightFont".to_string(),
            }))
            .unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("LightFont"), font_weight(100)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_font_family_normalizes_different_weights_across_segments() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("TwoWeight".to_string(), vec![300, 600]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(100)]) { "aaa" }
                    text(styles: [font_weight(900)]) { "bbb" }
                }
            }
            selection { (p, 0) -> (p, 6) }
        };

        let (actual, _) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontFamily(FontFamilyStyle {
                family: "TwoWeight".to_string(),
            }))
            .unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(300), font_family("TwoWeight")]) { "aaa" }
                    text(styles: [font_weight(600), font_family("TwoWeight")]) { "bbb" }
                }
            }
            selection { (p, 0) -> (p, 6) }
        };
        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_font_weight_collapsed_normalizes_pending() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("NarrowFont".to_string(), vec![100, 400]);
        let _guard = ScopedFontRegistration::new(fonts);

        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontFamily(FontFamilyStyle {
            family: "NarrowFont".to_string(),
        }))
        .unwrap();
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();
        let (view, _) = tr.commit().unwrap();

        assert!(
            view.pending_styles
                .contains(&Style::FontWeight(FontWeightStyle { weight: 400 }))
        );
        assert!(
            !view
                .pending_styles
                .contains(&Style::FontWeight(FontWeightStyle { weight: 700 }))
        );
    }

    #[test]
    fn set_font_weight_collapsed_ignored_when_family_unavailable() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();
        let (view, _) = tr.commit().unwrap();

        assert!(
            !view
                .pending_styles
                .contains(&Style::FontWeight(FontWeightStyle { weight: 700 })),
            "FontWeight should not be set when default family is not in available_fonts"
        );
    }

    #[test]
    fn set_font_family_collapsed_normalizes_pending_weight() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("LightFont".to_string(), vec![100, 300]);
        let _guard = ScopedFontRegistration::new(fonts);

        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();
        tr.set_style(Style::FontFamily(FontFamilyStyle {
            family: "LightFont".to_string(),
        }))
        .unwrap();
        let (view, _) = tr.commit().unwrap();

        assert!(
            view.pending_styles
                .contains(&Style::FontWeight(FontWeightStyle { weight: 300 }))
        );
        assert!(
            !view
                .pending_styles
                .contains(&Style::FontWeight(FontWeightStyle { weight: 700 }))
        );
        assert!(
            view.pending_styles
                .contains(&Style::FontFamily(FontFamilyStyle {
                    family: "LightFont".to_string()
                }))
        );
    }

    #[test]
    fn set_font_family_collapsed_normalizes_default_weight() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("LightFont".to_string(), vec![100]);
        let _guard = ScopedFontRegistration::new(fonts);

        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontFamily(FontFamilyStyle {
            family: "LightFont".to_string(),
        }))
        .unwrap();
        let (view, _) = tr.commit().unwrap();

        assert!(
            view.pending_styles
                .contains(&Style::FontWeight(FontWeightStyle { weight: 100 }))
        );
        assert!(
            view.pending_styles
                .contains(&Style::FontFamily(FontFamilyStyle {
                    family: "LightFont".to_string()
                }))
        );
    }

    #[test]
    fn set_font_weight_collapsed_no_font_detected_effect() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("TestFont".to_string(), vec![400]);
        let _guard = ScopedFontRegistration::new(fonts);

        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();
        let (_, effects) = tr.commit().unwrap();

        assert!(
            !effects
                .iter()
                .any(|e| matches!(e, Effect::FontDetected { .. })),
            "collapsed selection should not emit FontDetected"
        );
    }

    #[test]
    fn set_font_family_font_detected_uses_normalized_weight() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("SingleWeight".to_string(), vec![300]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let (_, effects) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontFamily(FontFamilyStyle {
                family: "SingleWeight".to_string(),
            }))
            .unwrap()
        });

        assert!(
            effects.iter().any(
                |e| matches!(e, Effect::FontDetected { family, weight: 300, .. } if family == "SingleWeight")
            ),
            "FontDetected should use normalized weight 300, not original 700"
        );
        assert!(
            !effects
                .iter()
                .any(|e| matches!(e, Effect::FontDetected { weight: 700, .. })),
            "FontDetected should not contain original weight 700"
        );
    }

    #[test]
    fn set_font_weight_font_detected_uses_normalized_weight() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("ThinFont".to_string(), vec![100]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("ThinFont")]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let (_, effects) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
                .unwrap()
        });

        assert!(
            effects.iter().any(
                |e| matches!(e, Effect::FontDetected { family, weight: 100, .. } if family == "ThinFont")
            ),
            "FontDetected should use normalized weight 100, not original 700"
        );
    }

    #[test]
    fn set_font_family_partial_selection_normalizes() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("NarrowFont".to_string(), vec![100, 400]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "hello world" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let (actual, _) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontFamily(FontFamilyStyle {
                family: "NarrowFont".to_string(),
            }))
            .unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "hello" => [font_weight(400), font_family("NarrowFont")],
                        " world" => [font_weight(700)]
                    }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(actual, expected);
    }

    #[test]
    fn cascade_empty_paragraph_returns_root_styles() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };
        let tr = Transaction::new(&state);
        let cascade = tr.resolve_style_cascade(p);

        assert!(
            cascade.iter().any(|s| matches!(s, Style::FontWeight(_))),
            "Empty paragraph should inherit FontWeight from root, got: {:?}",
            cascade
        );
        assert!(
            cascade.iter().any(|s| matches!(s, Style::FontSize(_))),
            "Empty paragraph should inherit FontSize from root, got: {:?}",
            cascade
        );
        assert!(
            cascade.iter().any(|s| matches!(s, Style::FontFamily(_))),
            "Empty paragraph should inherit FontFamily from root, got: {:?}",
            cascade
        );
    }

    #[test]
    fn cascade_paragraph_overrides_root() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };
        let tr = Transaction::new(&state);
        tr.set_cascade_attrs(
            p,
            &Attr::from_styles(&[Style::FontWeight(FontWeightStyle { weight: 700 })]),
        )
        .unwrap();

        let cascade = tr.resolve_style_cascade(p);
        assert!(
            cascade
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "Paragraph cascade should override root font_weight, got: {:?}",
            cascade
        );
    }

    #[test]
    fn cascade_merges_paragraph_and_root() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };
        let tr = Transaction::new(&state);
        tr.set_cascade_attrs(
            p,
            &Attr::from_styles(&[Style::FontWeight(FontWeightStyle { weight: 700 })]),
        )
        .unwrap();

        let cascade = tr.resolve_style_cascade(p);

        assert!(
            cascade
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );
        assert!(cascade.iter().any(|s| matches!(s, Style::FontSize(_))));
        assert!(cascade.iter().any(|s| matches!(s, Style::FontFamily(_))));
    }

    #[test]
    fn cascade_paragraph_with_no_cascade_attrs_passes_through_to_root() {
        let mut p1 = id!();
        let mut p2 = id!();
        let state = state! {
            doc {
                @p1 paragraph {}
                @p2 paragraph {}
            }
            selection { (p1, 0) }
        };
        let tr = Transaction::new(&state);

        let c1 = tr.resolve_style_cascade(p1);
        let c2 = tr.resolve_style_cascade(p2);

        assert_eq!(c1.len(), c2.len());
    }

    #[test]
    fn cascade_overwrite_replaces_previous() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };
        let tr = Transaction::new(&state);

        tr.set_cascade_attrs(
            p,
            &Attr::from_styles(&[Style::FontWeight(FontWeightStyle { weight: 700 })]),
        )
        .unwrap();
        tr.set_cascade_attrs(
            p,
            &Attr::from_styles(&[Style::FontWeight(FontWeightStyle { weight: 300 })]),
        )
        .unwrap();

        let cascade = tr.resolve_style_cascade(p);
        assert!(
            cascade
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 300)),
            "Second set_cascade_attrs should overwrite, got: {:?}",
            cascade
        );
        assert!(
            !cascade
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );
    }

    #[test]
    fn compute_styles_empty_paragraph_uses_cascade() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };
        let tr = Transaction::new(&state);
        tr.set_cascade_attrs(
            p,
            &Attr::from_styles(&[Style::FontWeight(FontWeightStyle { weight: 700 })]),
        )
        .unwrap();

        let styles = compute_styles_at_cursor(&tr, &Position::new(p, 0, Affinity::Downstream));

        assert!(
            styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "compute_styles_at_cursor in empty paragraph should use cascade, got: {:?}",
            styles
        );
    }

    #[test]
    fn compute_styles_text_segment_fills_missing_from_cascade() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                }
            }
            selection { (p, 3) }
        };
        let tr = Transaction::new(&state);

        let styles = compute_styles_at_cursor(&tr, &Position::new(p, 3, Affinity::Downstream));

        assert!(styles.iter().any(|s| matches!(s, Style::Italic(_))));
        assert!(styles.iter().any(|s| matches!(s, Style::FontWeight(_))));
    }

    #[test]
    fn compute_styles_text_segment_overrides_cascade() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "bold" }
                }
            }
            selection { (p, 2) }
        };
        let tr = Transaction::new(&state);

        let styles = compute_styles_at_cursor(&tr, &Position::new(p, 2, Affinity::Downstream));

        assert!(
            styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );
    }

    #[test]
    fn set_cascade_attrs_on_inline_node_fails() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) }
        };

        let tr = Transaction::new(&state);
        let paragraph = tr.node(p).unwrap();
        let text_node_id = paragraph.first_child().unwrap().node_id();

        let result = tr.set_cascade_attrs(
            text_node_id,
            &Attr::from_styles(&[Style::FontWeight(FontWeightStyle { weight: 700 })]),
        );
        assert!(
            result.is_err(),
            "set_cascade_attrs on inline node should fail"
        );
    }

    #[test]
    fn split_at_end_stores_cascade_on_new_paragraph() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 5) }
        };

        let mut tr = Transaction::new(&state);
        tr.split_paragraph().unwrap();

        let head = tr.selection().head;
        let node = tr.node(head.node_id).unwrap();
        assert!(
            node.cascade_attrs().is_some(),
            "New paragraph after split should have cascade attrs"
        );
    }

    #[test]
    fn split_at_end_preserves_font_weight_in_pending() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 5) }
        };

        let state = transact!(state, |tr| tr.split_paragraph().unwrap());

        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "pending_styles after split should have font_weight(700), got: {:?}",
            state.pending_styles
        );
    }

    #[test]
    fn split_empty_paragraph_preserves_cascade() {
        let mut p = id!();

        let font_family = DefaultAttrs::default().font_family().to_string();
        let mut fonts = std::collections::HashMap::new();
        fonts.insert(font_family, vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();
        tr.split_paragraph().unwrap();
        let (state, _) = tr.commit().unwrap();

        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "Split from empty bold paragraph should preserve font_weight(700), got: {:?}",
            state.pending_styles
        );
    }

    #[test]
    fn split_in_middle_preserves_styles_on_new_paragraph() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 3) }
        };

        let state = transact!(state, |tr| tr.split_paragraph().unwrap());

        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );
    }

    #[test]
    fn delete_backward_last_char_stores_cascade() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "a" }
                }
            }
            selection { (p, 1) }
        };

        let mut tr = Transaction::new(&state);
        tr.delete_text_backward().unwrap();

        let node = tr.node(p).unwrap();
        let cascade = node
            .cascade_attrs()
            .map(|attrs| Attr::extract_styles(&attrs))
            .expect("Should have cascade attrs");
        assert!(
            cascade
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );
    }

    #[test]
    fn delete_backward_last_char_preserves_pending() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "a" }
                }
            }
            selection { (p, 1) }
        };

        let state = transact!(state, |tr| tr.delete_text_backward().unwrap());

        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "pending_styles should preserve font_weight(700) after deleting last char, got: {:?}",
            state.pending_styles
        );
    }

    #[test]
    fn delete_backward_partial_does_not_store_cascade() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "ab" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = Transaction::new(&state);
        tr.delete_text_backward().unwrap();

        let node = tr.node(p).unwrap();
        assert!(
            node.cascade_attrs().is_none(),
            "Paragraph with remaining text should not have cascade attrs set"
        );

        let (state, _) = tr.commit().unwrap();
        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );
    }

    #[test]
    fn delete_forward_last_char_stores_cascade() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "a" }
                }
            }
            selection { (p, 0) }
        };

        let mut tr = Transaction::new(&state);
        tr.delete_text_forward().unwrap();

        let node = tr.node(p).unwrap();
        let cascade = node
            .cascade_attrs()
            .map(|attrs| Attr::extract_styles(&attrs))
            .expect("Should have cascade attrs");
        assert!(
            cascade
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );
    }

    #[test]
    fn delete_forward_last_char_preserves_pending() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "a" }
                }
            }
            selection { (p, 0) }
        };

        let state = transact!(state, |tr| tr.delete_text_forward().unwrap());

        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "got: {:?}",
            state.pending_styles
        );
    }

    #[test]
    fn delete_selection_all_text_preserves_pending() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let state = transact!(state, |tr| tr.delete_selection().unwrap());

        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "got: {:?}",
            state.pending_styles
        );
    }

    #[test]
    fn set_style_collapsed_empty_paragraph_updates_cascade() {
        let mut p = id!();

        let font_family = DefaultAttrs::default().font_family().to_string();
        let mut fonts = std::collections::HashMap::new();
        fonts.insert(font_family, vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();

        let node = tr.node(p).unwrap();
        let cascade = node
            .cascade_attrs()
            .map(|attrs| Attr::extract_styles(&attrs))
            .expect("Should have cascade attrs");
        assert!(
            cascade
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );
    }

    #[test]
    fn toggle_style_collapsed_empty_paragraph_updates_cascade() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };

        let mut tr = Transaction::new(&state);
        tr.toggle_style(Style::Italic(ItalicStyle {})).unwrap();

        let node = tr.node(p).unwrap();
        let cascade = node
            .cascade_attrs()
            .map(|attrs| Attr::extract_styles(&attrs))
            .expect("Should have cascade attrs");
        assert!(
            cascade.iter().any(|s| matches!(s, Style::Italic(_))),
            "cascade attrs should contain Italic after toggle"
        );
    }

    #[test]
    fn set_style_collapsed_non_empty_paragraph_does_not_update_cascade() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 3) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();

        let node = tr.node(p).unwrap();
        let cascade = node
            .cascade_attrs()
            .map(|attrs| Attr::extract_styles(&attrs));
        if let Some(styles) = cascade {
            assert!(
                !styles
                    .iter()
                    .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            );
        }
    }

    #[test]
    fn multiple_style_changes_in_empty_paragraph_all_reflected() {
        let mut p = id!();

        let font_family = DefaultAttrs::default().font_family().to_string();
        let mut fonts = std::collections::HashMap::new();
        fonts.insert(font_family, vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();
        tr.toggle_style(Style::Italic(ItalicStyle {})).unwrap();

        let node = tr.node(p).unwrap();
        let cascade = node
            .cascade_attrs()
            .map(|attrs| Attr::extract_styles(&attrs))
            .expect("Should have cascade attrs");
        assert!(
            cascade
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700))
        );
        assert!(cascade.iter().any(|s| matches!(s, Style::Italic(_))));
    }

    #[test]
    fn join_backward_empty_paragraphs_preserves_pending() {
        let mut p1 = id!();
        let mut p2 = id!();
        let state = state! {
            doc {
                @p1 paragraph {}
                @p2 paragraph {}
            }
            selection { (p2, 0) }
        };

        let mut tr = Transaction::new(&state);
        tr.state
            .pending_styles
            .push(Style::FontWeight(FontWeightStyle { weight: 700 }));
        tr.join_backward().unwrap();
        let (view, _) = tr.commit().unwrap();

        assert!(
            view.pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "got: {:?}",
            view.pending_styles
        );
    }

    #[test]
    fn join_forward_empty_paragraphs_preserves_pending() {
        let mut p1 = id!();
        let mut p2 = id!();
        let state = state! {
            doc {
                @p1 paragraph {}
                @p2 paragraph {}
            }
            selection { (p1, 0) }
        };

        let mut tr = Transaction::new(&state);
        tr.state
            .pending_styles
            .push(Style::FontWeight(FontWeightStyle { weight: 700 }));
        tr.join_forward().unwrap();
        let (view, _) = tr.commit().unwrap();

        assert!(
            view.pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "got: {:?}",
            view.pending_styles
        );
    }

    #[test]
    fn styled_text_enter_twice_move_up_preserves_style() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 5) }
        };

        let state = transact!(state, |tr| tr.split_paragraph().unwrap());
        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );

        let state = transact!(state, |tr| tr.split_paragraph().unwrap());
        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );

        let mut tr = Transaction::new(&state);
        let root = tr.node(NodeId::ROOT).unwrap();
        let children: Vec<_> = root.children().collect();
        assert_eq!(children.len(), 3);
        let middle_id = children[1].node_id();

        tr.set_selection(Selection::collapsed(Position::new(
            middle_id,
            0,
            Affinity::Downstream,
        )));
        let (state, _) = tr.commit().unwrap();

        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "Moving to middle empty paragraph should show font_weight(700), got: {:?}",
            state.pending_styles
        );
    }

    #[test]
    fn bold_in_empty_paragraph_move_away_and_back_preserves() {
        let mut p1 = id!();
        let mut p2 = id!();

        let font_family = DefaultAttrs::default().font_family().to_string();
        let mut fonts = std::collections::HashMap::new();
        fonts.insert(font_family, vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let state = state! {
            doc {
                @p1 paragraph {
                    text { "normal" }
                }
                @p2 paragraph {}
            }
            selection { (p2, 0) }
        };

        let state = transact!(state, |tr| tr
            .set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap());

        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );

        let state = transact!(state, |tr| tr.set_selection(Selection::collapsed(
            Position::new(p1, 3, Affinity::Downstream)
        )));
        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 400)),
        );

        let state = transact!(state, |tr| tr.set_selection(Selection::collapsed(
            Position::new(p2, 0, Affinity::Downstream)
        )));
        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "Returning to empty bold paragraph should show font_weight(700), got: {:?}",
            state.pending_styles
        );
    }

    #[test]
    fn delete_all_text_then_move_back_preserves_style() {
        let mut p1 = id!();
        let mut p2 = id!();
        let state = state! {
            doc {
                @p1 paragraph {
                    text(styles: [font_weight(700)]) { "a" }
                }
                @p2 paragraph {
                    text { "normal" }
                }
            }
            selection { (p1, 1) }
        };

        let state = transact!(state, |tr| tr.delete_text_backward().unwrap());
        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );

        let state = transact!(state, |tr| tr.set_selection(Selection::collapsed(
            Position::new(p2, 3, Affinity::Downstream)
        )));
        let state = transact!(state, |tr| tr.set_selection(Selection::collapsed(
            Position::new(p1, 0, Affinity::Downstream)
        )));

        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "Returning to emptied bold paragraph should show font_weight(700), got: {:?}",
            state.pending_styles
        );
    }

    #[test]
    fn enter_from_italic_text_preserves_italic() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "hello" }
                }
            }
            selection { (p, 5) }
        };

        let state = transact!(state, |tr| tr.split_paragraph().unwrap());

        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::Italic(_))),
            "New paragraph after split should preserve italic, got: {:?}",
            state.pending_styles
        );
    }

    #[test]
    fn enter_from_multi_styled_text_preserves_all_styles() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700), italic()]) { "hello" }
                }
            }
            selection { (p, 5) }
        };

        let state = transact!(state, |tr| tr.split_paragraph().unwrap());

        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );
        assert!(
            state
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::Italic(_))),
        );
    }

    #[test]
    fn root_cascade_attrs_includes_paragraph_attr() {
        let state = state! {
            doc { paragraph {} }
            selection { (NodeId::ROOT, 0) }
        };
        let tr = Transaction::new(&state);
        let attrs = tr.resolve_attr_cascade(NodeId::ROOT);

        assert!(
            Attr::extract_paragraph_attr(&attrs).is_some(),
            "Root cascade_attrs should include ParagraphAttr, got: {:?}",
            attrs
        );
        let para = Attr::extract_paragraph_attr(&attrs).unwrap();
        assert!(
            (para.line_height - 1.6).abs() < f32::EPSILON,
            "Default line_height should be 1.6, got: {}",
            para.line_height
        );
    }

    #[test]
    fn resolve_attr_cascade_includes_styles_and_paragraph_attrs() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };
        let tr = Transaction::new(&state);
        let attrs = tr.resolve_attr_cascade(p);

        let styles = Attr::extract_styles(&attrs);
        assert!(
            !styles.is_empty(),
            "resolve_attr_cascade should include styles from root"
        );
        assert!(
            styles.iter().any(|s| matches!(s, Style::FontFamily(_))),
            "Should contain FontFamily style"
        );

        let para = Attr::extract_paragraph_attr(&attrs);
        assert!(
            para.is_some(),
            "resolve_attr_cascade should include ParagraphAttr from root"
        );
    }

    #[test]
    fn set_style_range_updates_cascade_attrs_on_empty_paragraph() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();
        let state = state! {
            doc {
                @p1 paragraph { text { "hello" } }
                @p2 paragraph {}
                @p3 paragraph { text { "world" } }
            }
            selection { (p1, 0) -> (p3, 5) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();

        let node = tr.node(p2).unwrap();
        let cascade = node
            .cascade_attrs()
            .map(|attrs| Attr::extract_styles(&attrs));
        assert!(
            cascade.is_some(),
            "Empty paragraph within range selection should have cascade_attrs set"
        );
        let styles = cascade.unwrap();
        assert!(
            styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "Empty paragraph's cascade_attrs should contain the applied FontWeight(700), got: {:?}",
            styles
        );
    }

    #[test]
    fn toggle_style_range_removes_cascade_attrs_on_empty_paragraph() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();
        let state = state! {
            doc {
                @p1 paragraph { text { "hello" } }
                @p2 paragraph {}
                @p3 paragraph { text { "world" } }
            }
            selection { (p1, 0) -> (p3, 5) }
        };

        let mut tr = Transaction::new(&state);
        tr.toggle_style(Style::Italic(ItalicStyle {})).unwrap();

        let node = tr.node(p2).unwrap();
        let cascade = node
            .cascade_attrs()
            .map(|attrs| Attr::extract_styles(&attrs));
        assert!(
            cascade.is_some(),
            "After apply: empty paragraph should have cascade_attrs"
        );
        assert!(
            cascade
                .unwrap()
                .iter()
                .any(|s| matches!(s, Style::Italic(_))),
            "After apply: cascade_attrs should contain Italic"
        );

        let (state, _) = tr.commit().unwrap();
        let mut tr = Transaction::new(&state);
        tr.toggle_style(Style::Italic(ItalicStyle {})).unwrap();

        let node = tr.node(p2).unwrap();
        let has_italic = node
            .cascade_attrs()
            .map(|attrs| {
                Attr::extract_styles(&attrs)
                    .iter()
                    .any(|s| matches!(s, Style::Italic(_)))
            })
            .unwrap_or(false);
        assert!(
            !has_italic,
            "After remove: empty paragraph's cascade_attrs should not contain Italic"
        );
    }

    #[test]
    fn set_style_range_updates_cascade_attrs_on_two_empty_paragraphs() {
        let mut p1 = id!();
        let mut p2 = id!();
        let state = state! {
            doc {
                @p1 paragraph {}
                @p2 paragraph {}
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();

        for (label, id) in [("p1", p1), ("p2", p2)] {
            let node = tr.node(id).unwrap();
            let cascade = node
                .cascade_attrs()
                .map(|attrs| Attr::extract_styles(&attrs));
            assert!(cascade.is_some(), "{label} should have cascade_attrs set");
            assert!(
                cascade
                    .unwrap()
                    .iter()
                    .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
                "{label}'s cascade_attrs should contain FontWeight(700)"
            );
        }
    }

    #[test]
    fn set_font_family_ignored_when_not_in_available_fonts() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("KnownFont".to_string(), vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| {
            tr.set_style(Style::FontFamily(FontFamilyStyle {
                family: "UnknownFont".to_string(),
            }))
            .unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_font_weight_ignored_when_family_not_in_available_fonts() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("KnownFont".to_string(), vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("UnknownFont")]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| {
            tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
                .unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("UnknownFont")]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_font_family_applies_when_in_available_fonts() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("KnownFont".to_string(), vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| {
            tr.set_style(Style::FontFamily(FontFamilyStyle {
                family: "KnownFont".to_string(),
            }))
            .unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("KnownFont")]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_font_weight_ignored_per_segment_when_family_unavailable() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("FontA".to_string(), vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("FontA")]) { "aaa" }
                    text(styles: [font_family("FontB")]) { "bbb" }
                }
            }
            selection { (p, 0) -> (p, 6) }
        };

        let (actual, _) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
                .unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("FontA"), font_weight(700)]) { "aaa" }
                    text(styles: [font_family("FontB")]) { "bbb" }
                }
            }
            selection { (p, 0) -> (p, 6) }
        };
        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_font_family_no_font_detected_when_unavailable() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("KnownFont".to_string(), vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(400)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let (_, effects) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontFamily(FontFamilyStyle {
                family: "UnknownFont".to_string(),
            }))
            .unwrap()
        });

        assert!(
            !effects
                .iter()
                .any(|e| matches!(e, Effect::FontDetected { .. })),
            "FontDetected should not be emitted for unavailable family, got: {:?}",
            effects
        );
    }

    #[test]
    fn set_font_weight_no_font_detected_for_unavailable_family_segment() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("FontA".to_string(), vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("FontA")]) { "aaa" }
                    text(styles: [font_family("FontB")]) { "bbb" }
                }
            }
            selection { (p, 0) -> (p, 6) }
        };

        let (_, effects) = transact_with_effect!(initial, |tr| {
            tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
                .unwrap()
        });

        assert!(
            effects.iter().any(
                |e| matches!(e, Effect::FontDetected { family, weight: 700, .. } if family == "FontA")
            ),
            "FontDetected should be emitted for available FontA"
        );
        assert!(
            !effects
                .iter()
                .any(|e| matches!(e, Effect::FontDetected { family, .. } if family == "FontB")),
            "FontDetected should not be emitted for unavailable FontB"
        );
    }
}
