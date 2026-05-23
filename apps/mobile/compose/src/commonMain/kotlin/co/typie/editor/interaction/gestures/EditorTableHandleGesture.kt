package co.typie.editor.interaction.gestures

import co.typie.editor.interaction.sessions.EditorTableHandleDragSession

internal class EditorTableHandleGesture(
  private val session: EditorTableHandleDragSession = EditorTableHandleDragSession()
) {
  val dragging: Boolean
    get() = session.dragging

  fun reset() = session.reset()
}
