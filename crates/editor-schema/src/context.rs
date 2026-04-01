use editor_model::NodeType;

#[derive(Debug, Clone, PartialEq)]
pub enum ContextExpr {
    Any,
    SelfRef,
    Node(NodeType),
    GlobStar,
    Child {
        parent: Box<ContextExpr>,
        child: Box<ContextExpr>,
    },
    AnyOf(Vec<ContextExpr>),
    Not(Box<ContextExpr>),
}

impl ContextExpr {
    pub fn matches(&self, path: &[NodeType]) -> bool {
        match self {
            ContextExpr::Not(expr) => !expr.matches(path),
            _ => (0..=path.len()).any(|start| self.matches_exact(&path[start..])),
        }
    }

    fn matches_exact(&self, path: &[NodeType]) -> bool {
        match self {
            ContextExpr::Any => true,
            ContextExpr::SelfRef => path.len() == 1,
            ContextExpr::Node(nt) => path.len() == 1 && path[0] == *nt,
            ContextExpr::GlobStar => true,
            ContextExpr::AnyOf(exprs) => exprs.iter().any(|e| e.matches_exact(path)),
            ContextExpr::Not(expr) => !expr.matches(path),
            ContextExpr::Child { parent, child } => (0..=path.len()).any(|split| {
                parent.matches_exact(&path[..split]) && child.matches_exact(&path[split..])
            }),
        }
    }
}

impl Default for ContextExpr {
    fn default() -> Self {
        ContextExpr::Any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_model::NodeType::*;

    #[test]
    fn any_matches_everything() {
        assert!(ContextExpr::Any.matches(&[]));
        assert!(ContextExpr::Any.matches(&[Root, Paragraph, Text]));
    }

    #[test]
    fn node_matches_single_element() {
        let expr = ContextExpr::Node(Paragraph);
        assert!(expr.matches(&[Root, Paragraph]));
        assert!(expr.matches(&[Paragraph]));
        assert!(!expr.matches(&[Root, Text]));
        assert!(!expr.matches(&[]));
    }

    #[test]
    fn self_ref_matches_any_single() {
        assert!(ContextExpr::SelfRef.matches(&[Text]));
        assert!(ContextExpr::SelfRef.matches(&[Root, Paragraph]));
        assert!(!ContextExpr::SelfRef.matches(&[]));
    }

    #[test]
    fn child_direct_chain_with_self() {
        // Root > Paragraph > &
        let expr = ContextExpr::Child {
            parent: Box::new(ContextExpr::Child {
                parent: Box::new(ContextExpr::Node(Root)),
                child: Box::new(ContextExpr::Node(Paragraph)),
            }),
            child: Box::new(ContextExpr::SelfRef),
        };
        assert!(expr.matches(&[Root, Paragraph, Text]));
        assert!(!expr.matches(&[Blockquote, Paragraph, Text]));
    }

    #[test]
    fn any_of_alternatives() {
        // (BulletList | OrderedList) > &
        let expr = ContextExpr::Child {
            parent: Box::new(ContextExpr::AnyOf(vec![
                ContextExpr::Node(BulletList),
                ContextExpr::Node(OrderedList),
            ])),
            child: Box::new(ContextExpr::SelfRef),
        };
        assert!(expr.matches(&[Root, BulletList, ListItem]));
        assert!(expr.matches(&[Root, OrderedList, ListItem]));
        assert!(!expr.matches(&[Root, Paragraph, ListItem]));
    }

    #[test]
    fn globstar_matches_any_chain() {
        // Table > ** > &
        let expr = ContextExpr::Child {
            parent: Box::new(ContextExpr::Child {
                parent: Box::new(ContextExpr::Node(Table)),
                child: Box::new(ContextExpr::GlobStar),
            }),
            child: Box::new(ContextExpr::SelfRef),
        };
        assert!(expr.matches(&[Table, TableRow]));
        assert!(expr.matches(&[Root, Table, TableRow, TableCell, Paragraph, Text]));
        assert!(!expr.matches(&[Root, Paragraph, Text]));
    }

    #[test]
    fn not_descendant_of() {
        // !Table > ** > &
        let expr = ContextExpr::Not(Box::new(ContextExpr::Child {
            parent: Box::new(ContextExpr::Child {
                parent: Box::new(ContextExpr::Node(Table)),
                child: Box::new(ContextExpr::GlobStar),
            }),
            child: Box::new(ContextExpr::SelfRef),
        }));
        assert!(expr.matches(&[Root, Paragraph, Text]));
        assert!(!expr.matches(&[Root, Table, TableRow, TableCell, Paragraph, Text]));
    }

    #[test]
    fn modifier_context_without_self_ref() {
        // (Paragraph | Callout) > Text
        let expr = ContextExpr::Child {
            parent: Box::new(ContextExpr::AnyOf(vec![
                ContextExpr::Node(Paragraph),
                ContextExpr::Node(Callout),
            ])),
            child: Box::new(ContextExpr::Node(Text)),
        };
        assert!(expr.matches(&[Root, Paragraph, Text]));
        assert!(expr.matches(&[Root, Callout, Text]));
        assert!(!expr.matches(&[Root, Blockquote, Text]));
    }
}
