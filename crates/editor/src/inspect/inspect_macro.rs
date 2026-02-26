use crate::model::{
    Doc, Fragment, FragmentNode, Node, NodeId, NodeRef, ParagraphNode, Style, TextAlign, TextNode,
};
use crate::state::Selection;
use crate::types::Affinity;
use std::collections::{HashMap, HashSet};

const INDENT: &str = "    ";

pub fn inspect_state_as_macro(doc: &Doc, selection: &Selection) -> String {
    let mut labeler = Labeler::new(selection);
    labeler.collect_from_doc(doc);
    labeler.ensure_selection_labels();

    let mut result = String::new();
    for decl in labeler.declarations() {
        result.push_str(&decl);
        result.push('\n');
    }

    result.push_str("state! {\n");
    format_doc(doc, &labeler, &mut result);
    format_selection(selection, &labeler, &mut result);
    result.push('}');

    result
}

fn format_doc(doc: &Doc, labeler: &Labeler, output: &mut String) {
    let Some(root) = doc.node(NodeId::ROOT) else {
        output.push_str(&format!("{INDENT}doc {{}}\n"));
        return;
    };

    let children: Vec<_> = root.children().collect();
    if children.is_empty() {
        output.push_str(&format!("{INDENT}doc {{}}\n"));
        return;
    }

    output.push_str(&format!("{INDENT}doc {{\n"));
    for child in children {
        format_node(child, 2, labeler, output);
    }
    output.push_str(&format!("{INDENT}}}\n"));
}

