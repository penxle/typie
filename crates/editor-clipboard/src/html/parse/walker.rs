use crate::html::parse::inheritance::{
    merge_pending_block, merge_with_inheritance, split_modifiers,
};
use crate::html::parse::normalize::normalize_modifier;
use crate::html::parse::rules::{
    compute_modifiers_for_element, modifier_parse_rules, node_parse_rules, try_parse_node,
};
use crate::html::parse::stylesheet::{ComputedStylesheet, Declaration, parse_inline_style};
use editor_model::{Fragment, Modifier, PlainNode, PlainTextNode};
use editor_resource::Resource;
use scraper::{ElementRef, Node as ScraperNode};

pub fn walk<'a>(
    node: ego_tree::NodeRef<'a, ScraperNode>,
    out: &mut Vec<Fragment>,
    inline_mods: &[Modifier],
    pending_block: &[Modifier],
    sheet: &ComputedStylesheet,
    resource: &Resource,
) {
    match node.value() {
        ScraperNode::Element(elem_data) => {
            let elem = ElementRef::wrap(node).expect("element wrap");
            let local = elem_data.name.local.to_string();
            if matches!(local.as_str(), "script" | "style") {
                return;
            }

            let stylesheet_decls = sheet.matched_for(&elem);
            let inline_decls: Vec<Declaration> = match elem.value().attr("style") {
                Some(s) => parse_inline_style(s),
                None => vec![],
            };
            let mut effective: Vec<Declaration> = inline_decls;
            for d in stylesheet_decls {
                if !effective.iter().any(|c| c.property == d.property) {
                    effective.push(d);
                }
            }
            let decls_kv: Vec<(String, String)> = effective
                .into_iter()
                .map(|d| (d.property, d.value))
                .collect();

            let raw = compute_modifiers_for_element(&elem, &decls_kv, modifier_parse_rules());
            let all: Vec<Modifier> = raw
                .into_iter()
                .filter_map(|m| normalize_modifier(m, resource))
                .collect();

            let (inline_part, block_part) = split_modifiers(all);

            let new_inline = merge_with_inheritance(inline_mods, inline_part);
            let new_pending = merge_pending_block(pending_block, block_part);

            if let Some(plain_node) = try_parse_node(&elem, node_parse_rules()) {
                match &plain_node {
                    PlainNode::HardBreak(_) => {
                        out.push(Fragment::leaf(plain_node));
                    }
                    PlainNode::HorizontalRule(_) => {
                        out.push(Fragment::leaf(plain_node).with_modifiers(new_pending));
                    }
                    _ => {
                        let mut kids = vec![];
                        for c in elem.children() {
                            walk(c, &mut kids, &new_inline, &[], sheet, resource);
                        }
                        out.push(Fragment {
                            node: plain_node,
                            modifiers: new_pending,
                            children: kids,
                        });
                    }
                }
                return;
            }

            for c in node.children() {
                walk(c, out, &new_inline, &new_pending, sheet, resource);
            }
        }
        ScraperNode::Text(text) => {
            let s = text.to_string();
            if !s.is_empty() {
                out.push(
                    Fragment::leaf(PlainNode::Text(PlainTextNode { text: s }))
                        .with_modifiers(inline_mods.to_vec()),
                );
            }
        }
        _ => {}
    }
}
