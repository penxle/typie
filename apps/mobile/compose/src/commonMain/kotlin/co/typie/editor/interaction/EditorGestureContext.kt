package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.PagePoint

internal interface EditorGestureContext {
  val editor: Editor
  val semantics: EditorInteractionSemantics

  fun can(command: EditorInteractionCommand): Boolean

  fun transition(event: EditorInteractionEvent)

  fun resolvePoint(positionInNode: Offset): PagePoint?

  fun cancelTapDispatch()

  fun scheduleTapDispatch(dispatchAtMillis: Long)

  fun launchInteraction(block: suspend () -> Unit)

  fun requestFocus(editor: Editor): Boolean

  fun requestCurrentCursorLine(version: Long)
}
