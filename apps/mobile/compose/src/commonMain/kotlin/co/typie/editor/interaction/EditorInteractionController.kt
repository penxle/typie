package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.interaction.gestures.EditorSelectionHandleGesture
import co.typie.platform.Platform

internal class EditorInteractionController(
  private val editorProvider: () -> Editor,
  private val effects: EditorInteractionEffects,
  private val gestures: EditorInteractionGestures = EditorInteractionGestures(),
  private val semantics: EditorInteractionSemantics = EditorInteractionSemantics(effects = effects),
  private val platformProvider: () -> Platform = { Platform.Desktop },
) {
  private var mode = EditorInteractionMode.Idle
  private val gestureContext =
    object : EditorGestureContext {
      override val editor: Editor
        get() = editorProvider()

      override val semantics: EditorInteractionSemantics
        get() = this@EditorInteractionController.semantics

      override val effects: EditorInteractionEffects
        get() = this@EditorInteractionController.effects

      override val mode: EditorInteractionMode
        get() = this@EditorInteractionController.mode

      override val platform: Platform
        get() = platformProvider()

      override fun applyModeEvent(event: EditorInteractionEvent) {
        this@EditorInteractionController.applyModeEvent(event)
      }

      override fun reduceMode(event: EditorInteractionEvent) {
        this@EditorInteractionController.reduceMode(event)
      }
    }
  val interactionMode: EditorInteractionMode
    get() = mode

  val hasActivePointer: Boolean
    get() = gestures.hasActivePointer

  val isIgnoringUntilAllPointersUp: Boolean
    get() = gestures.isIgnoringUntilAllPointersUp

  val selectionHandleGesture: EditorSelectionHandleGesture
    get() = gestures.selectionHandle

  val interactionContext: EditorGestureContext
    get() = gestureContext

  val magnifierPosition: Offset?
    get() = semantics.magnifier.position

  fun isContextMenuVisibleFor(state: EditorState): Boolean =
    semantics.contextMenu.isVisibleFor(state)

  fun updateTapSlop(tapSlopPx: Float) {
    gestures.updateTapSlop(tapSlopPx)
  }

  fun canApplyModeEvent(event: EditorInteractionEvent): Boolean = mode.canApply(event)

  fun onPointerDown(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    tapEnabled: Boolean = true,
  ): Boolean =
    gestures.handlePointerDown(
      pointerId = pointerId,
      position = position,
      nowMillis = nowMillis,
      tapEnabled = tapEnabled,
      context = gestureContext,
    )

  fun onPointerMove(pointerId: Long, position: Offset, nowMillis: Long): Boolean =
    gestures.handlePointerMove(
      pointerId = pointerId,
      position = position,
      nowMillis = nowMillis,
      context = gestureContext,
    )

  fun onPointerUp(pointerId: Long, position: Offset, nowMillis: Long): Boolean =
    gestures.handlePointerUp(
      pointerId = pointerId,
      position = position,
      nowMillis = nowMillis,
      context = gestureContext,
    )

  fun onLongPressTimer(pointerId: Long, position: Offset, nowMillis: Long): Boolean =
    gestures.handleLongPressTimer(
      pointerId = pointerId,
      position = position,
      nowMillis = nowMillis,
      context = gestureContext,
    )

  fun onTapTimer(nowMillis: Long) {
    gestures.handleTapTimer(nowMillis = nowMillis, context = gestureContext)
  }

  fun onEditorStateChanged(state: EditorState) {
    semantics.onEditorStateChanged(editor = editorProvider(), state = state, mode = mode)
  }

  fun onViewportScrollStarted() {
    semantics.contextMenu.hide()
  }

  fun onEditorFocusChanged(focused: Boolean) {
    if (!focused) {
      semantics.contextMenu.hide()
    }
  }

  fun applyModeEvent(event: EditorInteractionEvent) {
    if (!mode.canApply(event)) {
      return
    }
    val previousMode = mode
    reduceMode(event)
    gestures.handleAppliedModeEvent(
      event = event,
      previousMode = previousMode,
      currentMode = mode,
      context = gestureContext,
    )
  }

  fun cancel() {
    reduceMode(EditorInteractionEvent.PointerCancel)
    gestures.cancel(context = gestureContext)
  }

  fun reset() {
    effects.setScrollGestureLocked(false)
    mode = EditorInteractionMode.Idle
    gestures.reset()
    semantics.reset()
  }

  private fun reduceMode(event: EditorInteractionEvent) {
    mode = mode.reduce(event)
  }
}
