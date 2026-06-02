use editor_macros::state;

use crate::editor::Editor;

#[test]
fn set_doc_replaces_rendered_doc() {
    let (initial, _t) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 5)
    };
    let mut editor = Editor::new_test(initial);
    assert!(editor.state().doc.extract_text().contains("hello"));

    let (other, _t2) = state! {
        doc { root { paragraph { t2: text("world") } } }
        selection: (t2, 0)
    };
    let plain = other.doc.to_plain();

    editor.set_doc(plain);
    let text = editor.state().doc.extract_text();
    assert!(
        text.contains("world"),
        "set_doc must replace rendered doc: {text:?}"
    );
    assert!(!text.contains("hello"));
}
