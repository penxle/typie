package co.typie.editor.interaction

import co.typie.editor.Editor
import co.typie.editor.runtime.EditorUiState
import co.typie.platform.Platform

internal interface EditorGestureContext {
  val editor: Editor
  val semantics: EditorInteractionSemantics
  val effects: EditorInteractionEffects
  val geometry: EditorInteractionGeometry
  val mode: EditorInteractionMode
  val uiState: EditorUiState

  val platform: Platform

  /** Reduces mode and runs shared cleanup for externally applied mode events. */
  fun applyModeEvent(event: EditorInteractionEvent)

  /** Reduces mode only; the gesture that calls this owns any required cleanup. */
  fun reduceMode(event: EditorInteractionEvent)
}
