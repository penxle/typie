package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.interaction.EditorInteractionEffects
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.isLongPressing

internal class EditorSelectionHapticSemantic(private val effects: EditorInteractionEffects) {
  private var previousSelection: Selection? = null
  private var previousEndpoints: SelectionEndpoints? = null

  fun onEditorStateChanged(editor: Editor, state: EditorState, mode: EditorInteractionMode) {
    val previousSelectionSnapshot = previousSelection
    val previousEndpointsSnapshot = previousEndpoints
    val currentSelection = state.selection
    val currentEndpoints = editor.selectionEndpointsForHaptics(currentSelection)
    val handlesJustAppeared =
      previousSelectionSnapshot != null &&
        previousEndpointsSnapshot == null &&
        currentEndpoints != null
    val anyHandleMoved =
      previousEndpointsSnapshot != null &&
        currentEndpoints != null &&
        !previousEndpointsSnapshot.hasSameHandlePositionsAs(currentEndpoints)
    val longPressSelectionMoved =
      mode.isLongPressing &&
        previousSelectionSnapshot != null &&
        currentSelection != null &&
        previousSelectionSnapshot != currentSelection &&
        !handlesJustAppeared

    if (
      handlesJustAppeared ||
        (mode == EditorInteractionMode.SelectionHandleDragging && anyHandleMoved) ||
        longPressSelectionMoved
    ) {
      effects.performSelectionHaptic()
    }

    previousSelection = currentSelection
    previousEndpoints = currentEndpoints
  }

  fun reset() {
    previousSelection = null
    previousEndpoints = null
  }
}

private fun Editor.selectionEndpointsForHaptics(selection: Selection?): SelectionEndpoints? {
  if (selection.isCollapsed()) {
    return null
  }
  return selectionEndpoints()
}

private fun SelectionEndpoints.hasSameHandlePositionsAs(other: SelectionEndpoints): Boolean =
  from.hasSameHandlePositionAs(other.from) && to.hasSameHandlePositionAs(other.to)

private fun PageRect.hasSameHandlePositionAs(other: PageRect): Boolean =
  pageIdx == other.pageIdx && rect.x == other.rect.x && rect.y == other.rect.y
