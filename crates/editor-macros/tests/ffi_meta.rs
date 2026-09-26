use editor_bindgen::meta::{FfiKind, FfiMeta};
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum AdjacentExample {
    Flag(bool),
    Empty,
}

#[ffi]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InternalExample {
    Empty,
}

fn decode(bytes: &[u8]) -> FfiMeta {
    let len = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
    bitcode::decode(&bytes[4..4 + len]).unwrap()
}

#[test]
fn adjacently_tagged_enum_records_its_content_key() {
    let meta = decode(&FFI_META_editor_macros_AdjacentExample);
    assert_eq!(meta.serde_rename_all.as_deref(), Some("snake_case"));
    match meta.kind {
        FfiKind::Enum {
            serde_tag,
            serde_content,
            ..
        } => {
            assert_eq!(serde_tag.as_deref(), Some("type"));
            assert_eq!(serde_content.as_deref(), Some("value"));
        }
        other => panic!("expected an enum, got {other:?}"),
    }
}

#[test]
fn internally_tagged_enum_has_no_content_key() {
    match decode(&FFI_META_editor_macros_InternalExample).kind {
        FfiKind::Enum {
            serde_tag,
            serde_content,
            ..
        } => {
            assert_eq!(serde_tag.as_deref(), Some("type"));
            assert_eq!(serde_content, None);
        }
        other => panic!("expected an enum, got {other:?}"),
    }
}
