use editor_crdt::LwwReg;
use editor_macros::NodeAttr;

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct FileNode {
    pub id: LwwReg<Option<String>>,
}
