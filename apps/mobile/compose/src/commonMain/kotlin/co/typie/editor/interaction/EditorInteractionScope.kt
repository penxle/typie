package co.typie.editor.interaction

import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent as EditorPointerEvent
import co.typie.editor.interaction.gestures.EditorLongPressDispatchDelayMillis
import co.typie.editor.interaction.gestures.EditorTapDispatchDelayMillis
import co.typie.editor.interaction.semantics.EditorViewportZoomSemanticConfig
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.ext.ScrollGestureLockHandle
import co.typie.ext.ScrollGestureLockState
import co.typie.platform.PlatformModule
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

internal class EditorInteractionScope(private val coroutineScope: CoroutineScope) :
  EditorInteractionEffects {
  private var editor: Editor? = null
  private var bringIntoViewRequests: EditorBringIntoViewRequests? = null
  private var uiState: EditorUiState? = null
  private var density: Float = 0f
  private var tapDispatchJob: Job? = null
  private var longPressDispatchJob: Job? = null
  private var scrollGestureLockState: ScrollGestureLockState? = null
  private var scrollGestureLockHandle: ScrollGestureLockHandle? = null
  private val semantics = EditorInteractionSemantics(effects = this)

  val controller: EditorInteractionController =
    EditorInteractionController(
      editorProvider = { checkNotNull(editor) { "Editor interaction scope has no editor" } },
      effects = this,
      semantics = semantics,
      platformProvider = { PlatformModule.platform },
    )

  fun update(
    editor: Editor?,
    bringIntoViewRequests: EditorBringIntoViewRequests,
    uiState: EditorUiState,
    density: Float,
    scrollGestureLockState: ScrollGestureLockState,
    viewportZoomConfig: EditorViewportZoomSemanticConfig?,
  ) {
    this.editor = editor
    this.bringIntoViewRequests = bringIntoViewRequests
    this.uiState = uiState
    this.density = density
    this.scrollGestureLockState = scrollGestureLockState
    semantics.viewportZoom.configure(viewportZoomConfig)
  }

  fun beginPointerSignalZoom(): Boolean {
    if (!controller.canApplyModeEvent(EditorInteractionEvent.ViewportZoomStart)) {
      return false
    }
    if (!semantics.viewportZoom.beginPointerSignal()) {
      return false
    }
    controller.applyModeEvent(EditorInteractionEvent.ViewportZoomStart)
    return true
  }

  fun updatePointerSignalZoom(focalPosition: Offset, normalizedDelta: Float): Boolean =
    semantics.viewportZoom.updatePointerSignal(
      focalPx = focalPosition,
      normalizedDelta = normalizedDelta,
    )

  fun endPointerSignalZoom() {
    semantics.viewportZoom.end()
    controller.applyModeEvent(EditorInteractionEvent.ViewportZoomEnd)
  }

  fun reset() {
    cancelTapDispatch()
    cancelLongPressDispatch()
    controller.reset()
    releaseScrollGestureLock()
    editor = null
    bringIntoViewRequests = null
    uiState = null
    density = 0f
    scrollGestureLockState = null
  }

  override fun resolvePoint(positionInNode: Offset): PagePoint? {
    val currentEditor = editor ?: return null
    val currentUiState = uiState ?: return null
    if (density <= 0f) {
      return null
    }

    val xDp = positionInNode.x / density
    val yDp = positionInNode.y / density
    return currentUiState
      .resolveViewportTransform(pageSizes = currentEditor.pageSizes)
      .globalToLocal(x = xDp, y = yDp)
  }

  override fun scheduleTapDispatch(dispatchAtMillis: Long) {
    tapDispatchJob?.cancel()
    tapDispatchJob = coroutineScope.launch {
      try {
        delay(EditorTapDispatchDelayMillis)
        controller.onTapTimer(nowMillis = dispatchAtMillis)
      } finally {
        tapDispatchJob = null
      }
    }
  }

  override fun cancelTapDispatch() {
    tapDispatchJob?.cancel()
    tapDispatchJob = null
  }

  override fun scheduleLongPressDispatch(
    pointerId: Long,
    position: Offset,
    dispatchAtMillis: Long,
  ) {
    longPressDispatchJob?.cancel()
    longPressDispatchJob = coroutineScope.launch {
      try {
        delay(EditorLongPressDispatchDelayMillis)
        controller.onLongPressTimer(
          pointerId = pointerId,
          position = position,
          nowMillis = dispatchAtMillis,
        )
      } finally {
        longPressDispatchJob = null
      }
    }
  }

  override fun cancelLongPressDispatch() {
    longPressDispatchJob?.cancel()
    longPressDispatchJob = null
  }

  override fun launchInteraction(block: suspend () -> Unit) {
    coroutineScope.launch { block() }
  }

  override fun requestFocus(editor: Editor): Boolean = editor.focus()

  override fun enqueuePointerCancel() {
    editor?.enqueue(Message.Pointer(EditorPointerEvent.Cancel))
  }

  override fun setScrollGestureLocked(locked: Boolean) {
    if (locked) {
      if (scrollGestureLockHandle == null) {
        scrollGestureLockHandle = scrollGestureLockState?.acquire()
      }
    } else {
      releaseScrollGestureLock()
    }
  }

  override fun requestCurrentCursorLine(version: Long) {
    bringIntoViewRequests?.requestForVersion(
      target = EditorBringIntoViewTarget.CurrentCursorLine,
      version = version,
    )
  }

  private fun releaseScrollGestureLock() {
    scrollGestureLockHandle?.release()
    scrollGestureLockHandle = null
  }
}

internal val LocalEditorInteractionScope =
  staticCompositionLocalOf<EditorInteractionScope> { error("No EditorInteractionScope provided") }
