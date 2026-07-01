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
    )
  if (rect == null) {
    return EditorScrollIntentResult.ConsumedWithoutScroll
  }

  val targetScroll =
    resolveBringIntoViewTargetOffset(
      mode = frame.autoScrollPolicy.mode,
      currentScroll = currentScroll,
      rect = rect,
      visibleArea = frame.visibleArea,
      autoScrollPolicy = frame.autoScrollPolicy,
    )
  if (targetScroll == null) {
    return EditorScrollIntentResult.ConsumedWithoutScroll
  }

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

internal fun isEditorScrollTargetVisible(
  frame: EditorScrollFrame,
  target: EditorBringIntoViewTarget,
  currentScroll: Float,
  visibleArea: EditorVisibleArea,
): Boolean? {
  val editorBounds = frame.editorBounds
  if (!editorBounds.isValid) return null
  val rect =
    resolveBringIntoViewTargetRect(
      state = frame.state,
      layoutSpec = frame.layoutSpec,
      headerHeight = frame.headerHeight,
      editorTopInContainer = editorBounds.y,
      displayZoom = frame.displayZoom,
      density = frame.density,
      target = target,
    ) ?: return null
  val visibleTopInContent = currentScroll + visibleArea.visibleViewportTop
  val visibleBottomInContent = currentScroll + visibleArea.visibleViewportBottom
  if (visibleBottomInContent <= visibleTopInContent) {
    return false
  }
  return rect.bottom >= visibleTopInContent && rect.top <= visibleBottomInContent
}

internal fun resolveBringIntoViewTargetHeight(
  state: EditorState,
  target: EditorBringIntoViewTarget,
  displayZoom: Float,
): Float? {
  val targetRect = resolveBringIntoViewTargetPageRect(state = state, target = target) ?: return null
  return targetRect.height * displayZoom
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
  val targetRect = resolveBringIntoViewTargetPageRect(state = state, target = target) ?: return null
  val offsetY =
    resolveTargetOffsetY(
      layoutSpec = layoutSpec,
      pageSizes = state.pageSizes,
      page = targetRect.pageIdx,
      y = targetRect.y,
      displayZoom = displayZoom,
      density = density,
    ) ?: return null
  val contentTop = headerHeight + editorTopInContainer + offsetY
  return VerticalSpan(top = contentTop, bottom = contentTop + targetRect.height * displayZoom)
}

private data class BringIntoViewTargetPageRect(val pageIdx: Int, val y: Float, val height: Float)

private fun resolveBringIntoViewTargetPageRect(
  state: EditorState,
  target: EditorBringIntoViewTarget,
): BringIntoViewTargetPageRect? =
  when (target) {
    EditorBringIntoViewTarget.CurrentCursorLine -> resolveCurrentCursorLinePageRect(state)
    EditorBringIntoViewTarget.CurrentSelectionHead -> resolveCurrentSelectionHeadPageRect(state)
    is EditorBringIntoViewTarget.OverlayRect ->
      BringIntoViewTargetPageRect(pageIdx = target.pageIdx, y = target.top, height = target.height)
  }

private fun resolveCurrentCursorLinePageRect(state: EditorState): BringIntoViewTargetPageRect? {
  val cursor = state.cursor ?: return null
  return BringIntoViewTargetPageRect(
    pageIdx = cursor.pageIdx,
    y = cursor.line.y,
    height = cursor.line.height,
  )
}

private fun resolveCurrentSelectionHeadPageRect(state: EditorState): BringIntoViewTargetPageRect? {
  val selection = state.selection ?: return null
  if (selection.anchor == selection.head) {
    return resolveCurrentCursorLinePageRect(state)
  }
  val endpoints = state.selectionEndpoints ?: return null
  val headRect =
    when (selection.head) {
      endpoints.toPosition -> endpoints.to
      endpoints.fromPosition -> endpoints.from
      else -> return null
    }
  return BringIntoViewTargetPageRect(
    pageIdx = headRect.pageIdx,
    y = headRect.rect.y,
    height = headRect.rect.height,
  )
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
