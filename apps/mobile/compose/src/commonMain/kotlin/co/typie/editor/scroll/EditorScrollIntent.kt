package co.typie.editor.scroll

import co.typie.editor.EditorState
import co.typie.editor.VerticalSpan
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolvePageContentTop
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.runtime.EditorUiState

internal data class EditorScrollFrame(
  val state: EditorState,
  val layoutSpec: EditorDocumentLayoutSpec,
  val displayZoom: Float,
  val visibleArea: EditorVisibleArea,
  val autoScrollPolicy: EditorAutoScrollPolicy,
  val headerHeight: Float,
  val density: Float,
  val editorBounds: EditorBoundsInContainer,
)

internal sealed interface EditorBringIntoViewTarget {
  data object CurrentCursorLine : EditorBringIntoViewTarget

  data object CurrentSelectionHead : EditorBringIntoViewTarget

  data class OverlayRect(
    val pageIdx: Int,
    val left: Float,
    val top: Float,
    val width: Float,
    val height: Float,
  ) : EditorBringIntoViewTarget
}

internal sealed interface EditorScrollIntentResult {
  data object Unresolved : EditorScrollIntentResult

  data object ConsumedWithoutScroll : EditorScrollIntentResult

  data class ScrollTo(val y: Float) : EditorScrollIntentResult
}

internal fun resolveEditorScrollIntent(
  frame: EditorScrollFrame,
  target: EditorBringIntoViewTarget,
  currentScroll: Float,
): EditorScrollIntentResult {
  val editorBounds = frame.editorBounds
  if (!editorBounds.isValid) {
    return EditorScrollIntentResult.Unresolved
  }

  val rect =
    resolveBringIntoViewTargetRect(
      state = frame.state,
      layoutSpec = frame.layoutSpec,
      headerHeight = frame.headerHeight,
      editorTopInContainer = editorBounds.y,
      displayZoom = frame.displayZoom,
      density = frame.density,
      target = target,
    ) ?: return EditorScrollIntentResult.ConsumedWithoutScroll

  val targetScroll =
    resolveBringIntoViewTargetOffset(
      mode = frame.autoScrollPolicy.mode,
      currentScroll = currentScroll,
      rect = rect,
      visibleArea = frame.visibleArea,
      autoScrollPolicy = frame.autoScrollPolicy,
    ) ?: return EditorScrollIntentResult.ConsumedWithoutScroll

  return EditorScrollIntentResult.ScrollTo(targetScroll.coerceAtLeast(0f))
}

internal fun resolveDistanceToPagesBottom(
  state: EditorState,
  layoutSpec: EditorDocumentLayoutSpec,
  uiState: EditorUiState,
  headerHeight: Float,
  pagesContentHeight: Float,
  target: EditorBringIntoViewTarget,
  density: Float = 0f,
): Float? {
  val editorBounds = uiState.editorBoundsInContainer
  if (!editorBounds.isValid) {
    return null
  }
  val rect =
    resolveBringIntoViewTargetRect(
      state = state,
      layoutSpec = layoutSpec,
      headerHeight = headerHeight,
      editorTopInContainer = editorBounds.y,
      displayZoom = uiState.displayZoom,
      density = density,
      target = target,
    ) ?: return null
  val contentBottomInContent = headerHeight + editorBounds.y + pagesContentHeight
  return (contentBottomInContent - rect.top).coerceAtLeast(0f)
}

private fun resolveBringIntoViewTargetRect(
  state: EditorState,
  layoutSpec: EditorDocumentLayoutSpec,
  headerHeight: Float,
  editorTopInContainer: Float,
  displayZoom: Float,
  density: Float,
  target: EditorBringIntoViewTarget,
): VerticalSpan? {
  return when (target) {
    EditorBringIntoViewTarget.CurrentCursorLine -> {
      val cursor = state.cursor ?: return null
      val lineOffsetY =
        resolveTargetOffsetY(
          layoutSpec = layoutSpec,
          pageSizes = state.pageSizes,
          page = cursor.pageIdx,
          y = cursor.line.y,
          displayZoom = displayZoom,
          density = density,
        ) ?: return null
      val lineContentTop = headerHeight + editorTopInContainer + lineOffsetY
      VerticalSpan(top = lineContentTop, bottom = lineContentTop + cursor.line.height * displayZoom)
    }

    EditorBringIntoViewTarget.CurrentSelectionHead -> {
      // TODO(editor-parity): KMP selection 모델/FFI가 실제 selection head bounds를 노출하면
      // 그 값을 써야 한다. 지금은 CurrentSelectionHead가 CurrentCursorLine으로 fallback 되어
      // non-collapsed selection의 typewriter/keep-visible 기준이 웹/플러터와 다르다.
      resolveBringIntoViewTargetRect(
        state = state,
        layoutSpec = layoutSpec,
        headerHeight = headerHeight,
        editorTopInContainer = editorTopInContainer,
        displayZoom = displayZoom,
        density = density,
        target = EditorBringIntoViewTarget.CurrentCursorLine,
      )
    }

    is EditorBringIntoViewTarget.OverlayRect -> {
      val overlayOffsetY =
        resolveTargetOffsetY(
          layoutSpec = layoutSpec,
          pageSizes = state.pageSizes,
          page = target.pageIdx,
          y = target.top,
          displayZoom = displayZoom,
          density = density,
        ) ?: return null
      val contentTop = headerHeight + editorTopInContainer + overlayOffsetY
      VerticalSpan(top = contentTop, bottom = contentTop + target.height * displayZoom)
    }
  }
}

private fun resolveTargetOffsetY(
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<PageSize>,
  page: Int,
  y: Float,
  displayZoom: Float,
  density: Float,
): Float? {
  val pageTop =
    layoutSpec.resolvePageContentTop(
      pageSizes = pageSizes,
      page = page,
      displayZoom = displayZoom,
      density = density,
    ) ?: return null
  return pageTop + y * normalizeDisplayZoom(displayZoom)
}

private fun normalizeDisplayZoom(displayZoom: Float): Float =
  if (displayZoom.isFinite() && displayZoom > 0f) {
    displayZoom
  } else {
    1f
  }

private fun resolveBringIntoViewTargetOffset(
  mode: EditorAutoScrollMode,
  currentScroll: Float,
  rect: VerticalSpan,
  visibleArea: EditorVisibleArea,
  autoScrollPolicy: EditorAutoScrollPolicy,
): Float? =
  when (mode) {
    EditorAutoScrollMode.KeepCursorVisible ->
      resolveKeepVisibleScrollOffset(
        currentScroll = currentScroll,
        targetTopInContent = rect.top,
        targetBottomInContent = rect.bottom,
        visibleArea = visibleArea,
      )

    EditorAutoScrollMode.Typewriter ->
      resolveTypewriterScrollOffset(
        currentScroll = currentScroll,
        targetTopInContent = rect.top,
        targetBottomInContent = rect.bottom,
        visibleArea = visibleArea,
        position = autoScrollPolicy.typewriterPosition,
      )
  }
