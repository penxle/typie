use editor_crdt::LwwReg;
use editor_macros::NodeAttr;

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct TableCellNode {
    pub col_width: LwwReg<Option<u32>>,
}
