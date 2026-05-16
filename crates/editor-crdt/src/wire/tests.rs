use super::*;
use crate::Dot;
use crate::wire::{DecCtx, EncCtx};
use editor_macros::Wire;

#[derive(Debug, PartialEq, Eq, Wire)]
enum Color {
    #[wire(n(0))]
    Red,
    #[wire(n(1))]
    Green,
    #[wire(n(2))]
    Blue,
}

#[derive(Debug, PartialEq, Eq, Wire)]
enum Shape {
    #[wire(n(0))]
    Circle {
        #[wire(n(0))]
        radius: u32,
    },
    #[wire(n(1))]
    Square {
        #[wire(n(0))]
        side: u32,
        #[wire(n(1))]
        color: Color,
    },
}

#[derive(Debug, PartialEq, Eq, Wire)]
enum Tagged {
    #[wire(n(0))]
    Empty,
    #[wire(n(1))]
    Single(#[wire(n(0))] u32),
    #[wire(n(2))]
    Pair(#[wire(n(0))] u32, #[wire(n(1))] u32),
}

#[test]
fn enum_unit_variants_round_trip() {
    let ec = EncCtx::from_table(&[], vec![]);
    let dc = DecCtx {
        actor_table: vec![],
        baselines: vec![],
    };
    let mut buf = Vec::new();
    Color::Green.encode(&ec, &mut buf).unwrap();
    let mut slice = &buf[..];
    let got = Color::decode(&dc, &mut slice).unwrap();
    assert_eq!(got, Color::Green);
}

#[test]
fn enum_named_variant_round_trip() {
    let ec = EncCtx::from_table(&[], vec![]);
    let dc = DecCtx {
        actor_table: vec![],
        baselines: vec![],
    };
    let s = Shape::Square {
        side: 7,
        color: Color::Blue,
    };
    let mut buf = Vec::new();
    s.encode(&ec, &mut buf).unwrap();
    let mut slice = &buf[..];
    let got = Shape::decode(&dc, &mut slice).unwrap();
    assert_eq!(got, s);
}

#[test]
fn enum_tuple_variant_round_trip() {
    let ec = EncCtx::from_table(&[], vec![]);
    let dc = DecCtx {
        actor_table: vec![],
        baselines: vec![],
    };
    let cases = [Tagged::Empty, Tagged::Single(42), Tagged::Pair(1, 2)];
    for v in &cases {
        let mut buf = Vec::new();
        v.encode(&ec, &mut buf).unwrap();
        let mut slice = &buf[..];
        let got = Tagged::decode(&dc, &mut slice).unwrap();
        assert_eq!(&got, v);
    }
}

#[test]
fn unknown_variant_tag_errors() {
    let dc = DecCtx {
        actor_table: vec![],
        baselines: vec![],
    };
    let bad = vec![99u8];
    let mut slice = &bad[..];
    let err = Color::decode(&dc, &mut slice).unwrap_err();
    assert!(matches!(err, crate::wire::WireError::UnknownVariant { .. }));
}

#[derive(Debug, PartialEq, Eq, Wire)]
struct Point {
    #[wire(n(0))]
    x: u32,
    #[wire(n(1))]
    y: u32,
}

#[derive(Debug, PartialEq, Eq, Wire)]
#[wire(transparent)]
struct UserId(u64);

#[derive(Debug, PartialEq, Eq, Wire)]
struct WithSkip {
    #[wire(n(0))]
    kept: u32,
    #[wire(skip)]
    _cached: u64,
}

impl Default for WithSkip {
    fn default() -> Self {
        Self {
            kept: 0,
            _cached: 0,
        }
    }
}

#[test]
fn struct_named_round_trip() {
    let ec = EncCtx::from_table(&[], vec![]);
    let dc = DecCtx {
        actor_table: vec![],
        baselines: vec![],
    };
    let p = Point { x: 7, y: 11 };
    let mut buf = Vec::new();
    p.encode(&ec, &mut buf).unwrap();
    let mut slice = &buf[..];
    let got = Point::decode(&dc, &mut slice).unwrap();
    assert_eq!(got, p);
}

#[test]
fn transparent_newtype_round_trip() {
    let ec = EncCtx::from_table(&[], vec![]);
    let dc = DecCtx {
        actor_table: vec![],
        baselines: vec![],
    };
    let id = UserId(123_456);
    let mut buf = Vec::new();
    id.encode(&ec, &mut buf).unwrap();
    let mut slice = &buf[..];
    let got = UserId::decode(&dc, &mut slice).unwrap();
    assert_eq!(got, id);
}

#[test]
fn skip_field_uses_default_on_decode() {
    let ec = EncCtx::from_table(&[], vec![]);
    let dc = DecCtx {
        actor_table: vec![],
        baselines: vec![],
    };
    let v = WithSkip {
        kept: 42,
        _cached: 999,
    };
    let mut buf = Vec::new();
    v.encode(&ec, &mut buf).unwrap();
    let mut slice = &buf[..];
    let got = WithSkip::decode(&dc, &mut slice).unwrap();
    assert_eq!(got.kept, 42);
    assert_eq!(got._cached, 0);
}

#[test]
fn empty_dots_round_trip() {
    let bytes = encode_dots(&[]).unwrap();
    assert!(bytes.is_empty());
    let decoded = decode_dots(&bytes).unwrap();
    assert!(decoded.is_empty());
}

#[test]
fn dots_round_trip() {
    let dots = vec![Dot::new(7, 10), Dot::new(99, 3), Dot::new(7, 12)];
    let bytes = encode_dots(&dots).unwrap();
    let decoded = decode_dots(&bytes).unwrap();
    assert_eq!(decoded, dots);
}
