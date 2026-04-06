use editor_common::StrExt;
use editor_model::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlatSegment<'a> {
    Text { node_id: NodeId, text: &'a str },
    Break { node_id: NodeId },
    Atom { node_id: NodeId },
}

impl FlatSegment<'_> {
    pub fn size(&self) -> usize {
        match self {
            FlatSegment::Text { text, .. } => text.char_count(),
            FlatSegment::Break { .. } | FlatSegment::Atom { .. } => 1,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            FlatSegment::Text { text, .. } => text,
            FlatSegment::Break { .. } => "\n",
            FlatSegment::Atom { .. } => "\u{fffc}",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_segment_text_size_counts_chars() {
        let node_id = NodeId::new();
        let seg = FlatSegment::Text {
            node_id,
            text: "한글",
        };
        assert_eq!(seg.size(), 2);
    }

    #[test]
    fn flat_segment_break_size_is_one() {
        let seg = FlatSegment::Break {
            node_id: NodeId::new(),
        };
        assert_eq!(seg.size(), 1);
    }

    #[test]
    fn flat_segment_atom_size_is_one() {
        let seg = FlatSegment::Atom {
            node_id: NodeId::new(),
        };
        assert_eq!(seg.size(), 1);
    }

    #[test]
    fn flat_segment_as_str_break_is_newline() {
        let seg = FlatSegment::Break {
            node_id: NodeId::new(),
        };
        assert_eq!(seg.as_str(), "\n");
    }

    #[test]
    fn flat_segment_as_str_atom_is_replacement_char() {
        let seg = FlatSegment::Atom {
            node_id: NodeId::new(),
        };
        assert_eq!(seg.as_str(), "\u{fffc}");
    }
}
