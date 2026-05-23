package co.typie.editor.interaction.gestures

import co.typie.editor.interaction.sessions.EditorSelectionHandleDragSession

internal class EditorSelectionHandleGesture(
  private val session: EditorSelectionHandleDragSession = EditorSelectionHandleDragSession()
) {
  val pendingDrag: Boolean
    get() = session.pendingDrag

  val activeDrag: Boolean
    get() = session.activeDrag

  fun reset() = session.reset()
}
