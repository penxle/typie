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
/// **`clock` is per-actor monotonic — *not a strict Lamport timestamp*.**
/// We don't bump our clock when observing remote ops, so cross-actor comparison
/// does not reflect causal precedence. Sufficient for RGA tie-break (deterministic
/// ordering) but reusing this for LWW winner determination requires the op-generation
/// layer to provide Lamport semantics (`L = max(L_self, observed_max) + 1`) separately.
///
/// `Ord` / `PartialOrd` are implemented manually — `derive` depends on field
/// declaration order, which is fragile. Clock-primary is just a setup that *can*
/// evolve into a Lamport-compatible form later; it is not itself a Lamport guarantee
/// (see caveat above).
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
    pub fn new(actor: u64, clock: u64) -> Self {
        Self { actor, clock }
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
