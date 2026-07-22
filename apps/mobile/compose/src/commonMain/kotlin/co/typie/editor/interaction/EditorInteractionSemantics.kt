package co.typie.editor.interaction

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.interaction.semantics.EditorContextMenuSemantic
import co.typie.editor.interaction.semantics.EditorEdgeAutoScrollSemantic
import co.typie.editor.interaction.semantics.EditorInteractiveHitSemantic
import co.typie.editor.interaction.semantics.EditorLongPressSemantic
import co.typie.editor.interaction.semantics.EditorMagnifierSemantic
import co.typie.editor.interaction.semantics.EditorPointSelectionSemantic
import co.typie.editor.interaction.semantics.EditorSelectionExpansionSemantic
import co.typie.editor.interaction.semantics.EditorSelectionHapticSemantic
import co.typie.editor.interaction.semantics.EditorTableColumnResizeSemantic
import co.typie.editor.interaction.semantics.EditorViewportZoomSemantic
import co.typie.editor.runtime.EditorContextMenuState

internal class EditorInteractionSemantics(
  effects: EditorInteractionEffects,
  contextMenuStateProvider: () -> EditorContextMenuState,
  val pointSelection: EditorPointSelectionSemantic =
    EditorPointSelectionSemantic(effects = effects),
  val contextMenu: EditorContextMenuSemantic =
    EditorContextMenuSemantic(stateProvider = contextMenuStateProvider),
  val interactiveHit: EditorInteractiveHitSemantic = EditorInteractiveHitSemantic(),
  val longPress: EditorLongPressSemantic = EditorLongPressSemantic(),
  val selectionExpansion: EditorSelectionExpansionSemantic = EditorSelectionExpansionSemantic(),
  val viewportZoom: EditorViewportZoomSemantic = EditorViewportZoomSemantic(),
  val magnifier: EditorMagnifierSemantic = EditorMagnifierSemantic(),
  val edgeAutoScroll: EditorEdgeAutoScrollSemantic = EditorEdgeAutoScrollSemantic(),
  val tableColumnResize: EditorTableColumnResizeSemantic = EditorTableColumnResizeSemantic(),
  val selectionHaptic: EditorSelectionHapticSemantic =
    EditorSelectionHapticSemantic(effects = effects),
) {
  fun onEditorStateChanged(editor: Editor, state: EditorState, mode: EditorInteractionMode) {
    contextMenu.onEditorStateChanged(state)
    selectionHaptic.onEditorStateChanged(editor = editor, state = state, mode = mode)
  }

  fun reset() {
    selectionExpansion.reset()
    viewportZoom.reset()
    magnifier.reset()
    edgeAutoScroll.reset()
    tableColumnResize.reset()
    selectionHaptic.reset()
  }
}
