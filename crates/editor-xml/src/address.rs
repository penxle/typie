use std::fmt;
use std::str::FromStr;

use editor_crdt::Dot;
use editor_model::NodeType;

use crate::error::XmlErrorDetail;
use crate::tree::{XmlChild, XmlNode};

pub type NodePath = Vec<usize>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Address {
    Root,
    Dot(Dot),
    Path(Vec<usize>),
}

impl FromStr for Address {
    type Err = XmlErrorDetail;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || XmlErrorDetail::AddressInvalid {
            value: s.to_owned(),
        };
        if s == "root" {
            return Ok(Address::Root);
        }
        if s.contains('_') {
            return Dot::from_str(s).map(Address::Dot).map_err(|_| invalid());
        }
        if s.is_empty() {
            return Err(invalid());
        }
        let mut path = Vec::new();
        for part in s.split('.') {
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return Err(invalid());
            }
            let n: usize = part.parse().map_err(|_| invalid())?;
            if n == 0 {
                return Err(invalid());
            }
            path.push(n);
        }
        Ok(Address::Path(path))
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Root => f.write_str("root"),
            Address::Dot(dot) => write!(f, "{dot}"),
            Address::Path(path) => {
                let parts: Vec<String> = path.iter().map(|n| n.to_string()).collect();
                f.write_str(&parts.join("."))
            }
        }
    }
}

pub fn display_path(path: &[usize]) -> String {
    if path.is_empty() {
        return "root".to_owned();
    }
    Address::Path(path.iter().map(|i| i + 1).collect()).to_string()
}

pub fn address_of(node: &XmlNode, path: &[usize]) -> String {
    match node.dot {
        Some(dot) => dot.to_string(),
        None => display_path(path),
    }
}

pub fn block_positions(node: &XmlNode) -> Vec<usize> {
    node.children
        .iter()
        .enumerate()
        .filter_map(|(i, c)| matches!(c, XmlChild::Block(_)).then_some(i))
        .collect()
}

pub fn types_along(root: &XmlNode, path: &[usize]) -> Vec<NodeType> {
    let mut out = vec![root.node.as_type()];
    let mut node = root;
    for &i in path {
        node = node.block_children().nth(i).expect("resolved path");
        out.push(node.node.as_type());
    }
    out
}

pub fn node_at<'a>(root: &'a XmlNode, path: &[usize]) -> Option<&'a XmlNode> {
    let mut node = root;
    for &index in path {
        node = node.block_children().nth(index)?;
    }
    Some(node)
}

pub fn node_at_mut<'a>(root: &'a mut XmlNode, path: &[usize]) -> Option<&'a mut XmlNode> {
    let mut node = root;
    for &index in path {
        let slot = *block_positions(node).get(index)?;
        node = match &mut node.children[slot] {
            XmlChild::Block(b) => b,
            XmlChild::Inline(_) => return None,
        };
    }
    Some(node)
}

pub fn resolve(root: &XmlNode, address: &Address) -> Option<NodePath> {
    match address {
        Address::Root => Some(Vec::new()),
        Address::Path(ordinals) => {
            let path: NodePath = ordinals
                .iter()
                .map(|n| n.checked_sub(1))
                .collect::<Option<_>>()?;
            node_at(root, &path).map(|_| path)
        }
        Address::Dot(dot) => {
            fn walk(node: &XmlNode, dot: Dot, path: &mut NodePath) -> bool {
                if node.dot == Some(dot) {
                    return true;
                }
                for (i, child) in node.block_children().enumerate() {
                    path.push(i);
                    if walk(child, dot, path) {
                        return true;
                    }
                    path.pop();
                }
                false
            }
            let mut path = Vec::new();
            walk(root, *dot, &mut path).then_some(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::from_xml;
    use crate::tree::XmlTree;

    fn tree() -> XmlTree {
        let base = crate::writer::encode_base(&[]).unwrap();
        from_xml(&format!(
            "<root dot=\"1_0\" base=\"{base}\" attr:layout_mode=\"continuous\" attr:max_width=\"600\">\
             <paragraph dot=\"1_1\">a</paragraph>\
             <blockquote dot=\"1_2\"><paragraph dot=\"1_3\">b</paragraph><paragraph>c</paragraph></blockquote>\
             <paragraph/></root>"
        ))
        .unwrap()
    }

    #[test]
    fn parses_root_dot_and_ordinal_path() {
        assert_eq!("root".parse::<Address>().unwrap(), Address::Root);
        assert_eq!(
            "1_3".parse::<Address>().unwrap(),
            Address::Dot("1_3".parse().unwrap())
        );
        assert_eq!("2.2".parse::<Address>().unwrap(), Address::Path(vec![2, 2]));
        for bad in ["", "0", "2.0", "a.b", "1_", "x", " 2"] {
            assert!(
                matches!(
                    bad.parse::<Address>(),
                    Err(XmlErrorDetail::AddressInvalid { .. })
                ),
                "{bad}"
            );
        }
    }

    #[test]
    fn displays_paths_and_addresses() {
        assert_eq!(display_path(&[]), "root");
        assert_eq!(display_path(&[1, 1]), "2.2");
        assert_eq!(Address::Path(vec![2, 2]).to_string(), "2.2");
        assert_eq!(Address::Root.to_string(), "root");
    }

    #[test]
    fn resolves_every_address_form_to_the_same_node() {
        let t = tree();
        assert_eq!(resolve(&t.root, &Address::Root), Some(vec![]));
        assert_eq!(resolve(&t.root, &"1_3".parse().unwrap()), Some(vec![1, 0]));
        assert_eq!(resolve(&t.root, &"2.1".parse().unwrap()), Some(vec![1, 0]));
        assert_eq!(resolve(&t.root, &"2.2".parse().unwrap()), Some(vec![1, 1]));
        assert_eq!(resolve(&t.root, &"9_9".parse().unwrap()), None);
        assert_eq!(resolve(&t.root, &"2.3".parse().unwrap()), None);
        assert_eq!(resolve(&t.root, &"1.1".parse().unwrap()), None);
        assert_eq!(resolve(&t.root, &Address::Path(vec![0])), None);
        let node = node_at(&t.root, &[1, 1]).unwrap();
        assert_eq!(node.dot, None);
        assert_eq!(address_of(node, &[1, 1]), "2.2");
        assert_eq!(
            address_of(node_at(&t.root, &[1, 0]).unwrap(), &[1, 0]),
            "1_3"
        );
        assert_eq!(block_positions(&t.root), vec![0, 1, 2]);
        assert_eq!(
            block_positions(node_at(&t.root, &[0]).unwrap()),
            Vec::<usize>::new()
        );
    }
}
