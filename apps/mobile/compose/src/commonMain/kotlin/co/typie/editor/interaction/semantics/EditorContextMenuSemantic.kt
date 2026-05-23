package co.typie.editor.interaction.semantics

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.editor.EditorState
import co.typie.editor.ffi.Selection

internal class EditorContextMenuSemantic {
  var visible: Boolean by mutableStateOf(false)
    private set

  private var shownForSelection: Selection? = null
  private var showAfterSelectionCommit = false

  fun show(state: EditorState) {
    showAfterSelectionCommit = false
    shownForSelection = state.selection
    visible = true
  }

  fun hide() {
    showAfterSelectionCommit = false
    shownForSelection = null
    visible = false
  }

  fun toggle(state: EditorState) {
    if (visible) {
      hide()
    } else {
      show(state)
    }
  }

  fun isVisibleFor(state: EditorState): Boolean = visible && state.selection == shownForSelection

  fun requestShowAfterSelectionCommit() {
    showAfterSelectionCommit = true
  }

  fun showAfterSelectionCommitIfRequested(state: EditorState): Boolean {
    if (!showAfterSelectionCommit) {
      return false
    }
    showAfterSelectionCommit = false
    if (state.selection.isCollapsed()) {
      return false
    }
    show(state)
    return true
  }

  fun onEditorStateChanged(state: EditorState) {
    if (!visible) {
      return
    }
    if (!isVisibleFor(state)) {
      hide()
    }
  }

  fun reset() {
    hide()
  }
}

private fun Selection?.isCollapsed(): Boolean = this == null || anchor == head
