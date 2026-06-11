use editor_crdt::{CrdtError, Dot, Text, ToPlain};
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextNode {
    pub text: Text,
}

#[ffi]
#[derive(
    Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize, editor_macros::Wire,
)]
pub enum TextNodeAttr {}

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlainTextNode {
    pub text: String,
}

impl TextNode {
    pub fn to_plain(&self) -> PlainTextNode {
        PlainTextNode {
            text: self.text.to_plain(),
        }
    }

    pub fn apply_attr(&mut self, _id: Dot, attr: &TextNodeAttr) -> Result<(), CrdtError> {
        match *attr {}
    }
}

impl PlainTextNode {
    pub fn to_attrs(&self) -> Vec<TextNodeAttr> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, EntryDot, PlacementId, TextPlacement};

    #[test]
    fn empty_text_node() {
        let t = TextNode::default();
        assert!(t.text.is_empty());
        assert_eq!(t.text.len(), 0);
    }

    #[test]
    fn apply_insert_char_via_wrapper() {
        let mut t = TextNode::default();
        let dot = Dot::new(1, 0);
        t.text = Text::from_visible_placements([TextPlacement {
            placement_id: PlacementId(dot),
            entry_dot: EntryDot(dot),
            ch: 'a',
        }]);
        assert_eq!(t.text.len(), 1);
        assert_eq!(t.text.to_string(), "a");
    }
}
