use editor_crdt::LwwReg;
use editor_macros::NodeAttr;

#[derive(Debug, Clone, PartialEq, Eq, NodeAttr, editor_macros::Wire)]
pub struct ImageNode {
    #[wire(n(0))]
    pub id: LwwReg<Option<String>>,
    #[node_attr(default = "100u32")]
    #[plain(ffi(default = "100"), serde(default = "default_proportion"))]
    #[wire(n(1))]
    pub proportion: LwwReg<u32>,
}

fn default_proportion() -> u32 {
    100
}
