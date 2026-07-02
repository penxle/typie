use editor_common::Ffi;
use editor_macros::ffi;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// CRDT-standard `(actor, clock)` identity. Used as char-id here; meant to be reused
/// across future primitives (sequence-CRDT, OR-Set add tokens, LWW register timestamps,
/// op identity).
///
/// **`clock` is per-actor monotonic, with Lamport-style advancement owned by
/// the op-generation layer.** Raw `Dot` construction does not inspect causal
/// context; it is just a pair. `OpGraph` advances its local next clock past
/// received remote ops before authoring later local ops, so dots created through
/// `OpGraph::add` after `receive_changeset` are causally later than the received
/// clock. Cross-actor comparison is still only a deterministic total order unless
/// the op was generated through that clock-management layer.
///
/// `Ord` / `PartialOrd` are implemented manually — `derive` depends on field
/// declaration order, which is fragile. Clock-primary matches the ordering used
/// by the clock-management layer, but the Lamport guarantee comes from `OpGraph`,
/// not from constructing a `Dot` by hand.
///
/// Serializes as the string `"{base62(actor)}_{base62(clock)}"`. The actor field is
/// a randomly-generated u64 and routinely overflows JS Number's 53-bit safe range,
/// so a struct shape would break round-tripping through serde-wasm-bindgen and
/// JSON.stringify on the JS side.
#[ffi(custom(String))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Dot {
    pub actor: u64,
    pub clock: u64,
}

impl Dot {
    /// Top `clock` bit, reserved to mark a *synthetic* dot — one that names a
    /// projection-synthesized node (scaffolded to satisfy the schema) rather than
    /// a real authored op. Real ops never set it: `clock` is a per-actor Lamport
    /// counter that cannot reach `2^63`.
    const SYNTHETIC_BIT: u64 = 1 << 63;

    /// The canonical, document-wide root id (the document-wide root identity).
    /// The root is implicit — never a stored op — so it is a fixed synthetic dot:
    /// always present, the same on every replica, and not a deletable seq op
    /// (`as_op_dot` is `None`). Block-modifier ops may still target it because it
    /// is a permanent anchor, unlike transient scaffolded synthetic dots.
    pub const ROOT: Dot = Dot {
        actor: 0,
        clock: Self::SYNTHETIC_BIT,
    };

    pub fn new(actor: u64, clock: u64) -> Self {
        Self { actor, clock }
    }

    /// `true` if this dot names a projection-synthesized node (no authored op).
    pub fn is_synthetic(&self) -> bool {
        self.clock & Self::SYNTHETIC_BIT != 0
    }

    /// Narrows to an [`OpDot`] iff this is a real authored op dot (not synthetic).
    /// The edit layer requires an `OpDot` to target a CRDT op, so a synthesized
    /// node must be materialized first.
    pub fn as_op_dot(self) -> Option<OpDot> {
        (!self.is_synthetic()).then_some(OpDot(self))
    }

    /// Builds a synthetic dot from a 128-bit content hash of the node's
    /// (parent, slot, role). Deterministic across replicas (same inputs → same
    /// dot), distinct from every real op dot (synthetic bit set), and distinct
    /// from other synthesized nodes (127-bit hash space).
    pub fn synthetic(hash: u128) -> Self {
        Self {
            actor: (hash >> 64) as u64,
            clock: (hash as u64) | Self::SYNTHETIC_BIT,
        }
    }
}

/// A [`Dot`] proven to name a real authored op (not synthetic). Obtain via
/// [`Dot::as_op_dot`]; required wherever a CRDT op targets an element, so the
/// type system prevents applying ops to projection-synthesized nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OpDot(Dot);

impl OpDot {
    pub fn dot(self) -> Dot {
        self.0
    }
}

impl From<OpDot> for Dot {
    fn from(op: OpDot) -> Self {
        op.0
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid Dot")]
pub struct ParseDotError;

impl fmt::Display for Dot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}_{}",
            base62::encode_fmt(self.actor),
            base62::encode_fmt(self.clock)
        )
    }
}

impl FromStr for Dot {
    type Err = ParseDotError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (actor_s, clock_s) = s.split_once('_').ok_or(ParseDotError)?;
        let actor = base62::decode(actor_s).map_err(|_| ParseDotError)?;
        let clock = base62::decode(clock_s).map_err(|_| ParseDotError)?;
        let actor = u64::try_from(actor).map_err(|_| ParseDotError)?;
        let clock = u64::try_from(clock).map_err(|_| ParseDotError)?;
        Ok(Self { actor, clock })
    }
}

