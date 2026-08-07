package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor

internal interface EditorInteractionEffects {
  fun dispatchEdgeAutoScroll(delta: Offset): Offset

  fun scheduleTapDispatch(dispatchAtMillis: Long)

  fun cancelTapDispatch()

  fun scheduleTapSequenceConfirmation(onConfirmed: () -> Unit)

  fun cancelTapSequenceConfirmation()

  fun scheduleLongPressDispatch(pointerId: Long, position: Offset, dispatchAtMillis: Long)

  fun cancelLongPressDispatch()

  fun launchInteraction(block: suspend () -> Unit)

  fun requestEditing(editor: Editor): Boolean

  fun showReadingTapHint()

  fun requestFocus(editor: Editor): Boolean

  fun requestSoftwareKeyboard()

  fun setScrollGestureLocked(locked: Boolean)

  fun performSelectionHaptic()

  fun requestPointerSelectionHead(version: Long)
}
