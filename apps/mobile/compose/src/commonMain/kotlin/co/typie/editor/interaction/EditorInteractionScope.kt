package co.typie.editor.interaction

import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent as EditorPointerEvent
import co.typie.editor.interaction.gestures.EditorTapDispatchDelayMillis
import co.typie.editor.interaction.semantics.EditorViewportZoomSemanticConfig
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

internal class EditorInteractionScope(private val coroutineScope: CoroutineScope) :
  EditorInteractionControllerHost {
  private var editor: Editor? = null
  private var bringIntoViewRequests: EditorBringIntoViewRequests? = null
  private var uiState: EditorUiState? = null
  private var density: Float = 0f
  private var tapDispatchJob: Job? = null
  private val semantics = EditorInteractionSemantics()

  val controller: EditorInteractionController =
    EditorInteractionController(
      editorProvider = { checkNotNull(editor) { "Editor interaction scope has no editor" } },
      host = this,
      semantics = semantics,
    )

  fun update(
    editor: Editor?,
    bringIntoViewRequests: EditorBringIntoViewRequests,
    uiState: EditorUiState,
    density: Float,
    viewportZoomConfig: EditorViewportZoomSemanticConfig?,
  ) {
    this.editor = editor
    this.bringIntoViewRequests = bringIntoViewRequests
    this.uiState = uiState
    this.density = density
    semantics.viewportZoom.configure(viewportZoomConfig)
  }

  fun beginPointerSignalZoom(): Boolean {
    if (!controller.can(EditorInteractionCommand.ViewportZoomStart)) {
      return false
    }
    if (!semantics.viewportZoom.beginPointerSignal()) {
      return false
    }
    controller.applyEvent(EditorInteractionEvent.ViewportZoomStart)
    return true
  }

  fun updatePointerSignalZoom(focalPosition: Offset, normalizedDelta: Float): Boolean =
    semantics.viewportZoom.updatePointerSignal(
      focalPx = focalPosition,
      normalizedDelta = normalizedDelta,
    )

  fun endPointerSignalZoom() {
    semantics.viewportZoom.end()
    controller.applyEvent(EditorInteractionEvent.ViewportZoomEnd)
  }

  fun reset() {
    cancelTapDispatch()
    controller.reset()
    editor = null
    bringIntoViewRequests = null
    uiState = null
    density = 0f
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

  override fun launchInteraction(block: suspend () -> Unit) {
    coroutineScope.launch { block() }
  }

  override fun requestFocus(editor: Editor): Boolean = editor.focus()

  override fun enqueuePointerCancel() {
    editor?.enqueue(Message.Pointer(EditorPointerEvent.Cancel))
  }

  override fun requestCurrentCursorLine(version: Long) {
    bringIntoViewRequests?.requestForVersion(
      target = EditorBringIntoViewTarget.CurrentCursorLine,
      version = version,
    )
  }
}

internal val LocalEditorInteractionScope =
  staticCompositionLocalOf<EditorInteractionScope> { error("No EditorInteractionScope provided") }
