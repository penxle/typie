use editor_common::Tri;
use editor_macros::ModifierState;
use serde::{Deserialize, Serialize};

#[derive(ModifierState, Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Demo {
    Bold,
    FontSize { value: u32 },
    Link { href: String },
}

#[test]
fn wrapper_structs_exist_with_fields() {
    let _: Demo = Demo::Bold;
    let v = FontSizeValue { value: 1600 };
    assert_eq!(v.value, 1600);
    let l = LinkValue {
        href: "https://example.com".into(),
    };
    assert_eq!(l.href, "https://example.com");
}

#[test]
fn modifier_state_struct_has_fields_with_tri() {
    let s = DemoState::default();
    assert_eq!(s.bold, Tri::Absent);
    assert_eq!(s.font_size, Tri::Absent);
    assert_eq!(s.link, Tri::Absent);
}
