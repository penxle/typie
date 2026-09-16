package co.typie.editor.runtime

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.editor.EditorState
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.Selection
import co.typie.editor.interaction.EditorInteractionEffects
import kotlinx.coroutines.delay

internal class EditorCursorHandleState {
  var visible by mutableStateOf(false)
    private set

  private var selection: Selection? = null
  private var documentRevision = 0L
  private var shownVersion = 0L
  private var generation = 0L
  private var pressed = false

  fun isVisibleFor(state: EditorState): Boolean =
    visible &&
      state.version >= shownVersion &&
      state.documentRevision == documentRevision &&
      (pressed || state.selection == selection)

  fun show(state: EditorState, effects: EditorInteractionEffects) {
    if (state.selection == null || !state.selection.isCollapsed() || state.cursor == null) {
      hide()
      return
    }
    selection = state.selection
    documentRevision = state.documentRevision
    shownVersion = state.version
    visible = true
    pressed = false
    val request = ++generation
    effects.launchInteraction {
      delay(4_000L)
      if (generation == request) hide()
    }
  }

  fun hold() {
    generation += 1
    pressed = true
  }

  fun onEditorStateChanged(state: EditorState) {
    if (!visible || state.version < shownVersion) return
    if (
      state.documentRevision != documentRevision ||
        state.selection == null ||
        !state.selection.isCollapsed() ||
        state.cursor == null ||
        (!pressed && state.selection != selection)
    )
      hide()
  }

  fun hide() {
    generation += 1
    visible = false
    pressed = false
    selection = null
  }
}
