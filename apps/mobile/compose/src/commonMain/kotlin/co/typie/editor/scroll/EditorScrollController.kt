package co.typie.editor.scroll

import androidx.compose.foundation.ScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import co.typie.editor.Editor
import co.typie.editor.EditorViewportTransform
import co.typie.editor.VerticalSpan
import co.typie.editor.runtime.EditorUiState
import kotlin.math.roundToInt
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

@Composable
internal fun rememberEditorScrollController(
  editorProvider: () -> Editor?,
  uiState: EditorUiState,
  scrollState: ScrollState,
  visibleArea: EditorVisibleArea,
  scrollPolicy: EditorScrollPolicy,
  headerHeight: Float,
  density: Float,
): EditorScrollController {
  val scope = rememberCoroutineScope()
  val controller =
    remember(scope, uiState, scrollState) {
      EditorScrollController(scope = scope, uiState = uiState, scrollState = scrollState)
    }

  controller.update(
    editorProvider = editorProvider,
    visibleArea = visibleArea,
    scrollPolicy = scrollPolicy,
    headerHeight = headerHeight,
    density = density,
  )

  return controller
}

internal sealed interface EditorScrollTarget {
  data object CurrentCursor : EditorScrollTarget

  data object CurrentSelectionHead : EditorScrollTarget

  data class OverlayRect(
    val pageIdx: Int,
    val left: Float,
    val top: Float,
    val width: Float,
    val height: Float,
  ) : EditorScrollTarget
}

internal val LocalEditorScrollController = compositionLocalOf<EditorScrollController?> { null }

internal class EditorScrollController(
  private val scope: CoroutineScope,
  private val uiState: EditorUiState,
  private val scrollState: ScrollState,
) {
  private var editorProvider: () -> Editor? = { null }
  private var visibleArea: EditorVisibleArea = EditorVisibleArea()
  private var scrollPolicy: EditorScrollPolicy =
    EditorScrollPolicy(
      mode = EditorScrollMode.KeepCursorVisible,
      typewriterPosition = 0.5f,
      keepVisibleRange = VerticalSpan(),
      typewriterTargetTop = null,
      typewriterCursorHeight = 0f,
      bottomSpacerHeight = 0f,
    )
  private var headerHeight: Float = 0f
  private var density: Float = 1f
  private var activeJob: Job? = null

  fun update(
    editorProvider: () -> Editor?,
    visibleArea: EditorVisibleArea,
    scrollPolicy: EditorScrollPolicy,
    headerHeight: Float,
    density: Float,
  ) {
    this.editorProvider = editorProvider
    this.visibleArea = visibleArea
    this.scrollPolicy = scrollPolicy
    this.headerHeight = headerHeight
    this.density = density
  }

  fun request(target: EditorScrollTarget = EditorScrollTarget.CurrentCursor) {
    launchScroll {
      if (scrollState.isScrollInProgress) {
        return@launchScroll
      }

      val editor = editorProvider() ?: return@launchScroll
      val editorBounds = uiState.editorBoundsInContainer
      if (!editorBounds.isValid) {
        return@launchScroll
      }
      val viewportTransform = uiState.resolveViewportTransform(pageSizes = editor.pageSizes)
      val rect =
        resolveScrollTargetRect(
          editor = editor,
          viewportTransform = viewportTransform,
          headerHeight = headerHeight,
          editorTopInContainer = editorBounds.y,
          displayZoom = uiState.displayZoom,
          target = target,
        ) ?: return@launchScroll
      if (density <= 0f) {
        return@launchScroll
      }

      val currentScrollPx = scrollState.value
      val currentScroll = resolveScrollViewportOffset(scrollPx = currentScrollPx, density = density)
      val targetScroll =
        resolveScrollTargetOffset(
          mode = scrollPolicy.mode,
          currentScroll = currentScroll,
          rect = rect,
          visibleArea = visibleArea,
          scrollPolicy = scrollPolicy,
        ) ?: return@launchScroll
      val targetPx =
        resolveScrollPx(
          targetScroll = targetScroll,
          density = density,
          maxScrollPx = scrollState.maxValue,
        ) ?: return@launchScroll
      val deltaPx = targetPx - currentScrollPx
      if (deltaPx != 0) {
        scrollState.dispatchRawDelta(deltaPx.toFloat())
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
  editor: Editor,
  uiState: EditorUiState,
  headerHeight: Float,
  pagesContentHeight: Float,
  bottomOcclusion: Float,
  target: EditorScrollTarget,
): Float? {
  val viewportTransform = uiState.resolveViewportTransform(pageSizes = editor.pageSizes)
  val editorBounds = uiState.editorBoundsInContainer
  if (!editorBounds.isValid) {
    return null
  }
  val rect =
    resolveScrollTargetRect(
      editor = editor,
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
  editor: Editor,
  viewportTransform: EditorViewportTransform,
  headerHeight: Float,
  editorTopInContainer: Float,
  displayZoom: Float,
  target: EditorScrollTarget,
): VerticalSpan? {
  return when (target) {
    EditorScrollTarget.CurrentCursor -> {
      val cursor = editor.cursor ?: return null
      val cursorOffset =
        viewportTransform.localToGlobal(page = cursor.pageIdx, x = cursor.rect.x, y = cursor.rect.y)
          ?: return null
      val contentTop = headerHeight + editorTopInContainer + cursorOffset.y
      VerticalSpan(top = contentTop, bottom = contentTop + cursor.rect.height * displayZoom)
    }

    EditorScrollTarget.CurrentSelectionHead -> {
      // TODO(editor-parity): KMP selection 모델/FFI가 실제 selection head bounds를 노출하면
      // 그 값을 써야 한다. 지금은 CurrentSelectionHead가 CurrentCursor로 fallback 되어
      // non-collapsed selection의 typewriter/keep-visible 기준이 웹/플러터와 다르고,
      // collapsed selection에서는 표시 높이 부족분이 displayZoom만큼 확대된다.
      resolveScrollTargetRect(
        editor = editor,
        viewportTransform = viewportTransform,
        headerHeight = headerHeight,
        editorTopInContainer = editorTopInContainer,
        displayZoom = displayZoom,
        target = EditorScrollTarget.CurrentCursor,
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
  mode: EditorScrollMode,
  currentScroll: Float,
  rect: VerticalSpan,
  visibleArea: EditorVisibleArea,
  scrollPolicy: EditorScrollPolicy,
): Float? =
  when (mode) {
    EditorScrollMode.KeepCursorVisible ->
      resolveKeepVisibleScrollTarget(
        currentScroll = currentScroll,
        cursorTopInContent = rect.top,
        cursorBottomInContent = rect.bottom,
        visibleArea = visibleArea,
      )

    EditorScrollMode.Typewriter ->
      resolveTypewriterScrollTarget(
        currentScroll = currentScroll,
        cursorTopInContent = rect.top,
        cursorBottomInContent = rect.bottom,
        visibleArea = visibleArea,
        position = scrollPolicy.typewriterPosition,
      )
  }

private fun resolveScrollViewportOffset(scrollPx: Int, density: Float): Float {
  if (density <= 0f) {
    return 0f
  }

  return scrollPx / density
}

private fun resolveScrollPx(targetScroll: Float, density: Float, maxScrollPx: Int): Int? {
  if (density <= 0f) {
    return null
  }

  return (targetScroll * density).roundToInt().coerceIn(0, maxScrollPx)
}
