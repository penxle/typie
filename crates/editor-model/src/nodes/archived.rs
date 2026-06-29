use editor_crdt::LwwReg;
use editor_macros::NodeAttr;

#[derive(Debug, Clone, PartialEq, Eq, NodeAttr, editor_macros::Wire)]
pub struct ArchivedNode {
    #[wire(n(0))]
    pub id: LwwReg<Option<String>>,
}
