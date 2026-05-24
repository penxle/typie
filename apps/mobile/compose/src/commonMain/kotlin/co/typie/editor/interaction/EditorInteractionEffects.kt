package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor

internal interface EditorInteractionEffects {
  fun dispatchEdgeAutoScroll(delta: Offset): Offset

  fun scheduleTapDispatch(dispatchAtMillis: Long)

  fun cancelTapDispatch()

  fun scheduleLongPressDispatch(pointerId: Long, position: Offset, dispatchAtMillis: Long)

  fun cancelLongPressDispatch()

  fun launchInteraction(block: suspend () -> Unit)

  fun requestFocus(editor: Editor): Boolean

  fun enqueuePointerCancel()

  fun setScrollGestureLocked(locked: Boolean)

  fun performSelectionHaptic()

  fun requestCurrentCursorLine(version: Long)
}
