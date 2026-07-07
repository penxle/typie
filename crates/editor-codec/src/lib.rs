//! Wire/storage 포맷 v2의 코덱 코어.
//!
//! 이 크레이트 안의 상수·타입 레이아웃·태그는 저장/전송되는 바이트의 계약이다.
//! 레이아웃에 영향을 주는 변경은 스키마 진화 규약(append-only, 태그 재사용 금지,
//! frozen/closed 불변)을 따라야 한다.
//!
//! ## 확장 분류 규범 (새 태그 추가 시 필수 판정)
//!
//! > "구 리더가 이것을 보존-무시했을 때, 그 리더의 로컬 편집이 신 리더와
//! > 위치 산술/해석에서 어긋날 수 있는가?"
//!
//! - 아니오 → feature bit 불필요(open enum 태그 추가만): 새 attr·modifier·
//!   node type·dot-앵커 오버레이 op·새 item 종류(1-슬롯 계약 하).
//! - 예 → required feature bit 의무: 위치 산술 개입 seq op(예: Move),
//!   replay 규칙 변경, baseline/epoch.
//! - optional bit는 관측용 자유.
//! - 새 attr(및 그 payload 값 타입)는 Dot-free 폐쇄 안에 있어야 한다 — attr 바이트의
//!   ctx-독립(무손실 런타임 캐리어·자유 재인코딩)이 이 성질에 기댄다. 스키마 테스트
//!   attr_type_universe_is_dot_free가 기계 강제한다.
//!
//! 새 node type 승격 기준 2종: (1) 기존 item kind로 표현 불가능한 구조 동작은
//! open 태그로 도입 불가 — 새 item kind 또는 required bit, (2) 수리 불가능한
//! 불변식을 가진 노드는 required bit로 승격.

extern crate self as editor_codec;

pub mod bundle;
pub mod consolidate;
pub mod convert;
pub mod ctx;
pub mod durable;
pub mod envelope;
pub mod error;
pub mod framing;
pub mod primitives;
pub mod registry;
pub mod schema;
pub mod types;
pub mod varint;

pub use bundle::{
    bundle_contains_unknown, bundle_stream_contains_unknown, decode_dots, encode_dots,
    split_bundle_bytes,
};
pub use consolidate::{Consolidation, consolidate_stream};
pub use convert::{
    Decoded, ReencodableChangesets, changesets_contain_unknown, decode_changeset_stream,
    decode_changesets, encode_changesets,
};
pub use error::{CodecError, CodecResult, Corruption, EncodeInvariant, Fenced};
