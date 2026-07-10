package co.typie.editor.interaction

import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.interaction.gestures.EditorLongPressDispatchDelayMillis
import co.typie.editor.interaction.gestures.EditorTapDispatchDelayMillis
import co.typie.editor.interaction.semantics.EditorViewportZoomSemanticConfig
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.viewport.EditorViewportState
import co.typie.ext.ScrollGestureLockHandle
import co.typie.ext.ScrollGestureLockState
import co.typie.platform.Platform
import co.typie.platform.PlatformModule
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

internal class EditorInteractionScope(
  private val coroutineScope: CoroutineScope,
  private val platformProvider: () -> Platform = { PlatformModule.platform },
) : EditorInteractionEffects, EditorInteractionGeometry {
  private var editor: Editor? = null
  private var bringIntoViewRequests: EditorBringIntoViewRequests? = null
  private var uiState: EditorUiState? = null
  private var visibleArea: EditorVisibleArea? = null
  private var viewportState: EditorViewportState? = null
  override var density: Float = 0f
    private set

  private var onSelectionHaptic: (() -> Unit)? = null
  private var onRequestSoftwareKeyboard: (() -> Unit)? = null
  private var pointerInputEnabled: () -> Boolean = { true }
  private var tapDispatchJob: Job? = null
  private var longPressDispatchJob: Job? = null
  private var scrollGestureLockState: ScrollGestureLockState? = null
  private var scrollGestureLockHandle: ScrollGestureLockHandle? = null
  private val semantics = EditorInteractionSemantics(effects = this)

  val controller: EditorInteractionController =
    EditorInteractionController(
      editorProvider = { checkNotNull(editor) { "Editor interaction scope has no editor" } },
      effects = this,
      geometry = this,
      semantics = semantics,
      platformProvider = { platformProvider() },
      uiStateProvider = { checkNotNull(uiState) { "Editor interaction scope has no UI state" } },
      pointerInputEnabledProvider = { pointerInputEnabled() },
    )

  fun update(
    editor: Editor?,
    bringIntoViewRequests: EditorBringIntoViewRequests,
    uiState: EditorUiState,
    visibleArea: EditorVisibleArea,
    viewportState: EditorViewportState,
    density: Float,
    scrollGestureLockState: ScrollGestureLockState,
    viewportZoomConfig: EditorViewportZoomSemanticConfig?,
    pointerInputEnabled: () -> Boolean = { true },
    onSelectionHaptic: () -> Unit,
    onRequestSoftwareKeyboard: () -> Unit,
  ) {
    this.editor = editor
    this.bringIntoViewRequests = bringIntoViewRequests
    this.uiState = uiState
    this.visibleArea = visibleArea
    this.viewportState = viewportState
    this.density = density
    this.scrollGestureLockState = scrollGestureLockState
    this.onSelectionHaptic = onSelectionHaptic
    this.onRequestSoftwareKeyboard = onRequestSoftwareKeyboard
    this.pointerInputEnabled = pointerInputEnabled
    semantics.viewportZoom.configure(viewportZoomConfig)
  }

  fun onEditorStateChanged(state: EditorState) {
    if (editor == null) {
      return
    }
    controller.onEditorStateChanged(state)
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
    visibleArea = null
    viewportState = null
    density = 0f
    onSelectionHaptic = null
    onRequestSoftwareKeyboard = null
    pointerInputEnabled = { true }
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

  override fun resolvePagePosition(page: Int, x: Float, y: Float): Offset? {
    val currentEditor = editor ?: return null
    val currentUiState = uiState ?: return null
    if (density <= 0f) {
      return null
    }

    val positionDp =
      currentUiState
        .resolveViewportTransform(pageSizes = currentEditor.pageSizes)
        .localToGlobal(page = page, x = x, y = y) ?: return null
    return Offset(x = positionDp.x * density, y = positionDp.y * density)
  }

  override fun resolveEdgeAutoScrollViewport(): EditorEdgeAutoScrollViewport? {
    val currentUiState = uiState ?: return null
    val currentVisibleArea = visibleArea ?: return null
    val currentViewportState = viewportState ?: return null
    return resolveEditorEdgeAutoScrollViewport(
      uiState = currentUiState,
      visibleArea = currentVisibleArea,
      viewportState = currentViewportState,
      density = density,
    )
  }

  override fun dispatchEdgeAutoScroll(delta: Offset): Offset {
    val currentViewportState = viewportState ?: return Offset.Zero
    if (density <= 0f) {
      return Offset.Zero
    }
    val consumed =
      currentViewportState.consumePan(
        delta = Offset(x = delta.x / density, y = delta.y / density),
        isAutoScroll = true,
      )
    return Offset(x = consumed.x * density, y = consumed.y * density)
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

  override fun requestSoftwareKeyboard() {
    onRequestSoftwareKeyboard?.invoke()
  }

  override fun enqueuePointerCancel() = Unit

  override fun setScrollGestureLocked(locked: Boolean) {
    if (locked) {
      if (scrollGestureLockHandle == null) {
        scrollGestureLockHandle = scrollGestureLockState?.acquire()
      }
    } else {
      releaseScrollGestureLock()
    }
  }

  override fun performSelectionHaptic() {
    onSelectionHaptic?.invoke()
  }

  override fun requestCurrentSelectionHead(version: Long) {
    bringIntoViewRequests?.requestForVersion(
      target = EditorBringIntoViewTarget.CurrentSelectionHead,
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
