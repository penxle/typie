package co.typie.editor.interaction

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.interaction.semantics.EditorCursorMoveSemantic
import co.typie.editor.interaction.semantics.EditorEdgeAutoScrollSemantic
import co.typie.editor.interaction.semantics.EditorMagnifierSemantic
import co.typie.editor.interaction.semantics.EditorSelectionExpansionSemantic
import co.typie.editor.interaction.semantics.EditorSelectionHapticSemantic
import co.typie.editor.interaction.semantics.EditorTableColumnResizeSemantic
import co.typie.editor.interaction.semantics.EditorViewportZoomSemantic

internal class EditorInteractionSemantics(
  effects: EditorInteractionEffects,
  val cursorMove: EditorCursorMoveSemantic = EditorCursorMoveSemantic(effects = effects),
  val selectionExpansion: EditorSelectionExpansionSemantic = EditorSelectionExpansionSemantic(),
  val viewportZoom: EditorViewportZoomSemantic = EditorViewportZoomSemantic(),
  val magnifier: EditorMagnifierSemantic = EditorMagnifierSemantic(),
  val edgeAutoScroll: EditorEdgeAutoScrollSemantic = EditorEdgeAutoScrollSemantic(),
  val tableColumnResize: EditorTableColumnResizeSemantic = EditorTableColumnResizeSemantic(),
  val selectionHaptic: EditorSelectionHapticSemantic =
    EditorSelectionHapticSemantic(effects = effects),
) {
  fun onEditorStateChanged(editor: Editor, state: EditorState, mode: EditorInteractionMode) {
    selectionHaptic.onEditorStateChanged(editor = editor, state = state, mode = mode)
  }

  fun reset() {
    cursorMove.reset()
    selectionExpansion.reset()
    viewportZoom.reset()
    magnifier.reset()
    edgeAutoScroll.reset()
    tableColumnResize.reset()
    selectionHaptic.reset()
  }
}
