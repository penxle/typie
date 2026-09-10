package co.typie.editor.runtime

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Rect
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.Selection

internal enum class EditorContextMenuMode {
  Compact,
  Expanded,
}

internal class EditorContextMenuState {
  var visible: Boolean by mutableStateOf(false)
    private set

  var mode: EditorContextMenuMode by mutableStateOf(EditorContextMenuMode.Compact)
    private set

  private var shownForSelection: Selection? = null
  var pointerPosition: PagePoint? by mutableStateOf(null)
    private set

  var boundsInWindow: List<Rect> = emptyList()
  private var outsideDismissPointerId: Long? = null

  fun beginOutsideDismissGesture(pointerId: Long) {
    outsideDismissPointerId = pointerId
    hide()
  }

  fun suppressesTap(pointerId: Long): Boolean = outsideDismissPointerId == pointerId

  fun endOutsideDismissGesture(pointerId: Long) {
    if (outsideDismissPointerId == pointerId) outsideDismissPointerId = null
  }

  private var pendingPublicationTarget: PendingPublicationTarget? = null

  fun show(
    state: EditorState,
    pointerPosition: PagePoint? = null,
    mode: EditorContextMenuMode =
      if (pointerPosition == null) EditorContextMenuMode.Compact else EditorContextMenuMode.Expanded,
  ) {
    pendingPublicationTarget = null
    this.pointerPosition = pointerPosition
    this.mode = mode
    shownForSelection = state.selection
    visible = true
  }

  fun expand() {
    if (visible) mode = EditorContextMenuMode.Expanded
  }

  fun hide() {
    hide(clearPendingPublicationRequest = true)
  }

  private fun hide(clearPendingPublicationRequest: Boolean) {
    if (clearPendingPublicationRequest) {
      pendingPublicationTarget = null
    }
    shownForSelection = null
    pointerPosition = null
    mode = EditorContextMenuMode.Compact
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

  fun requestShowForAppliedSelection(
    editor: Editor,
    state: EditorState,
    pointerPosition: PagePoint? = null,
    mode: EditorContextMenuMode =
      if (pointerPosition == null) EditorContextMenuMode.Compact else EditorContextMenuMode.Expanded,
  ) {
    val selection = state.selection
    if (selection == null || (selection.isCollapsed() && pointerPosition == null)) {
      pendingPublicationTarget = null
      return
    }
    pendingPublicationTarget =
      PendingPublicationTarget(
        editor = editor,
        version = state.version,
        selection = selection,
        pointerPosition = pointerPosition,
        mode = mode,
      )
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
    if (
      state.selection == target.selection &&
        (!state.selection.isCollapsed() || target.pointerPosition != null)
    ) {
      show(state, pointerPosition = target.pointerPosition, mode = target.mode)
    }
  }

  fun reset() {
    outsideDismissPointerId = null
    boundsInWindow = emptyList()
    hide()
  }

  private data class PendingPublicationTarget(
    val editor: Editor,
    val version: Long,
    val selection: Selection,
    val pointerPosition: PagePoint?,
    val mode: EditorContextMenuMode,
  )
}
