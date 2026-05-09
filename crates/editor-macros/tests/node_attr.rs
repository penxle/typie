use editor_crdt::LwwReg;
use editor_macros::NodeAttr;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    serde::Serialize,
    serde::Deserialize,
    minicbor::Encode,
    minicbor::Decode,
)]
#[cbor(index_only)]
pub enum ExampleVariant {
    #[default]
    #[n(0)]
    A,
    #[n(1)]
    B,
}

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct ExampleNode {
    pub variant: LwwReg<ExampleVariant>,
}

#[test]
fn macro_generates_attr_enum() {
    let attr = ExampleNodeAttr::Variant(ExampleVariant::B);
    assert!(matches!(attr, ExampleNodeAttr::Variant(ExampleVariant::B)));
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
    #[node_attr(default = "100u32")]
    pub proportion: LwwReg<u32>,
}

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct MultiFieldNode {
    pub variant: LwwReg<ExampleVariant>,
    #[node_attr(default = "50u32")]
    pub size: LwwReg<u32>,
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
    assert_eq!(*n.proportion.get(), 100u32);
    let p = PlainProportionedNode::default();
    assert_eq!(p.proportion, 100u32);
}

#[test]
fn multi_field_default_combines_overrides_and_t_default() {
    let n = MultiFieldNode::default();
    assert_eq!(*n.variant.get(), ExampleVariant::A);
    assert_eq!(*n.size.get(), 50u32);
    let p = PlainMultiFieldNode::default();
    assert_eq!(p.variant, ExampleVariant::A);
    assert_eq!(p.size, 50u32);
}

#[test]
fn apply_attr_sets_winner() {
    let mut n = ExampleNode::default();
    n.apply_attr(Dot::new(1, 0), &ExampleNodeAttr::Variant(ExampleVariant::B))
        .unwrap();
    assert_eq!(*n.variant.get(), ExampleVariant::B);
}

#[test]
fn to_plain_projects_winner() {
    let mut n = ExampleNode::default();
    n.apply_attr(Dot::new(1, 0), &ExampleNodeAttr::Variant(ExampleVariant::B))
        .unwrap();
    assert_eq!(n.to_plain().variant, ExampleVariant::B);
}

#[test]
fn apply_then_to_plain_round_trip() {
    let mut n = MultiFieldNode::default();
    n.apply_attr(Dot::new(1, 0), &MultiFieldNodeAttr::Size(75))
        .unwrap();
    let p = n.to_plain();
    assert_eq!(p.variant, ExampleVariant::A);
    assert_eq!(p.size, 75);
}

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct PlainAttrNode {
    #[plain(serde(default))]
    pub variant: LwwReg<ExampleVariant>,
    #[node_attr(default = "100u32")]
    #[plain(serde(default = "default_size"))]
    pub size: LwwReg<u32>,
}

fn default_size() -> u32 {
    100
}

#[test]
fn plain_serde_default_fills_missing_fields() {
    let p: PlainPlainAttrNode = serde_json::from_str("{}").unwrap();
    assert_eq!(p.variant, ExampleVariant::A);
    assert_eq!(p.size, 100);
}

#[test]
fn plain_serde_default_overrides_partial_payload() {
    let p: PlainPlainAttrNode = serde_json::from_str(r#"{"variant":"B"}"#).unwrap();
    assert_eq!(p.variant, ExampleVariant::B);
    assert_eq!(p.size, 100);
}