fn format_node(node: NodeRef, indent_level: usize, labeler: &Labeler, output: &mut String) {
    let indent = INDENT.repeat(indent_level);
    let prefix = labeler.prefix(node.node_id());

    let Some(node_data) = node.node() else {
        let indent = INDENT.repeat(indent_level);
        let prefix = labeler.prefix(node.node_id());
        output.push_str(&format!("{indent}{prefix}[undecodable] {{}}\n"));
        return;
    };

    match node_data {
        Node::Paragraph(paragraph) => {
            let attrs = format_paragraph_attrs(paragraph);
            format_container_node(
                &format!("{prefix}paragraph{attrs}"),
                node,
                indent_level,
                labeler,
                output,
            );
        }
        Node::Blockquote(blockquote) => {
            let attrs = format_attributes(&[(
                "variant",
                format!("BlockquoteVariant::{:?}", blockquote.variant),
            )]);
            format_container_node(
                &format!("{prefix}blockquote{attrs}"),
                node,
                indent_level,
                labeler,
                output,
            );
        }
        Node::Text(text_node) => {
            let text_repr = format_text_node(text_node);
            output.push_str(&format!("{indent}{prefix}{text_repr}\n"));
        }
        Node::Image(image) => {
            let id_value = match &image.id {
                Some(s) => format!("Some(\"{}\".to_string())", escape_str(s)),
                None => "None".to_string(),
            };
            let attrs = format_attributes(&[
                ("id", id_value),
                ("proportion", format_number(image.proportion)),
            ]);
            output.push_str(&format!("{indent}{prefix}image{attrs}\n"));
        }
        Node::File(file) => {
            let id_value = match &file.id {
                Some(s) => format!("Some(\"{}\".to_string())", escape_str(s)),
                None => "None".to_string(),
            };
            let attrs = format_attributes(&[("id", id_value)]);
            output.push_str(&format!("{indent}{prefix}file{attrs}\n"));
        }
        Node::Embed(embed) => {
            let id_value = match &embed.id {
                Some(s) => format!("Some(\"{}\".to_string())", escape_str(s)),
                None => "None".to_string(),
            };
            let attrs = format_attributes(&[("id", id_value)]);
            output.push_str(&format!("{indent}{prefix}embed{attrs}\n"));
        }
        Node::Archived(archived) => {
            let id_value = match &archived.id {
                Some(s) => format!("Some(\"{}\".to_string())", escape_str(s)),
                None => "None".to_string(),
            };
            let attrs = format_attributes(&[("id", id_value)]);
            output.push_str(&format!("{indent}{prefix}archived{attrs}\n"));
        }
        Node::HardBreak(_) => {
            output.push_str(&format!("{indent}{prefix}hard_break {{}}\n"));
        }
        Node::HorizontalRule(_) => {
            output.push_str(&format!("{indent}{prefix}horizontal_rule {{}}\n"));
        }
        Node::PageBreak(_) => {
            output.push_str(&format!("{indent}{prefix}page_break {{}}\n"));
        }
        Node::BulletList(_) => {
            format_container_node(
                &format!("{prefix}bullet_list"),
                node,
                indent_level,
                labeler,
                output,
            );
        }
        Node::OrderedList(_) => {
            format_container_node(
                &format!("{prefix}ordered_list"),
                node,
                indent_level,
                labeler,
                output,
            );
        }
        Node::ListItem(_) => {
            format_container_node(
                &format!("{prefix}list_item"),
                node,
                indent_level,
                labeler,
                output,
            );
        }
        Node::Fold(_) => {
            format_container_node(
                &format!("{prefix}fold"),
                node,
                indent_level,
                labeler,
                output,
            );
        }
        Node::FoldTitle(_) => {
            format_container_node(
                &format!("{prefix}fold_title"),
                node,
                indent_level,
                labeler,
                output,
            );
        }
        Node::FoldContent(_) => {
            format_container_node(
                &format!("{prefix}fold_content"),
                node,
                indent_level,
                labeler,
                output,
            );
        }
        Node::Callout(callout) => {
            let attrs =
                format_attributes(&[("variant", format!("CalloutVariant::{:?}", callout.variant))]);
            format_container_node(
                &format!("{prefix}callout{attrs}"),
                node,
                indent_level,
                labeler,
                output,
            );
        }
        Node::Table(table) => {
            let attrs = vec![
                (
                    "border_style",
                    format!("TableBorderStyle::{:?}", table.border_style),
                ),
                ("proportion", format_number(table.proportion)),
            ];
            let attrs = format_attributes(&attrs);
            format_container_node(
                &format!("{prefix}table{attrs}"),
                node,
                indent_level,
                labeler,
                output,
            );
        }
        Node::TableRow(_) => {
            format_container_node(
                &format!("{prefix}table_row"),
                node,
                indent_level,
                labeler,
                output,
            );
        }
        Node::TableCell(_) => {
            format_container_node(
                &format!("{prefix}table_cell"),
                node,
                indent_level,
                labeler,
                output,
            );
        }
        Node::Root(_) => {}
    }
}

fn format_container_node(
    head: &str,
    node: NodeRef,
    indent_level: usize,
    labeler: &Labeler,
    output: &mut String,
) {
    let indent = INDENT.repeat(indent_level);
    let children: Vec<_> = node.children().collect();

    if children.is_empty() {
        output.push_str(&format!("{indent}{head} {{}}\n"));
        return;
    }

    output.push_str(&format!("{indent}{head} {{\n"));
    for child in children {
        format_node(child, indent_level + 1, labeler, output);
    }
    output.push_str(&format!("{}{}\n", INDENT.repeat(indent_level), "}"));
}

fn format_text_node(text_node: &TextNode) -> String {
    let segments = text_node.text.get_segments();

    if segments.is_empty() {
        return "text { \"\" }".to_string();
    }

    let mut parts = Vec::new();
    for segment in segments {
        let text = escape_str(&segment.text);
        if segment.styles.is_empty() {
            parts.push(format!("\"{text}\""));
            continue;
        }

        let mut styles = segment.styles.clone();
        styles.sort_by_key(|s| style_to_macro(s));
        let styles_str = styles
            .iter()
            .map(style_to_macro)
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("\"{text}\" => [{styles_str}]"));
    }

    format!("text {{ {} }}", parts.join(", "))
}