impl Serialize for Dot {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Dot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Ffi for Dot {
    type Target = String;
    type Error = ParseDotError;

    fn to_ffi(&self) -> String {
        self.to_string()
    }

    fn from_ffi(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dots(pub Vec<Dot>);

impl Ord for Dot {
    fn cmp(&self, other: &Self) -> Ordering {
        self.clock
            .cmp(&other.clock)
            .then_with(|| self.actor.cmp(&other.actor))
    }
}

impl PartialOrd for Dot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_dots_are_op_dots_not_synthetic() {
        let d = Dot::new(12345, 678);
        assert!(!d.is_synthetic());
        assert_eq!(d.as_op_dot().map(|o| o.dot()), Some(d));
    }

    #[test]
    fn synthetic_dot_is_synthetic_and_not_an_op_dot() {
        let s = Dot::synthetic(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210);
        assert!(s.is_synthetic());
        assert!(s.as_op_dot().is_none());
    }

    #[test]
    fn root_is_a_stable_synthetic_anchor() {
        assert!(
            Dot::ROOT.is_synthetic(),
            "root is implicit, not an authored op"
        );
        assert!(
            Dot::ROOT.as_op_dot().is_none(),
            "root is not a deletable seq op"
        );
        assert_eq!(Dot::ROOT, Dot::ROOT, "canonical: same on every replica");
    }

    #[test]
    fn root_string_form_is_pinned_for_web_client() {
        // The web client hardcodes this exact string to target the root in set_attrs
        // messages (apps/website/src/lib/editor-ffi/root-attrs.ts, ROOT_ID). If this
        // assertion fails the encoding changed; update that constant too, or root-only
        // ops (layout mode) fail to deserialize with "invalid Dot".
        assert_eq!(Dot::ROOT.to_string(), "0_AzL8n0Y58m8");
        assert_eq!("0_AzL8n0Y58m8".parse::<Dot>().unwrap(), Dot::ROOT);
    }

    #[test]
    fn synthetic_is_deterministic_and_distinct() {
        let a = Dot::synthetic(1);
        let b = Dot::synthetic(1);
        let c = Dot::synthetic(2);
        assert_eq!(a, b, "same hash → same synthetic dot");
        assert_ne!(a, c, "different hash → different synthetic dot");
    }

    #[test]
    fn synthetic_never_collides_with_a_max_clock_real_dot() {
        // Real clocks are Lamport counters that never reach 2^63, so even a huge
        // real clock has the synthetic bit clear and stays an op dot.
        let big_real = Dot::new(u64::MAX, (1u64 << 63) - 1);
        assert!(!big_real.is_synthetic());
        assert!(big_real.as_op_dot().is_some());
    }

    #[test]
    fn ord_clock_primary() {
        let lo_actor_high_clock = Dot::new(1, 10);
        let hi_actor_low_clock = Dot::new(5, 2);
        assert!(hi_actor_low_clock < lo_actor_high_clock);
    }

    #[test]
    fn ord_actor_tie_break_on_equal_clock() {
        let x = Dot::new(1, 3);
        let y = Dot::new(2, 3);
        assert!(x < y);
    }

    #[test]
    fn ord_same_actor_by_clock() {
        let a = Dot::new(5, 1);
        let b = Dot::new(5, 7);
        assert!(a < b);
    }

    #[test]
    fn string_roundtrip_preserves_full_u64_range() {
        // Actor is randomly generated u64 and routinely exceeds JS Number's safe range,
        // so the string repr must roundtrip values outside the 2^53 window.
        let original = Dot::new(4_264_341_739_882_772_773, 7);
        let s = original.to_string();
        let parsed: Dot = s.parse().unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn serde_string_roundtrip() {
        let original = Dot::new(u64::MAX, u64::MAX - 1);
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.starts_with('"') && json.ends_with('"'));
        let parsed: Dot = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn wire_roundtrip() {
        use crate::wire::{CollectCtx, DecCtx, EncCtx, Wire};
        let original = Dot::new(42, 1234);
        let mut cc = CollectCtx::new();
        original.collect(&mut cc);
        let (table, baselines) = cc.finalize();
        let ec = EncCtx::from_table(&table, baselines.clone());
        let dc = DecCtx {
            actor_table: table,
            baselines,
        };
        let mut buf = Vec::new();
        original.encode(&ec, &mut buf).unwrap();
        let mut slice = &buf[..];
        let decoded = Dot::decode(&dc, &mut slice).unwrap();
        assert_eq!(original, decoded);
    }
}
