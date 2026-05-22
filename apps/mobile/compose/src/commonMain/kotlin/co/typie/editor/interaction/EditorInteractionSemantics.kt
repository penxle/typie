package co.typie.editor.interaction

import co.typie.editor.interaction.semantics.EditorCursorMoveSemantic
import co.typie.editor.interaction.semantics.EditorSelectionExpansionSemantic

internal class EditorInteractionSemantics(
  val cursorMove: EditorCursorMoveSemantic = EditorCursorMoveSemantic(),
  val selectionExpansion: EditorSelectionExpansionSemantic = EditorSelectionExpansionSemantic(),
) {
  fun reset() {
    cursorMove.reset()
    selectionExpansion.reset()
  }
}
