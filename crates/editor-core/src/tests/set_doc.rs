use editor_macros::state;

use crate::editor::Editor;

#[test]
fn set_doc_replaces_rendered_doc() {
    let (initial, _p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 5)
    };
    let mut editor = Editor::new_test(initial);
    {
        let view = editor.state().view();
        assert!(editor_state::prose(&view).text().contains("hello"));
    }

    let (other, _p2) = state! {
        doc { root { p2: paragraph { text("world") } } }
        selection: (p2, 0)
    };
    let plain = other.to_plain();

    editor.set_doc(plain);
    let view = editor.state().view();
    let text = editor_state::prose(&view).text().to_string();
    assert!(
        text.contains("world"),
        "set_doc must replace rendered doc: {text:?}"
    );
    assert!(!text.contains("hello"));
}
