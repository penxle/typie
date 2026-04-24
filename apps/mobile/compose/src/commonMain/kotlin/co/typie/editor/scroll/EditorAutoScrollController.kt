package co.typie.editor.scroll

import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.EditorViewportTransform
import co.typie.editor.VerticalSpan
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

@Composable
internal fun rememberEditorAutoScrollController(
  editorProvider: () -> Editor?,
  uiState: EditorUiState,
  viewportState: EditorViewportState,
  isDirectScrollInProgress: () -> Boolean,
  visibleArea: EditorVisibleArea,
  autoScrollPolicy: EditorAutoScrollPolicy,
  headerHeight: Float,
): EditorAutoScrollController {
  val scope = rememberCoroutineScope()
  val controller =
    remember(scope, uiState, viewportState, isDirectScrollInProgress) {
      EditorAutoScrollController(
        scope = scope,
        uiState = uiState,
        viewportState = viewportState,
        isDirectScrollInProgress = isDirectScrollInProgress,
      )
    }

  controller.update(
    editorProvider = editorProvider,
    visibleArea = visibleArea,
    autoScrollPolicy = autoScrollPolicy,
    headerHeight = headerHeight,
  )

  return controller
}

internal sealed interface EditorScrollTarget {
  data object CurrentCursorLine : EditorScrollTarget

  data object CurrentSelectionHead : EditorScrollTarget

  data class OverlayRect(
    val pageIdx: Int,
    val left: Float,
    val top: Float,
    val width: Float,
    val height: Float,
  ) : EditorScrollTarget
}

internal val LocalEditorAutoScrollController =
  compositionLocalOf<EditorAutoScrollController?> { null }

internal class EditorAutoScrollController(
  private val scope: CoroutineScope,
  private val uiState: EditorUiState,
  private val viewportState: EditorViewportState,
  private val isDirectScrollInProgress: () -> Boolean,
) {
  private var editorProvider: () -> Editor? = { null }
  private var visibleArea: EditorVisibleArea = EditorVisibleArea()
  private var autoScrollPolicy: EditorAutoScrollPolicy =
    EditorAutoScrollPolicy(
      mode = EditorAutoScrollMode.KeepCursorVisible,
      typewriterPosition = 0.5f,
      keepVisibleRange = VerticalSpan(),
      targetTop = null,
      targetLineHeight = 0f,
      bottomSpacerHeight = 0f,
    )
  private var headerHeight: Float = 0f
  private var activeJob: Job? = null

  fun update(
    editorProvider: () -> Editor?,
    visibleArea: EditorVisibleArea,
    autoScrollPolicy: EditorAutoScrollPolicy,
    headerHeight: Float,
  ) {
    this.editorProvider = editorProvider
    this.visibleArea = visibleArea
    this.autoScrollPolicy = autoScrollPolicy
    this.headerHeight = headerHeight
  }

  fun request(
    target: EditorScrollTarget = EditorScrollTarget.CurrentCursorLine,
    state: EditorState? = null,
  ) {
    launchScroll {
      if (viewportState.isTransforming || isDirectScrollInProgress()) {
        return@launchScroll
      }

      val editor = editorProvider() ?: return@launchScroll
      val snapshot = state ?: editor.state
      val editorBounds = uiState.editorBoundsInContainer
      if (!editorBounds.isValid) {
        return@launchScroll
      }
      val viewportTransform = uiState.resolveViewportTransform(pageSizes = snapshot.pageSizes)
      val rect =
        resolveScrollTargetRect(
          snapshot = snapshot,
          viewportTransform = viewportTransform,
          headerHeight = headerHeight,
          editorTopInContainer = editorBounds.y,
          displayZoom = uiState.displayZoom,
          target = target,
        ) ?: return@launchScroll
      val currentScroll = viewportState.scrollOffset.y
      val targetScroll =
        resolveScrollTargetOffset(
          mode = autoScrollPolicy.mode,
          currentScroll = currentScroll,
          rect = rect,
          visibleArea = visibleArea,
          autoScrollPolicy = autoScrollPolicy,
        ) ?: return@launchScroll
      val targetViewportY =
        resolveScrollViewportY(targetScroll = targetScroll, maxScrollY = viewportState.maxScrollY)
      val deltaY = targetViewportY - currentScroll
      if (deltaY != 0f) {
        viewportState.dispatchDeltaY(deltaY = deltaY, isAutoScroll = true)
      }
    }
  }

  fun cancel() {
    cancelActiveScroll()
  }

  private fun launchScroll(block: suspend () -> Unit): Job {
    cancelActiveScroll()
    val job = scope.launch { block() }
    activeJob = job
    job.invokeOnCompletion {
      if (activeJob === job) {
        activeJob = null
      }
    }
    return job
  }

  private fun cancelActiveScroll() {
    activeJob?.cancel()
    activeJob = null
  }
}

