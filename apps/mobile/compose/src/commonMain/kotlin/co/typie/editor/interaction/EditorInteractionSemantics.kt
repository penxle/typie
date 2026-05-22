package co.typie.editor.interaction

import co.typie.editor.interaction.semantics.EditorAutoScrollSemantic
import co.typie.editor.interaction.semantics.EditorContextMenuSemantic
import co.typie.editor.interaction.semantics.EditorCursorMoveSemantic
import co.typie.editor.interaction.semantics.EditorMagnifierSemantic
import co.typie.editor.interaction.semantics.EditorSelectionExpansionSemantic
import co.typie.editor.interaction.semantics.EditorViewportZoomSemantic

internal class EditorInteractionSemantics(
  val cursorMove: EditorCursorMoveSemantic = EditorCursorMoveSemantic(),
  val selectionExpansion: EditorSelectionExpansionSemantic = EditorSelectionExpansionSemantic(),
  val viewportZoom: EditorViewportZoomSemantic = EditorViewportZoomSemantic(),
  val contextMenu: EditorContextMenuSemantic = EditorContextMenuSemantic(),
  val magnifier: EditorMagnifierSemantic = EditorMagnifierSemantic(),
  val autoScroll: EditorAutoScrollSemantic = EditorAutoScrollSemantic(),
) {
  fun reset() {
    cursorMove.reset()
    selectionExpansion.reset()
    viewportZoom.reset()
    contextMenu.reset()
    magnifier.reset()
    autoScroll.reset()
  }
}