fn format_selection(selection: &Selection, labeler: &Labeler, output: &mut String) {
    let anchor = format_position(
        selection.anchor.node_id,
        selection.anchor.offset,
        selection.anchor.affinity,
        labeler,
    );
    if selection.anchor == selection.head {
        output.push_str(&format!("{INDENT}selection {{ ({anchor}) }}\n"));
        return;
    }

    let head = format_position(
        selection.head.node_id,
        selection.head.offset,
        selection.head.affinity,
        labeler,
    );
    output.push_str(&format!("{INDENT}selection {{ ({anchor}) -> ({head}) }}\n"));
}

fn format_position(
    node_id: NodeId,
    offset: usize,
    affinity: Affinity,
    labeler: &Labeler,
) -> String {
    let label = labeler.expr(node_id);
    if affinity == Affinity::default() {
        format!("{label}, {offset}")
    } else {
        format!("{label}, {offset}, Affinity::{affinity:?}")
    }
}

fn format_paragraph_attrs(paragraph: &ParagraphNode) -> String {
    let mut attrs = Vec::new();

    if paragraph.align != TextAlign::default() {
        attrs.push(("align", format!("TextAlign::{:?}", paragraph.align)));
    }

    format_attributes(&attrs)
}

fn format_attributes(attrs: &[(&str, String)]) -> String {
    if attrs.is_empty() {
        return String::new();
    }

    let mut parts = Vec::new();
    for (name, value) in attrs {
        parts.push(format!("{name}: {value}"));
    }

    let rendered = format!("{},", parts.join(", "));
    format!("({rendered})")
}

fn style_to_macro(style: &Style) -> String {
    match style {
        Style::BackgroundColor(s) => format!("bg_color(\"{}\")", escape_str(&s.color)),
        Style::Bold(_) => "bold()".to_string(),
        Style::TextColor(s) => format!("text_color(\"{}\")", escape_str(&s.color)),
        Style::FontSize(s) => format!("font_size({})", format_number(s.size)),
        Style::FontFamily(s) => format!("font_family(\"{}\")", escape_str(&s.family)),
        Style::FontWeight(s) => format!("font_weight({})", s.weight),
        Style::Italic(_) => "italic()".to_string(),
        Style::LetterSpacing(s) => format!("letter_spacing({})", format_number(s.spacing)),
        Style::Strikethrough(_) => "strikethrough()".to_string(),
        Style::Underline(_) => "underline()".to_string(),
    }
}

