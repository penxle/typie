package co.typie.editor.interaction.semantics

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

  fun requestShowAfterSelectionCommit() {
    stateProvider().requestShowAfterSelectionCommit()
  }

  fun showAfterSelectionCommitIfRequested(state: EditorState) {
    stateProvider().showAfterSelectionCommitIfRequested(state)
  }

  fun onEditorStateChanged(state: EditorState) {
    stateProvider().onEditorStateChanged(state)
    stateProvider().showAfterSelectionCommitIfRequested(state)
  }
}
