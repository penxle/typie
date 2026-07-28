package co.typie.editor.runtime

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.Selection

internal class EditorContextMenuState {
  var visible: Boolean by mutableStateOf(false)
    private set

  private var shownForSelection: Selection? = null
  private var pendingPublicationTarget: PendingPublicationTarget? = null

  fun show(state: EditorState) {
    pendingPublicationTarget = null
    shownForSelection = state.selection
    visible = true
  }

  fun hide() {
    hide(clearPendingPublicationRequest = true)
  }

  private fun hide(clearPendingPublicationRequest: Boolean) {
    if (clearPendingPublicationRequest) {
      pendingPublicationTarget = null
    }
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

  fun requestShowForAppliedSelection(editor: Editor, state: EditorState) {
    val selection = state.selection
    if (selection == null || selection.isCollapsed()) {
      pendingPublicationTarget = null
      return
    }
    pendingPublicationTarget =
      PendingPublicationTarget(editor = editor, version = state.version, selection = selection)
    onEditorStateChanged(editor = editor, state = editor.publishedState)
  }

  fun onEditorStateChanged(editor: Editor, state: EditorState) {
    val target = pendingPublicationTarget
    if (target != null && target.editor !== editor) {
      hide()
      return
    }

    if (visible && !isVisibleFor(state)) {
      hide(clearPendingPublicationRequest = target == null)
    }

    if (target == null || state.version < target.version) {
      return
    }
    pendingPublicationTarget = null
    if (state.selection == target.selection && !state.selection.isCollapsed()) {
      show(state)
    }
  }

  fun reset() {
    hide()
  }

  private data class PendingPublicationTarget(
    val editor: Editor,
    val version: Long,
    val selection: Selection,
  )
}
