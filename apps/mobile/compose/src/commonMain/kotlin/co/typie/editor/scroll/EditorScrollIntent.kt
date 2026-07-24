package co.typie.editor.scroll

import co.typie.editor.EditorState
import co.typie.editor.VerticalSpan
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.PageRect
import co.typie.editor.pageRectsToContentRect
import co.typie.editor.runtime.EditorBoundsInContainer

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
  data object CurrentSelectionHead : EditorBringIntoViewTarget

  data class PageRects(val rects: List<PageRect>) : EditorBringIntoViewTarget
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
  layoutSpec: EditorDocumentLayoutSpec,
  target: EditorBringIntoViewTarget,
  displayZoom: Float,
  density: Float = 0f,
): Float? {
  val targetRects =
    resolveBringIntoViewTargetPageRects(state = state, target = target) ?: return null
  return pageRectsToContentRect(
      rects = targetRects,
      layoutSpec = layoutSpec,
      pageSizes = state.pageSizes,
      displayZoom = displayZoom,
      density = density,
    )
    ?.height
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
  val targetRects =
    resolveBringIntoViewTargetPageRects(state = state, target = target) ?: return null
  val contentRect =
    pageRectsToContentRect(
      rects = targetRects,
      layoutSpec = layoutSpec,
      pageSizes = state.pageSizes,
      displayZoom = displayZoom,
      density = density,
      contentOriginY = headerHeight + editorTopInContainer,
    ) ?: return null
  return VerticalSpan(top = contentRect.top, bottom = contentRect.bottom)
}

private fun resolveBringIntoViewTargetPageRects(
  state: EditorState,
  target: EditorBringIntoViewTarget,
): List<PageRect>? =
  when (target) {
    EditorBringIntoViewTarget.CurrentSelectionHead ->
      resolveCurrentSelectionHeadPageRect(state)?.let(::listOf)
    is EditorBringIntoViewTarget.PageRects -> target.rects.takeIf { it.isNotEmpty() }
  }

internal fun List<PageRect>.toPageRectsTarget(): EditorBringIntoViewTarget.PageRects? =
  takeIf { it.isNotEmpty() }?.let(EditorBringIntoViewTarget::PageRects)

private fun resolveCollapsedSelectionHeadPageRect(state: EditorState): PageRect? {
  val cursor = state.cursor ?: return null
  return PageRect(pageIdx = cursor.pageIdx, rect = cursor.line)
}

private fun resolveCurrentSelectionHeadPageRect(state: EditorState): PageRect? {
  val selection = state.selection ?: return null
  if (selection.anchor == selection.head) {
    return resolveCollapsedSelectionHeadPageRect(state)
  }
  val endpoints = state.selectionEndpoints ?: return null
  val headRect =
    when (selection.head) {
      endpoints.toPosition -> endpoints.to
      endpoints.fromPosition -> endpoints.from
      else -> return null
    }
  return headRect
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