internal fun resolveDistanceToPagesBottom(
  state: EditorState,
  uiState: EditorUiState,
  headerHeight: Float,
  pagesContentHeight: Float,
  bottomOcclusion: Float,
  target: EditorScrollTarget,
): Float? {
  val viewportTransform = uiState.resolveViewportTransform(pageSizes = state.pageSizes)
  val editorBounds = uiState.editorBoundsInContainer
  if (!editorBounds.isValid) {
    return null
  }
  val rect =
    resolveScrollTargetRect(
      snapshot = state,
      viewportTransform = viewportTransform,
      headerHeight = headerHeight,
      editorTopInContainer = editorBounds.y,
      displayZoom = uiState.displayZoom,
      target = target,
    ) ?: return null
  val contentBottomInContent = headerHeight + editorBounds.y + pagesContentHeight
  return (contentBottomInContent - rect.top + bottomOcclusion).coerceAtLeast(0f)
}

private fun resolveScrollTargetRect(
  snapshot: EditorState,
  viewportTransform: EditorViewportTransform,
  headerHeight: Float,
  editorTopInContainer: Float,
  displayZoom: Float,
  target: EditorScrollTarget,
): VerticalSpan? {
  return when (target) {
    EditorScrollTarget.CurrentCursorLine -> {
      val cursor = snapshot.cursor ?: return null
      val cursorLineOffset =
        viewportTransform.localToGlobal(page = cursor.pageIdx, x = cursor.line.x, y = cursor.line.y)
          ?: return null
      val lineContentTop = headerHeight + editorTopInContainer + cursorLineOffset.y
      VerticalSpan(top = lineContentTop, bottom = lineContentTop + cursor.line.height * displayZoom)
    }

    EditorScrollTarget.CurrentSelectionHead -> {
      // TODO(editor-parity): KMP selection 모델/FFI가 실제 selection head bounds를 노출하면
      // 그 값을 써야 한다. 지금은 CurrentSelectionHead가 CurrentCursorLine으로 fallback 되어
      // non-collapsed selection의 typewriter/keep-visible 기준이 웹/플러터와 다르다.
      resolveScrollTargetRect(
        snapshot = snapshot,
        viewportTransform = viewportTransform,
        headerHeight = headerHeight,
        editorTopInContainer = editorTopInContainer,
        displayZoom = displayZoom,
        target = EditorScrollTarget.CurrentCursorLine,
      )
    }

    is EditorScrollTarget.OverlayRect -> {
      val overlayOffset =
        viewportTransform.localToGlobal(page = target.pageIdx, x = target.left, y = target.top)
          ?: return null
      val contentTop = headerHeight + editorTopInContainer + overlayOffset.y
      VerticalSpan(top = contentTop, bottom = contentTop + target.height * displayZoom)
    }
  }
}

private fun resolveScrollTargetOffset(
  mode: EditorAutoScrollMode,
  currentScroll: Float,
  rect: VerticalSpan,
  visibleArea: EditorVisibleArea,
  autoScrollPolicy: EditorAutoScrollPolicy,
): Float? =
  when (mode) {
    EditorAutoScrollMode.KeepCursorVisible ->
      resolveKeepVisibleScrollTarget(
        currentScroll = currentScroll,
        targetTopInContent = rect.top,
        targetBottomInContent = rect.bottom,
        visibleArea = visibleArea,
      )

    EditorAutoScrollMode.Typewriter ->
      resolveTypewriterScrollTarget(
        currentScroll = currentScroll,
        targetTopInContent = rect.top,
        targetBottomInContent = rect.bottom,
        visibleArea = visibleArea,
        position = autoScrollPolicy.typewriterPosition,
      )
  }

private fun resolveScrollViewportY(targetScroll: Float, maxScrollY: Float): Float =
  targetScroll.coerceIn(0f, maxScrollY)
