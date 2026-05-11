use editor_macros::ffi;
use serde::{Deserialize, Serialize};

/// The directional bias of a [`Position`](crate::Position) at a boundary.
///
/// Affinity disambiguates which side of a boundary a position belongs to.
/// Its meaning depends on the kind of node that contains the position:
///
/// - **Text node**: determines whether a position between two characters
///   leans toward the preceding char or the following char. Primarily used
///   at soft-wrap boundaries to decide whether a caret is shown at the end
///   of the upper line or at the start of the lower line. The role may
///   be extended to other situations in the future.
/// - **Container node**: when a boundary position must be resolved to a
///   single child node, affinity picks between the preceding and the
///   following child. `Upstream` → `child[offset - 1]` (preceding);
///   `Downstream` → `child[offset]` (following).
#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Affinity {
    /// Bias toward the following content (the next char in a text node,
    /// or `child[offset]` in a container node).
    #[default]
    Downstream,
    /// Bias toward the preceding content (the previous char in a text
    /// node, or `child[offset - 1]` in a container node).
    Upstream,
}

impl PartialOrd for Affinity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Affinity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        fn rank(a: &Affinity) -> u8 {
            match a {
                Affinity::Upstream => 0,
                Affinity::Downstream => 1,
            }
        }
        rank(self).cmp(&rank(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upstream_lt_downstream() {
        assert!(Affinity::Upstream < Affinity::Downstream);
    }

    #[test]
    fn downstream_gt_upstream() {
        assert!(Affinity::Downstream > Affinity::Upstream);
    }

    #[test]
    fn affinity_equal_to_self() {
        assert_eq!(
            Affinity::Upstream.cmp(&Affinity::Upstream),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            Affinity::Downstream.cmp(&Affinity::Downstream),
            std::cmp::Ordering::Equal
        );
    }
}
