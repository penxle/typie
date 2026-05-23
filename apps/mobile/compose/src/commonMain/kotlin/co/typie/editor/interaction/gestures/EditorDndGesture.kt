package co.typie.editor.interaction.gestures

import co.typie.editor.interaction.sessions.EditorDndSession

internal class EditorDndGesture(private val session: EditorDndSession = EditorDndSession()) {
  fun reset() = session.reset()
}
