use editor_crdt::LwwReg;
use editor_macros::NodeAttr;

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum ExampleVariant {
    #[default]
    A,
    B,
}

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct ExampleNode {
    pub variant: LwwReg<ExampleVariant>,
}

#[test]
fn macro_generates_attr_enum() {
    let attr = ExampleAttr::Variant(ExampleVariant::B);
    assert!(matches!(attr, ExampleAttr::Variant(ExampleVariant::B)));
}

#[test]
fn macro_generates_plain_struct() {
    let plain = PlainExampleNode {
        variant: ExampleVariant::B,
    };
    let json = serde_json::to_string(&plain).unwrap();
    let parsed: PlainExampleNode = serde_json::from_str(&json).unwrap();
    assert_eq!(plain, parsed);
}

use editor_crdt::Dot;

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct ProportionedNode {
    #[node_attr(default = "1.0f32")]
    pub proportion: LwwReg<f32>,
}

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct MultiFieldNode {
    pub variant: LwwReg<ExampleVariant>,
    #[node_attr(default = "0.5f32")]
    pub size: LwwReg<f32>,
}

#[test]
fn default_uses_lwwreg_default_for_no_override() {
    let n = ExampleNode::default();
    assert_eq!(*n.variant.get(), ExampleVariant::A);
}

#[test]
fn plain_default_uses_t_default_for_no_override() {
    let p = PlainExampleNode::default();
    assert_eq!(p.variant, ExampleVariant::A);
}

#[test]
fn default_override_preserves_non_zero() {
    let n = ProportionedNode::default();
    assert_eq!(*n.proportion.get(), 1.0f32);
    let p = PlainProportionedNode::default();
    assert_eq!(p.proportion, 1.0f32);
}

#[test]
fn multi_field_default_combines_overrides_and_t_default() {
    let n = MultiFieldNode::default();
    assert_eq!(*n.variant.get(), ExampleVariant::A);
    assert_eq!(*n.size.get(), 0.5f32);
    let p = PlainMultiFieldNode::default();
    assert_eq!(p.variant, ExampleVariant::A);
    assert_eq!(p.size, 0.5f32);
}

#[test]
fn apply_attr_sets_winner() {
    let mut n = ExampleNode::default();
    n.apply_attr(Dot::new(1, 0), &ExampleAttr::Variant(ExampleVariant::B))
        .unwrap();
    assert_eq!(*n.variant.get(), ExampleVariant::B);
}

#[test]
fn to_plain_projects_winner() {
    let mut n = ExampleNode::default();
    n.apply_attr(Dot::new(1, 0), &ExampleAttr::Variant(ExampleVariant::B))
        .unwrap();
    assert_eq!(n.to_plain().variant, ExampleVariant::B);
}

#[test]
fn apply_then_to_plain_round_trip() {
    let mut n = MultiFieldNode::default();
    n.apply_attr(Dot::new(1, 0), &MultiFieldAttr::Size(0.75))
        .unwrap();
    let p = n.to_plain();
    assert_eq!(p.variant, ExampleVariant::A);
    assert_eq!(p.size, 0.75);
}