fn escape_str(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn format_number<T: Into<f64>>(num: T) -> String {
    let val: f64 = num.into();
    if val.fract().abs() < f64::EPSILON {
        format!("{val:.1}")
    } else {
        format!("{val}")
    }
}

struct Labeler {
    labels: HashMap<NodeId, String>,
    order: Vec<NodeId>,
    needed: HashSet<NodeId>,
}

impl Labeler {
    fn new(selection: &Selection) -> Self {
        let mut needed = HashSet::new();
        if selection.anchor.node_id != NodeId::ROOT {
            needed.insert(selection.anchor.node_id);
        }
        if selection.head.node_id != NodeId::ROOT {
            needed.insert(selection.head.node_id);
        }

        Self {
            labels: HashMap::new(),
            order: Vec::new(),
            needed,
        }
    }

    fn collect_from_doc(&mut self, doc: &Doc) {
        let Some(root) = doc.node(NodeId::ROOT) else {
            return;
        };
        self.visit_node(root);
    }

    fn visit_node(&mut self, node: NodeRef) {
        self.register(node.node_id());
        for child in node.children() {
            self.visit_node(child);
        }
    }

    fn register(&mut self, node_id: NodeId) {
        if !self.needed.contains(&node_id) || self.labels.contains_key(&node_id) {
            return;
        }

        let name = format!("n{}", self.labels.len() + 1);
        self.labels.insert(node_id, name);
        self.order.push(node_id);
    }

    fn ensure_selection_labels(&mut self) {
        let mut missing: Vec<_> = self
            .needed
            .iter()
            .filter(|id| !self.labels.contains_key(id))
            .copied()
            .collect();
        missing.sort_by(|a, b| a.to_string().cmp(&b.to_string()));

        for node_id in missing {
            let name = format!("n{}", self.labels.len() + 1);
            self.labels.insert(node_id, name);
            self.order.push(node_id);
        }
    }

    fn declarations(&self) -> Vec<String> {
        self.order
            .iter()
            .filter_map(|id| self.labels.get(id))
            .map(|name| format!("let mut {name} = id!();"))
            .collect()
    }

    fn prefix(&self, node_id: NodeId) -> String {
        self.labels
            .get(&node_id)
            .map(|name| format!("@{} ", name))
            .unwrap_or_default()
    }

    fn expr(&self, node_id: NodeId) -> String {
        if node_id == NodeId::ROOT {
            "NodeId::ROOT".to_string()
        } else {
            self.labels
                .get(&node_id)
                .cloned()
                .unwrap_or_else(|| "id!()".to_string())
        }
    }
}

pub fn inspect_fragment_as_macro(fragment: &Fragment) -> String {
    let mut labeler = Labeler::new_for_fragment();
    labeler.collect_from_fragment(fragment);

    let mut result = String::new();

    result.push_str("fragment! {\n");
    result.push_str(&format!("    open_start: {},\n", fragment.open_start));
    result.push_str(&format!("    open_end: {},\n\n", fragment.open_end));

    format_fragment(fragment, &labeler, &mut result);
    result.push('}');

    result
}

impl Labeler {
    fn new_for_fragment() -> Self {
        Self {
            labels: HashMap::new(),
            order: Vec::new(),
            needed: HashSet::new(),
        }
    }

    fn collect_from_fragment(&mut self, fragment: &Fragment) {
        for id in fragment.nodes.keys() {
            self.register(*id);
        }
    }
}

fn format_fragment(fragment: &Fragment, labeler: &Labeler, output: &mut String) {
    let top_levels = fragment.top_level_node_ids();
    if top_levels.is_empty() {
        return;
    }

    for id in top_levels {
        if let Some(node) = fragment.node(id) {
            format_fragment_node(id, node, fragment, 1, labeler, output);
        }
    }
}

fn format_fragment_node(
    id: NodeId,
    node: &FragmentNode,
    fragment: &Fragment,
    indent_level: usize,
    labeler: &Labeler,
    output: &mut String,
) {
    let indent = INDENT.repeat(indent_level);

    match node.data() {
        Node::Paragraph(paragraph) => {
            let attrs = format_paragraph_attrs(paragraph);
            format_fragment_container_node(
                &format!("paragraph{attrs}"),
                id,
                fragment,
                indent_level,
                labeler,
                output,
            );
        }
        Node::Blockquote(blockquote) => {
            let attrs = format_attributes(&[(
                "variant",
                format!("BlockquoteVariant::{:?}", blockquote.variant),
            )]);
            format_fragment_container_node(
                &format!("blockquote{attrs}"),
                id,
                fragment,
                indent_level,
                labeler,
                output,
            );
        }
        Node::Text(text_node) => {
            let text_repr = format_text_node(text_node);
            output.push_str(&format!("{indent}{text_repr}\n"));
        }
        Node::Image(image) => {
            let id_value = match &image.id {
                Some(s) => format!("Some(\"{}\".to_string())", escape_str(s)),
                None => "None".to_string(),
            };
            let attrs = format_attributes(&[
                ("id", id_value),
                ("proportion", format_number(image.proportion)),
            ]);
            output.push_str(&format!("{indent}image{attrs}\n"));
        }
        Node::File(file) => {
            let id_value = match &file.id {
                Some(s) => format!("Some(\"{}\".to_string())", escape_str(s)),
                None => "None".to_string(),
            };
            let attrs = format_attributes(&[("id", id_value)]);
            output.push_str(&format!("{indent}file{attrs}\n"));
        }
        Node::Embed(embed) => {
            let id_value = match &embed.id {
                Some(s) => format!("Some(\"{}\".to_string())", escape_str(s)),
                None => "None".to_string(),
            };
            let attrs = format_attributes(&[("id", id_value)]);
            output.push_str(&format!("{indent}embed{attrs}\n"));
        }
        Node::Archived(archived) => {
            let id_value = match &archived.id {
                Some(s) => format!("Some(\"{}\".to_string())", escape_str(s)),
                None => "None".to_string(),
            };
            let attrs = format_attributes(&[("id", id_value)]);
            output.push_str(&format!("{indent}archived{attrs}\n"));
        }
        Node::HardBreak(_) => {
            output.push_str(&format!("{indent}hard_break {{}}\n"));
        }
        Node::HorizontalRule(_) => {
            output.push_str(&format!("{indent}horizontal_rule {{}}\n"));
        }
        Node::PageBreak(_) => {
            output.push_str(&format!("{indent}page_break {{}}\n"));
        }
        Node::BulletList(_) => {
            format_fragment_container_node(
                "bullet_list",
                id,
                fragment,
                indent_level,
                labeler,
                output,
            );
        }
        Node::OrderedList(_) => {
            format_fragment_container_node(
                "ordered_list",
                id,
                fragment,
                indent_level,
                labeler,
                output,
            );
        }
        Node::ListItem(_) => {
            format_fragment_container_node(
                "list_item",
                id,
                fragment,
                indent_level,
                labeler,
                output,
            );
        }
        Node::Fold(_) => {
            format_fragment_container_node("fold", id, fragment, indent_level, labeler, output);
        }
        Node::FoldTitle(_) => {
            format_fragment_container_node(
                "fold_title",
                id,
                fragment,
                indent_level,
                labeler,
                output,
            );
        }
        Node::FoldContent(_) => {
            format_fragment_container_node(
                "fold_content",
                id,
                fragment,
                indent_level,
                labeler,
                output,
            );
        }
        Node::Callout(callout) => {
            let attrs =
                format_attributes(&[("variant", format!("CalloutVariant::{:?}", callout.variant))]);
            format_fragment_container_node(
                &format!("callout{attrs}"),
                id,
                fragment,
                indent_level,
                labeler,
                output,
            );
        }
        Node::Table(table) => {
            let attrs = vec![
                (
                    "border_style",
                    format!("TableBorderStyle::{:?}", table.border_style),
                ),
                ("proportion", format_number(table.proportion)),
            ];
            let attrs = format_attributes(&attrs);
            format_fragment_container_node(
                &format!("table{attrs}"),
                id,
                fragment,
                indent_level,
                labeler,
                output,
            );
        }
        Node::TableRow(_) => {
            format_fragment_container_node(
                "table_row",
                id,
                fragment,
                indent_level,
                labeler,
                output,
            );
        }
        Node::TableCell(_) => {
            format_fragment_container_node(
                "table_cell",
                id,
                fragment,
                indent_level,
                labeler,
                output,
            );
        }
        Node::Root(_) => {}
    }
}

fn format_fragment_container_node(
    head: &str,
    node_id: NodeId,
    fragment: &Fragment,
    indent_level: usize,
    labeler: &Labeler,
    output: &mut String,
) {
    let indent = INDENT.repeat(indent_level);
    let children = fragment.children_of_node(node_id);

    if children.is_empty() {
        output.push_str(&format!("{indent}{head} {{}}\n"));
        return;
    }

    output.push_str(&format!("{indent}{head} {{\n"));
    for (child_id, child_node) in children {
        format_fragment_node(
            child_id,
            child_node,
            fragment,
            indent_level + 1,
            labeler,
            output,
        );
    }
    output.push_str(&format!("{}{}\n", INDENT.repeat(indent_level), "}"));
}
