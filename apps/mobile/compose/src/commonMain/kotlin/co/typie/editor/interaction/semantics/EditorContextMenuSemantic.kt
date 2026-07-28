package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.runtime.EditorContextMenuState

internal class EditorContextMenuSemantic(private val stateProvider: () -> EditorContextMenuState) {
  val visible: Boolean
    get() = stateProvider().visible

  fun show(state: EditorState) {
    stateProvider().show(state)
  }

  fun hide() {
    stateProvider().hide()
  }

  fun requestShowForAppliedSelection(editor: Editor, state: EditorState) {
    stateProvider().requestShowForAppliedSelection(editor = editor, state = state)
  }

  fun onEditorStateChanged(editor: Editor, state: EditorState) {
    stateProvider().onEditorStateChanged(editor = editor, state = state)
  }

  fun reset() {
    stateProvider().reset()
  }
}
