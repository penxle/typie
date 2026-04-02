use crate::editor::Editor;
use crate::message::*;

pub fn handle_formatting_intent(_editor: &mut Editor, _intent: FormattingIntent) {
    // stub
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;

    #[test]
    fn unimplemented_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state.clone());
        editor.apply(Message::Intent(Intent::Formatting(
            FormattingIntent::ClearModifiers,
        )));
        assert_eq!(editor.state().selection, state.selection);
    }
}
