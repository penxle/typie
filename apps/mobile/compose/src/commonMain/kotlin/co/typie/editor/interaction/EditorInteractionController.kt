package co.typie.editor.interaction

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.interaction.gestures.EditorPanGestureDriver
import co.typie.editor.interaction.semantics.EditorTableColumnResizePresentation
import co.typie.editor.runtime.EditorUiState
import co.typie.platform.Platform

internal class EditorInteractionController(
  private val editorProvider: () -> Editor,
  private val effects: EditorInteractionEffects,
  private val geometry: EditorInteractionGeometry,
  private val semantics: EditorInteractionSemantics = EditorInteractionSemantics(effects = effects),
  private val platformProvider: () -> Platform = { Platform.Desktop },
  private val uiStateProvider: () -> EditorUiState,
  private val pointerInputEnabledProvider: () -> Boolean = { true },
  private val readOnlyProvider: () -> Boolean = { false },
) {
  private var mode by mutableStateOf(EditorInteractionMode.Idle)
  private val gestureContext =
    object : EditorGestureContext {
      override val editor: Editor
        get() = editorProvider()

      override val semantics: EditorInteractionSemantics
        get() = this@EditorInteractionController.semantics

      override val effects: EditorInteractionEffects
        get() = this@EditorInteractionController.effects

      override val geometry: EditorInteractionGeometry
        get() = this@EditorInteractionController.geometry

      override val mode: EditorInteractionMode
        get() = this@EditorInteractionController.mode

      override val uiState: EditorUiState
        get() = uiStateProvider()

      override val readOnly: Boolean
        get() = readOnlyProvider()

      override val platform: Platform
        get() = platformProvider()

      override fun applyModeEvent(event: EditorInteractionEvent) {
        this@EditorInteractionController.applyModeEvent(event)
      }

      override fun reduceMode(event: EditorInteractionEvent) {
        this@EditorInteractionController.reduceMode(event)
      }
    }
  private val gestures = EditorInteractionGestures(contextProvider = { gestureContext })

  val interactionMode: EditorInteractionMode
    get() = mode

  val magnifierPosition: Offset?
    get() = semantics.magnifier.position

  val tableColumnResizePresentation: EditorTableColumnResizePresentation
    get() = semantics.tableColumnResize.presentation

  fun updateTapSlop(tapSlopPx: Float) {
    gestures.updateTapSlop(tapSlopPx)
  }

  fun updateColumnResizeSlop(dragSlopPx: Float) {
    gestures.updateColumnResizeSlop(dragSlopPx)
  }

  fun canApplyModeEvent(event: EditorInteractionEvent): Boolean = mode.canApply(event)

  fun onPointerDown(
    pointerId: Long,
    position: Offset?,
    nowMillis: Long,
    tapEnabled: Boolean = true,
    inputModifiers: InputModifiers = InputModifiers(),
    positionInRoot: Offset = requireNotNull(position),
    touchPanDriver: EditorPanGestureDriver? = null,
  ): Boolean =
    if (ensurePointerInputEnabled()) {
      gestures.handlePointerDown(
        pointerId = pointerId,
        positionInEditor = position,
        positionInRoot = positionInRoot,
        nowMillis = nowMillis,
        tapEnabled = tapEnabled && position != null,
        inputModifiers = inputModifiers,
        touchPanDriver = touchPanDriver,
        context = gestureContext,
      )
    } else {
      false
    }

  fun onPointerMove(
    pointerId: Long,
    position: Offset?,
    nowMillis: Long,
    positionInRoot: Offset = requireNotNull(position),
    pressed: Boolean = true,
    consumed: Boolean = false,
  ): Boolean =
    if (ensurePointerInputEnabled()) {
      gestures.handlePointerMove(
        pointerId = pointerId,
        positionInEditor = position,
        positionInRoot = positionInRoot,
        nowMillis = nowMillis,
        pressed = pressed,
        consumed = consumed,
        context = gestureContext,
      )
    } else {
      false
    }

  fun onPointerUp(
    pointerId: Long,
    position: Offset?,
    nowMillis: Long,
    positionInRoot: Offset = requireNotNull(position),
  ): Boolean =
    if (ensurePointerInputEnabled()) {
      gestures.handlePointerUp(
        pointerId = pointerId,
        positionInEditor = position,
        positionInRoot = positionInRoot,
        nowMillis = nowMillis,
        context = gestureContext,
      )
    } else {
      false
    }

  fun onPinchSample(sample: EditorPinchSample): Boolean =
    if (ensurePointerInputEnabled()) {
      gestures.handlePinchSample(sample = sample, context = gestureContext)
    } else {
      false
    }

  fun onPinchEnd(): Boolean = gestures.endPinch(context = gestureContext)

  fun endPinchAndResumeViewportPan(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    driver: EditorPanGestureDriver,
  ): Boolean {
    if (!ensurePointerInputEnabled()) {
      return false
    }
    return gestures.endPinchAndResumeViewportPan(
      pointerId = pointerId,
      position = position,
      nowMillis = nowMillis,
      driver = driver,
      context = gestureContext,
    )
  }

  fun beginIndirectZoom(): Boolean =
    if (ensurePointerInputEnabled()) {
      gestures.beginIndirectZoom(context = gestureContext)
    } else {
      false
    }

  fun cancelPendingPointerForIndirectInput(): Boolean =
    gestures.cancelPendingPointerForIndirectInput(context = gestureContext)

  fun updateIndirectScrollZoom(focalInRootPx: Offset, normalizedDelta: Float): Boolean =
    if (ensurePointerInputEnabled()) {
      gestures.updateIndirectScrollZoom(
        focalInRootPx = focalInRootPx,
        normalizedDelta = normalizedDelta,
        context = gestureContext,
      )
    } else {
      false
    }

  fun updateIndirectScaleZoom(focalInRootPx: Offset, scaleFactor: Float): Boolean =
    if (ensurePointerInputEnabled()) {
      gestures.updateIndirectScaleZoom(
        focalInRootPx = focalInRootPx,
        scaleFactor = scaleFactor,
        context = gestureContext,
      )
    } else {
      false
    }

  fun endIndirectZoom() {
    gestures.endIndirectZoom(context = gestureContext)
  }

  fun onLongPressTimer(pointerId: Long, position: Offset, nowMillis: Long): Boolean =
    if (ensurePointerInputEnabled()) {
      gestures.handleLongPressTimer(
        pointerId = pointerId,
        position = position,
        nowMillis = nowMillis,
        context = gestureContext,
      )
    } else {
      false
    }

  fun onTapTimer(nowMillis: Long) {
    if (!ensurePointerInputEnabled()) {
      return
    }
    gestures.handleTapTimer(nowMillis = nowMillis, context = gestureContext)
  }

  fun onEditorStateChanged(state: EditorState) {
    semantics.onEditorStateChanged(editor = editorProvider(), state = state, mode = mode)
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

  private fun ensurePointerInputEnabled(): Boolean {
    if (pointerInputEnabledProvider()) {
      return true
    }
    cancel()
    return false
  }
}
