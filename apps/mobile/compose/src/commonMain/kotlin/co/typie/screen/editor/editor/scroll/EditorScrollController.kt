package co.typie.screen.editor.editor.scroll

import androidx.compose.foundation.ScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import co.typie.editor.Editor
import co.typie.editor.runtime.EditorUiState
import co.typie.screen.editor.editor.layout.EditorVisibleArea
import co.typie.screen.editor.editor.state.EditorScreenState
import kotlin.math.roundToInt
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

@Composable
internal fun rememberEditorScrollController(
  editorProvider: () -> Editor?,
  uiState: EditorUiState,
  screenState: EditorScreenState,
  visibleArea: EditorVisibleArea,
  scrollPolicy: EditorScrollPolicy,
  density: Float,
): EditorScrollController {
  val scope = rememberCoroutineScope()
  val controller =
    remember(scope, uiState, screenState.scrollState) {
      EditorScrollController(
        scope = scope,
        uiState = uiState,
        scrollState = screenState.scrollState,
      )
    }

  controller.update(
    editorProvider = editorProvider,
    visibleArea = visibleArea,
    scrollPolicy = scrollPolicy,
    headerHeight = screenState.headerHeight,
    density = density,
  )

  return controller
}

internal enum class EditorScrollRequestMode {
  Preferred,
  KeepVisible,
  Typewriter,
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
      keepVisibleRange = EditorScrollRange(),
      typewriterRange = EditorScrollRange(),
      typewriterBottomPadding = 0f,
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

  fun request(
    mode: EditorScrollRequestMode = EditorScrollRequestMode.Preferred,
    target: EditorScrollTarget = EditorScrollTarget.CurrentCursor,
  ) {
    launchScroll {
      if (scrollState.isScrollInProgress) {
        return@launchScroll
      }

      val editor = editorProvider() ?: return@launchScroll
      val rect =
        resolveScrollTargetRect(
          editor = editor,
          uiState = uiState,
          headerHeight = headerHeight,
          target = target,
        ) ?: return@launchScroll
      if (density <= 0f) {
        return@launchScroll
      }

      val currentScrollPx = scrollState.value
      val currentScroll = resolveScrollViewportOffset(scrollPx = currentScrollPx, density = density)
      val resolvedMode = resolveRequestMode(mode, scrollPolicy.mode)
      val targetScroll =
        resolveScrollTargetOffset(
          mode = resolvedMode,
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

private data class EditorScrollTargetRect(val topInContent: Float, val bottomInContent: Float)

private fun resolveScrollTargetRect(
  editor: Editor,
  uiState: EditorUiState,
  headerHeight: Float,
  target: EditorScrollTarget,
): EditorScrollTargetRect? {
  val editorBounds = uiState.editorBoundsInContainer
  if (!editorBounds.isValid) {
    return null
  }

  return when (target) {
    EditorScrollTarget.CurrentCursor -> {
      val cursor = editor.cursor ?: return null
      val cursorOffset =
        uiState.localToGlobal(page = cursor.pageIdx, x = cursor.rect.x, y = cursor.rect.y)
          ?: return null
      val contentTop = headerHeight + editorBounds.y + cursorOffset.y
      EditorScrollTargetRect(
        topInContent = contentTop,
        bottomInContent = contentTop + cursor.rect.height,
      )
    }

    EditorScrollTarget.CurrentSelectionHead -> {
      // TODO(editor-parity): Resolve the true selection-head bounds once the KMP selection
      // model exposes them distinctly from the current cursor snapshot.
      resolveScrollTargetRect(
        editor = editor,
        uiState = uiState,
        headerHeight = headerHeight,
        target = EditorScrollTarget.CurrentCursor,
      )
    }

    is EditorScrollTarget.OverlayRect -> {
      val overlayOffset =
        uiState.localToGlobal(page = target.pageIdx, x = target.left, y = target.top) ?: return null
      val contentTop = headerHeight + editorBounds.y + overlayOffset.y
      EditorScrollTargetRect(
        topInContent = contentTop,
        bottomInContent = contentTop + target.height,
      )
    }
  }
}

private fun resolveRequestMode(
  requestedMode: EditorScrollRequestMode,
  policyMode: EditorScrollMode,
): EditorScrollMode =
  when (requestedMode) {
    EditorScrollRequestMode.Preferred -> policyMode
    EditorScrollRequestMode.KeepVisible -> EditorScrollMode.KeepCursorVisible
    EditorScrollRequestMode.Typewriter -> EditorScrollMode.Typewriter
  }

private fun resolveScrollTargetOffset(
  mode: EditorScrollMode,
  currentScroll: Float,
  rect: EditorScrollTargetRect,
  visibleArea: EditorVisibleArea,
  scrollPolicy: EditorScrollPolicy,
): Float? =
  when (mode) {
    EditorScrollMode.KeepCursorVisible ->
      resolveKeepVisibleScrollTarget(
        currentScroll = currentScroll,
        cursorTopInContent = rect.topInContent,
        cursorBottomInContent = rect.bottomInContent,
        visibleArea = visibleArea,
      )

    EditorScrollMode.Typewriter ->
      resolveEditorScrollTarget(
        currentScroll = currentScroll,
        cursorTopInContent = rect.topInContent,
        cursorBottomInContent = rect.bottomInContent,
        range = scrollPolicy.typewriterRange,
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
