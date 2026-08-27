use editor_model::PlainNode;

use crate::address::{Address, NodePath, display_path, node_at, resolve, types_along};
use crate::error::{XmlError, XmlErrorDetail};
use crate::names::{element_name, is_textblock, modifier_type_name, modifier_value, node_attrs};
use crate::tree::{InlineLeaf, XmlNode, XmlTree};
use crate::write_tree::{write_fragment, write_tree};

pub const PREVIEW_CHARS: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineScope {
    pub under: Address,
    pub depth: u32,
    pub offset: u32,
    pub limit: u32,
    pub full: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineRow {
    pub path: String,
    pub name: String,
    pub dot: Option<String>,
    pub attrs: Vec<(String, String)>,
    pub preview: Option<String>,
    pub chars: Option<u32>,
    pub children: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineResult {
    pub head: Option<OutlineRow>,
    pub rows: Vec<OutlineRow>,
    pub total: u32,
    pub xml: Option<String>,
}

pub fn outline(tree: &XmlTree, scope: &OutlineScope) -> Result<OutlineResult, XmlError> {
    let path = resolve(&tree.root, &scope.under).ok_or_else(|| {
        XmlError::new(XmlErrorDetail::AddressUnresolved {
            value: scope.under.to_string(),
        })
    })?;
    if scope.full {
        let xml = if path.is_empty() {
            write_tree(tree)?.0
        } else {
            let node = node_at(&tree.root, &path)
                .ok_or_else(|| XmlError::internal("resolved path vanished"))?;
            let types = types_along(&tree.root, &path);
            write_fragment(&[node], &types[..types.len() - 1])?
        };
        return Ok(OutlineResult {
            head: None,
            rows: Vec::new(),
            total: 0,
            xml: Some(xml),
        });
    }
    let mut result = outline_at(tree, &path, scope.depth)?;
    let total = result.rows.len() as u32;
    let start = (scope.offset as usize).min(result.rows.len());
    let end = start
        .saturating_add(scope.limit as usize)
        .min(result.rows.len());
    result.rows = result.rows[start..end].to_vec();
    result.total = total;
    Ok(result)
}

pub fn outline_at(tree: &XmlTree, path: &[usize], depth: u32) -> Result<OutlineResult, XmlError> {
    let Some(node) = node_at(&tree.root, path) else {
        return Ok(OutlineResult {
            head: None,
            rows: Vec::new(),
            total: 0,
            xml: None,
        });
    };
    let head = (!path.is_empty())
        .then(|| row_of(node, path, 0))
        .transpose()?;
    let mut rows = Vec::new();
    let mut child_path: NodePath = path.to_vec();
    collect(node, &mut child_path, 1, depth, &mut rows)?;
    let total = rows.len() as u32;
    Ok(OutlineResult {
        head,
        rows,
        total,
        xml: None,
    })
}

fn collect(
    node: &XmlNode,
    path: &mut NodePath,
    level: u32,
    depth: u32,
    rows: &mut Vec<OutlineRow>,
) -> Result<(), XmlError> {
    for (i, child) in node.block_children().enumerate() {
        path.push(i);
        let folded = if level >= depth {
            child.block_children().count() as u32
        } else {
            0
        };
        rows.push(row_of(child, path, folded)?);
        if level < depth {
            collect(child, path, level + 1, depth, rows)?;
        }
        path.pop();
    }
    Ok(())
}

fn row_of(node: &XmlNode, path: &[usize], folded_children: u32) -> Result<OutlineRow, XmlError> {
    let node_type = node.node.as_type();
    let mut attrs: Vec<(String, String)> = node_attrs(&node.node)
        .into_iter()
        .map(|(k, v)| (format!("attr:{k}"), v))
        .collect();
    for (ty, m) in &node.modifiers {
        if let Some(v) = modifier_value(m) {
            attrs.push((format!("mod:{}", modifier_type_name(*ty)), v));
        }
    }
    for (ty, m) in &node.carry {
        if let Some(v) = modifier_value(m) {
            attrs.push((format!("carry:{}", modifier_type_name(*ty)), v));
        }
    }
    let (preview, chars) = if is_textblock(node_type) {
        let (p, c) = preview_of(node);
        (Some(p), Some(c))
    } else {
        (None, None)
    };
    let name =
        element_name(node_type).ok_or_else(|| XmlError::internal("text node in block position"))?;
    Ok(OutlineRow {
        path: display_path(path),
        name: name.to_owned(),
        dot: node.dot.map(|d| d.to_string()),
        attrs,
        preview,
        chars,
        children: folded_children,
    })
}

fn preview_of(node: &XmlNode) -> (String, u32) {
    let mut chars = 0u32;
    let mut text = String::new();
    for item in node.inline_items() {
        match &item.leaf {
            InlineLeaf::Char(ch) => {
                chars += 1;
                text.push(if matches!(ch, '\n' | '\r' | '\t') {
                    ' '
                } else {
                    *ch
                });
            }
            InlineLeaf::Atom(PlainNode::HardBreak(_) | PlainNode::Tab(_)) => text.push(' '),
            InlineLeaf::Atom(_) => {}
        }
    }
    let mut preview: String = text.chars().take(PREVIEW_CHARS).collect();
    if text.chars().count() > PREVIEW_CHARS {
        preview.push('…');
    }
    (preview, chars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::from_xml;

    fn tree() -> XmlTree {
        let base = crate::writer::encode_base(&[]).unwrap();
        from_xml(&format!(
            "<root dot=\"1_0\" base=\"{base}\" attr:layout_mode=\"continuous\" attr:max_width=\"600\" mod:font_size=\"1200\">\
             <paragraph dot=\"1_1\" mod:alignment=\"center\" carry:font_size=\"1400\">Hello <bold>world</bold> and more text that runs on well past forty characters<hard_break/>next</paragraph>\
             <blockquote dot=\"1_2\" attr:variant=\"left_line\"><paragraph dot=\"1_3\">b</paragraph><paragraph>c</paragraph></blockquote>\
             <table dot=\"1_4\" attr:proportion=\"80\"><table_row dot=\"1_5\"><table_cell dot=\"1_6\"><paragraph dot=\"1_7\">cell</paragraph></table_cell></table_row></table>\
             <horizontal_rule dot=\"1_8\"/><paragraph/></root>"
        ))
        .unwrap()
    }

    fn scope(under: &str, depth: u32) -> OutlineScope {
        OutlineScope {
            under: under.parse().unwrap(),
            depth,
            offset: 0,
            limit: 200,
            full: false,
        }
    }

    #[test]
    fn root_depth_one_lists_direct_children_with_folded_counts() {
        let out = outline(&tree(), &scope("root", 1)).unwrap();
        assert!(out.head.is_none());
        assert_eq!(out.total, 5);
        let rows = &out.rows;
        assert_eq!(rows[0].path, "1");
        assert_eq!(rows[0].name, "paragraph");
        assert_eq!(rows[0].dot.as_deref(), Some("1_1"));
        assert_eq!(
            rows[0].attrs,
            vec![
                ("mod:alignment".to_owned(), "center".to_owned()),
                ("carry:font_size".to_owned(), "1400".to_owned())
            ]
        );
        assert_eq!(
            rows[0].preview.as_deref(),
            Some("Hello world and more text that runs on w…")
        );
        assert_eq!(rows[0].chars, Some(69));
        assert_eq!(rows[0].children, 0);
        assert_eq!(rows[1].path, "2");
        assert_eq!(
            rows[1].attrs,
            vec![("attr:variant".to_owned(), "left_line".to_owned())]
        );
        assert_eq!(rows[1].preview, None);
        assert_eq!(rows[1].chars, None);
        assert_eq!(rows[1].children, 2);
        assert_eq!(rows[2].children, 1);
        assert_eq!(rows[3].name, "horizontal_rule");
        assert_eq!(
            rows[3].attrs,
            vec![("attr:variant".to_owned(), "line".to_owned())]
        );
        assert_eq!(rows[4].dot, None);
        assert_eq!(rows[4].preview.as_deref(), Some(""));
        assert_eq!(rows[4].chars, Some(0));
    }

    #[test]
    fn under_and_depth_walk_the_subtree_in_document_order() {
        let out = outline(&tree(), &scope("1_2", 1)).unwrap();
        let head = out.head.unwrap();
        assert_eq!(head.path, "2");
        assert_eq!(head.dot.as_deref(), Some("1_2"));
        assert_eq!(
            out.rows.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
            ["2.1", "2.2"]
        );
        let deep = outline(&tree(), &scope("3", 3)).unwrap();
        assert_eq!(
            deep.rows
                .iter()
                .map(|r| r.path.as_str())
                .collect::<Vec<_>>(),
            ["3.1", "3.1.1", "3.1.1.1"]
        );
        assert_eq!(deep.rows[2].children, 0);
        let shallow = outline(&tree(), &scope("3", 2)).unwrap();
        assert_eq!(shallow.rows[1].children, 1);
    }

    #[test]
    fn pages_rows_and_reports_the_total() {
        let mut s = scope("root", 8);
        s.offset = 2;
        s.limit = 3;
        let out = outline(&tree(), &s).unwrap();
        assert_eq!(out.total, 10);
        assert_eq!(
            out.rows.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
            ["2.1", "2.2", "3"]
        );
        s.offset = 100;
        assert!(outline(&tree(), &s).unwrap().rows.is_empty());
    }

    #[test]
    fn full_returns_the_subtree_xml_or_the_whole_file() {
        let mut s = scope("1_2", 1);
        s.full = true;
        let out = outline(&tree(), &s).unwrap();
        assert!(out.rows.is_empty());
        assert_eq!(
            out.xml.as_deref(),
            Some(
                "<blockquote dot=\"1_2\" attr:variant=\"left_line\">\n  <paragraph dot=\"1_3\">b</paragraph>\n  <paragraph>c</paragraph>\n</blockquote>\n"
            )
        );
        let mut whole = scope("root", 1);
        whole.full = true;
        assert!(
            outline(&tree(), &whole)
                .unwrap()
                .xml
                .unwrap()
                .starts_with("<root dot=\"1_0\" base=")
        );
    }

    #[test]
    fn an_unresolved_under_is_an_error() {
        let err = outline(&tree(), &scope("9_9", 1)).unwrap_err();
        assert!(
            matches!(*err.detail, XmlErrorDetail::AddressUnresolved { ref value } if value == "9_9")
        );
    }

    #[test]
    fn full_keeps_the_modifiers_the_ancestor_context_allows() {
        let base = crate::writer::encode_base(&[]).unwrap();
        let t = from_xml(&format!(
            "<root dot=\"1_0\" base=\"{base}\" attr:layout_mode=\"continuous\" attr:max_width=\"600\">\
             <paragraph dot=\"1_1\" mod:paragraph_indent=\"100\">a</paragraph><paragraph/></root>"
        ))
        .unwrap();
        let mut s = scope("1_1", 1);
        s.full = true;
        assert_eq!(
            outline(&t, &s).unwrap().xml.as_deref(),
            Some("<paragraph dot=\"1_1\" mod:paragraph_indent=\"100\">a</paragraph>\n")
        );
    }

    #[test]
    fn outline_at_a_childless_block_is_a_head_with_no_rows() {
        let out = outline_at(&tree(), &[0], 1).unwrap();
        let head = out.head.expect("head");
        assert_eq!(head.path, "1");
        assert_eq!(head.name, "paragraph");
        assert_eq!(head.dot.as_deref(), Some("1_1"));
        assert_eq!(head.chars, Some(69));
        assert_eq!(head.children, 0);
        assert!(out.rows.is_empty());
        assert_eq!(out.total, 0);
        assert_eq!(out.xml, None);
    }

    #[test]
    fn full_on_a_textblock_is_that_paragraph_alone() {
        let mut s = scope("1_1", 1);
        s.full = true;
        let out = outline(&tree(), &s).unwrap();
        assert!(out.head.is_none());
        assert!(out.rows.is_empty());
        assert_eq!(
            out.xml.as_deref(),
            Some(
                "<paragraph dot=\"1_1\" mod:alignment=\"center\" carry:font_size=\"1400\">Hello <bold>world</bold> and more text that runs on well past forty characters<hard_break/>next</paragraph>\n"
            )
        );
    }

    #[test]
    fn outline_at_depth_one_folds_only_below_the_listed_children() {
        let out = outline_at(&tree(), &[2], 1).unwrap();
        let head = out.head.expect("head");
        assert_eq!(head.path, "3");
        assert_eq!(head.name, "table");
        assert_eq!(head.children, 0);
        assert_eq!(out.total, 1);
        assert_eq!(out.rows[0].path, "3.1");
        assert_eq!(out.rows[0].name, "table_row");
        assert_eq!(out.rows[0].dot.as_deref(), Some("1_5"));
        assert_eq!(out.rows[0].children, 1);
    }
}
