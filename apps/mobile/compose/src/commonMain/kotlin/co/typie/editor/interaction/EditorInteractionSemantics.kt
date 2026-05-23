package co.typie.editor.interaction

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.interaction.semantics.EditorAutoScrollSemantic
import co.typie.editor.interaction.semantics.EditorContextMenuSemantic
import co.typie.editor.interaction.semantics.EditorCursorMoveSemantic
import co.typie.editor.interaction.semantics.EditorMagnifierSemantic
import co.typie.editor.interaction.semantics.EditorSelectionExpansionSemantic
import co.typie.editor.interaction.semantics.EditorSelectionHapticSemantic
import co.typie.editor.interaction.semantics.EditorViewportZoomSemantic

internal class EditorInteractionSemantics(
  effects: EditorInteractionEffects,
  val cursorMove: EditorCursorMoveSemantic = EditorCursorMoveSemantic(effects = effects),
  val selectionExpansion: EditorSelectionExpansionSemantic = EditorSelectionExpansionSemantic(),
  val viewportZoom: EditorViewportZoomSemantic = EditorViewportZoomSemantic(),
  val contextMenu: EditorContextMenuSemantic = EditorContextMenuSemantic(),
  val magnifier: EditorMagnifierSemantic = EditorMagnifierSemantic(),
  val autoScroll: EditorAutoScrollSemantic = EditorAutoScrollSemantic(),
  val selectionHaptic: EditorSelectionHapticSemantic =
    EditorSelectionHapticSemantic(effects = effects),
) {
  fun onEditorStateChanged(editor: Editor, state: EditorState, mode: EditorInteractionMode) {
    selectionHaptic.onEditorStateChanged(editor = editor, state = state, mode = mode)
    contextMenu.onEditorStateChanged(state)
    contextMenu.showAfterSelectionCommitIfRequested(state)
  }

  fun reset() {
    cursorMove.reset()
    selectionExpansion.reset()
    viewportZoom.reset()
    contextMenu.reset()
    magnifier.reset()
    autoScroll.reset()
    selectionHaptic.reset()
  }
}
