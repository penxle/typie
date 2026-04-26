use editor_macros::context_expr;
use editor_model::{ContextExpr, NodeType};

#[test]
fn any() {
    let expr = context_expr!(Any);
    assert_eq!(expr, ContextExpr::Any);
}

#[test]
fn self_ref() {
    let expr = context_expr!(&);
    assert_eq!(expr, ContextExpr::SelfRef);
}

#[test]
fn globstar() {
    let expr = context_expr!(**);
    assert_eq!(expr, ContextExpr::GlobStar);
}

#[test]
fn single_node() {
    let expr = context_expr!(Paragraph);
    assert_eq!(expr, ContextExpr::Node(NodeType::Paragraph));
}

#[test]
fn direct_child_with_self() {
    // Root > Paragraph > &
    let expr = context_expr!(Root > Paragraph > &);
    assert_eq!(
        expr,
        ContextExpr::Child {
            parent: Box::new(ContextExpr::Child {
                parent: Box::new(ContextExpr::Node(NodeType::Root)),
                child: Box::new(ContextExpr::Node(NodeType::Paragraph)),
            }),
            child: Box::new(ContextExpr::SelfRef),
        }
    );
}

#[test]
fn or_with_self() {
    // (BulletList | OrderedList) > &
    let expr = context_expr!((BulletList | OrderedList) > &);
    assert_eq!(
        expr,
        ContextExpr::Child {
            parent: Box::new(ContextExpr::AnyOf(vec![
                ContextExpr::Node(NodeType::BulletList),
                ContextExpr::Node(NodeType::OrderedList),
            ])),
            child: Box::new(ContextExpr::SelfRef),
        }
    );
}

#[test]
fn not_descendant_of() {
    // !Table > ** > &
    let expr = context_expr!(!Table > ** > &);
    assert_eq!(
        expr,
        ContextExpr::Not(Box::new(ContextExpr::Child {
            parent: Box::new(ContextExpr::Child {
                parent: Box::new(ContextExpr::Node(NodeType::Table)),
                child: Box::new(ContextExpr::GlobStar),
            }),
            child: Box::new(ContextExpr::SelfRef),
        }))
    );
}

#[test]
fn modifier_context() {
    // (Paragraph | Callout) > Text
    let expr = context_expr!((Paragraph | Callout) > Text);
    assert_eq!(
        expr,
        ContextExpr::Child {
            parent: Box::new(ContextExpr::AnyOf(vec![
                ContextExpr::Node(NodeType::Paragraph),
                ContextExpr::Node(NodeType::Callout),
            ])),
            child: Box::new(ContextExpr::Node(NodeType::Text)),
        }
    );
}

#[test]
fn not_descendant_matching() {
    use editor_model::NodeType::*;

    let expr = context_expr!(!Table > ** > &);
    assert!(expr.matches(&[Root, Paragraph, Text]));
    assert!(!expr.matches(&[Root, Table, TableRow, TableCell, Paragraph, Text]));
}
