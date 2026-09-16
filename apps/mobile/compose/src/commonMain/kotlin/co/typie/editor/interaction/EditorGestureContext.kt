package co.typie.editor.interaction

import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.input.pointer.PointerType
import co.typie.editor.Editor
import co.typie.editor.interaction.gestures.EditorSelectionHandleType
import co.typie.editor.runtime.EditorCursorHandleState
import co.typie.platform.Platform

internal interface EditorGestureContext {
  val selectionHandlesHidden: Boolean
  val editor: Editor
  val cursorHandle: EditorCursorHandleState
  val selectionHandleImages: Map<EditorSelectionHandleType, ImageBitmap>
  val semantics: EditorInteractionSemantics
  val effects: EditorInteractionEffects
  val geometry: EditorInteractionGeometry
  val mode: EditorInteractionMode
  val pointerType: PointerType
  val isFocused: Boolean
  val readOnly: Boolean
  val editing: Boolean
  val doubleTapToEditEnabled: Boolean

  val platform: Platform

  /** Reduces mode and runs shared cleanup for externally applied mode events. */
  fun applyModeEvent(event: EditorInteractionEvent)

  /** Reduces mode only; the gesture that calls this owns any required cleanup. */
  fun reduceMode(event: EditorInteractionEvent)
}
