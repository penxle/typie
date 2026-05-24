use editor_crdt::LwwReg;
use editor_macros::NodeAttr;

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct TableCellNode {
    pub col_width: LwwReg<Option<u32>>,
    // Tombstone: background_color was moved to Modifier::BackgroundColor.
    // This field exists only to decode legacy wire ops (tag 1) without error.
    // It is never read by rendering code; use explicit_modifiers() instead.
    pub background_color: LwwReg<Option<String>>,
}
