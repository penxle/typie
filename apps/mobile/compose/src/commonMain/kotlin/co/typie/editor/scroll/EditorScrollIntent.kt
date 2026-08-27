package co.typie.editor.scroll

import co.typie.editor.EditorState
import co.typie.editor.VerticalSpan
import co.typie.editor.body.EditorBodyGeometry
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolveEditorBodyGeometry
import co.typie.editor.body.resolveMeasuredPageLength
import co.typie.editor.body.resolvePageContentTopPrefixes
import co.typie.editor.ffi.PageRect
import co.typie.editor.pageRectsToContentRect
import co.typie.editor.runtime.EditorBoundsInContainer
import kotlin.math.abs

internal data class EditorScrollFrame(
  val state: EditorState,
  val layoutSpec: EditorDocumentLayoutSpec,
  val displayZoom: Float,
  val visibleArea: EditorVisibleArea,
  val autoScrollPolicy: EditorAutoScrollPolicy,
  val headerHeight: Float,
  val density: Float,
  val editorBounds: EditorBoundsInContainer,
) {
  val bodyGeometry: EditorBodyGeometry by
    lazy(LazyThreadSafetyMode.NONE) {
      resolveEditorBodyGeometry(
        visibleArea = visibleArea,
        layoutSpec = layoutSpec,
        pageSizes = state.pageSizes,
        displayZoom = displayZoom,
      )
    }

  val pageContentTopPrefixes: FloatArray by
    lazy(LazyThreadSafetyMode.NONE) {
      layoutSpec.resolvePageContentTopPrefixes(
        pageSizes = state.pageSizes,
        displayZoom = displayZoom,
        density = density,
      )
    }

  val pagesContentHeight: Float
    get() = pageContentTopPrefixes.lastOrNull() ?: 0f

  fun pageContentTop(page: Int): Float? {
    if (page !in state.pageSizes.indices) return null
    return pageContentTopPrefixes[page]
  }

  fun pageAtContentY(y: Float): Int? {
    if (state.pageSizes.isEmpty() || !y.isFinite()) return null
    var low = 0
    var high = state.pageSizes.lastIndex
    while (low < high) {
      val middle = (low + high + 1) ushr 1
      if (pageContentTopPrefixes[middle] <= y) low = middle else high = middle - 1
    }
    if (low == state.pageSizes.lastIndex) return low

    val pageBottom =
      pageContentTopPrefixes[low] +
        resolveMeasuredPageLength(
          length = state.pageSizes[low].height,
          displayZoom = displayZoom,
          density = density,
        )
    val nextPageTop = pageContentTopPrefixes[low + 1]
    return if (y >= (pageBottom + nextPageTop) / 2f) low + 1 else low
  }

  fun withState(state: EditorState): EditorScrollFrame {
    return copy(
      state = state,
      autoScrollPolicy =
        autoScrollPolicy.resolveForState(
          state = state,
          visibleArea = visibleArea,
          layoutSpec = layoutSpec,
          displayZoom = displayZoom,
          density = density,
        ),
    )
  }
}

internal fun EditorAutoScrollPolicy.resolveForState(
  state: EditorState,
  visibleArea: EditorVisibleArea,
  layoutSpec: EditorDocumentLayoutSpec,
  displayZoom: Float,
  density: Float,
): EditorAutoScrollPolicy {
  val targetLineHeight =
    resolveBringIntoViewTargetHeight(
      state = state,
      layoutSpec = layoutSpec,
      target = EditorBringIntoViewTarget.CurrentSelectionHead,
      displayZoom = displayZoom,
      density = density,
    )
  return resolveForState(
    visibleArea = visibleArea,
    typewriterActive = targetLineHeight != null,
    targetLineHeight = targetLineHeight ?: 0f,
  )
}

internal sealed interface EditorBringIntoViewTarget {
  data object CurrentSelectionHead : EditorBringIntoViewTarget

  data class TrackedItem(val id: String) : EditorBringIntoViewTarget
}

internal enum class EditorBringIntoViewPolicy {
  CursorGuard,
  PointerCursorGuard,
  Typewriter,
  Reveal,
}

internal sealed interface EditorScrollIntentResult {
  data object Unresolved : EditorScrollIntentResult

  data object NoScroll : EditorScrollIntentResult

  data class ScrollTo(val y: Float) : EditorScrollIntentResult
}

internal fun resolveEditorScrollIntent(
  frame: EditorScrollFrame,
  target: EditorBringIntoViewTarget,
  policy: EditorBringIntoViewPolicy,
  currentScroll: Float,
  maximumScrollY: Float = Float.POSITIVE_INFINITY,
): EditorScrollIntentResult {
  val editorBounds = frame.editorBounds
  if (!editorBounds.isValid) {
    return EditorScrollIntentResult.Unresolved
  }

  return resolveEditorScrollIntent(
    frame = frame,
    target = target,
    policy = policy,
    currentScroll = currentScroll,
    contentOriginY = frame.headerHeight + editorBounds.y,
    maximumScrollY = maximumScrollY,
  )
}

internal fun resolveEditorScrollIntent(
  frame: EditorScrollFrame,
  target: EditorBringIntoViewTarget,
  policy: EditorBringIntoViewPolicy,
  currentScroll: Float,
  contentOriginY: Float,
  maximumScrollY: Float = Float.POSITIVE_INFINITY,
): EditorScrollIntentResult {
  if (!contentOriginY.isFinite()) return EditorScrollIntentResult.Unresolved

  val rect =
    resolveBringIntoViewTargetRect(frame = frame, contentOriginY = contentOriginY, target = target)
  if (rect == null) {
    return EditorScrollIntentResult.NoScroll
  }

  val targetScroll =
    resolveBringIntoViewTargetOffset(
      policy = frame.resolvePolicy(target = target, requestedPolicy = policy),
      currentScroll = currentScroll,
      rect = rect,
      visibleArea = frame.visibleArea,
      autoScrollPolicy = frame.autoScrollPolicy,
      maximumScrollY = maximumScrollY,
      oversizedMinimumVisibleHeight =
        if (policy == EditorBringIntoViewPolicy.PointerCursorGuard) {
          CursorVisibleMargin
        } else {
          null
        },
    )
  if (targetScroll == null) {
    return EditorScrollIntentResult.NoScroll
  }

  return EditorScrollIntentResult.ScrollTo(targetScroll)
}

internal fun resolveInstantRevealPreparationViewports(
  frame: EditorScrollFrame,
  target: EditorBringIntoViewTarget,
  policy: EditorBringIntoViewPolicy,
  currentScroll: Float,
  contentOriginY: Float,
  maximumScrollY: Float,
): List<VerticalSpan> {
  val rect =
    resolveBringIntoViewTargetRect(frame = frame, contentOriginY = contentOriginY, target = target)
      ?: return emptyList()
  return resolveInstantRevealPreparationViewports(
    currentScroll = currentScroll,
    viewportHeight = frame.visibleArea.viewport.height,
    maximumScrollY = maximumScrollY,
    target = rect,
    visibleArea = frame.visibleArea,
    autoScrollPolicy = frame.autoScrollPolicy,
    policy = frame.resolvePolicy(target = target, requestedPolicy = policy),
    oversizedMinimumVisibleHeight =
      if (policy == EditorBringIntoViewPolicy.PointerCursorGuard) {
        CursorVisibleMargin
      } else {
        null
      },
  )
}

internal fun resolveInstantRevealPreparationViewports(
  currentScroll: Float,
  viewportHeight: Float,
  maximumScrollY: Float,
  target: VerticalSpan,
  visibleArea: EditorVisibleArea,
  autoScrollPolicy: EditorAutoScrollPolicy,
  policy: EditorBringIntoViewPolicy,
  oversizedMinimumVisibleHeight: Float? = null,
): List<VerticalSpan> {
  if (
    !currentScroll.isFinite() ||
      !viewportHeight.isFinite() ||
      viewportHeight <= 0f ||
      !maximumScrollY.isFinite() ||
      maximumScrollY < 0f ||
      !target.top.isFinite() ||
      !target.bottom.isFinite() ||
      target.bottom < target.top
  ) {
    return emptyList()
  }

  val keepVisibleRange = resolveKeepVisibleRange(visibleArea)
  val preparationPolicy =
    if (
      policy == EditorBringIntoViewPolicy.Typewriter &&
        (!keepVisibleRange.isValid || target.height > keepVisibleRange.height)
    ) {
      EditorBringIntoViewPolicy.CursorGuard
    } else {
      policy
    }
  val destinations =
    when (preparationPolicy) {
      EditorBringIntoViewPolicy.Typewriter ->
        listOf(
          resolveTypewriterScrollOffset(
            currentScroll = currentScroll,
            targetTopInContent = target.top,
            targetBottomInContent = target.bottom,
            visibleArea = visibleArea,
            position = autoScrollPolicy.typewriterPosition,
            maximumScrollY = maximumScrollY,
          ) ?: currentScroll
        )

      EditorBringIntoViewPolicy.CursorGuard,
      EditorBringIntoViewPolicy.PointerCursorGuard,
      EditorBringIntoViewPolicy.Reveal -> {
        val range = keepVisibleRange
        if (!range.isValid) {
          listOf(
            resolveKeepVisibleScrollOffset(
              currentScroll = currentScroll,
              targetTopInContent = target.top,
              targetBottomInContent = target.bottom,
              visibleArea = visibleArea,
              maximumScrollY = maximumScrollY,
            ) ?: currentScroll
          )
        } else {
          buildList {
            if (target.height > range.height) {
              if (oversizedMinimumVisibleHeight == null) {
                add(currentScroll)
                add(target.top - range.top)
                add(target.bottom - range.bottom)
              } else {
                val minimumVisibleHeight = oversizedMinimumVisibleHeight.coerceIn(0f, range.height)
                add(currentScroll)
                add(target.top - (range.bottom - minimumVisibleHeight))
                add(target.bottom - (range.top + minimumVisibleHeight))
              }
            } else {
              val targetScroll =
                resolveKeepVisibleScrollOffset(
                  currentScroll = currentScroll,
                  targetTopInContent = target.top,
                  targetBottomInContent = target.bottom,
                  visibleArea = visibleArea,
                  maximumScrollY = maximumScrollY,
                )
              if (targetScroll == null) {
                add(currentScroll)
              }
              add(target.top - range.top)
              add(target.bottom - range.bottom)
            }
          }
        }
      }
    }

  return buildList {
    destinations.forEach { destination ->
      val top = destination.coerceIn(0f, maximumScrollY)
      if (none { abs(it.top - top) <= 1f })
        add(VerticalSpan(top = top, bottom = top + viewportHeight))
    }
  }
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
      frame = frame,
      contentOriginY = frame.headerHeight + editorBounds.y,
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
  if (target == EditorBringIntoViewTarget.CurrentSelectionHead) {
    val pageRect = resolveCurrentSelectionHeadPageRect(state) ?: return null
    if (pageRect.pageIdx !in state.pageSizes.indices) return null
    val rect = pageRect.rect
    if (
      !rect.x.isFinite() || !rect.y.isFinite() || !rect.width.isFinite() || !rect.height.isFinite()
    ) {
      return null
    }
    val zoom = displayZoom.takeIf { it.isFinite() && it > 0f } ?: 1f
    return abs(rect.height * zoom).takeIf(Float::isFinite)
  }

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
  frame: EditorScrollFrame,
  contentOriginY: Float,
  target: EditorBringIntoViewTarget,
): VerticalSpan? {
  val targetRects =
    resolveBringIntoViewTargetPageRects(state = frame.state, target = target) ?: return null
  val contentRect =
    pageRectsToContentRect(
      rects = targetRects,
      layoutSpec = frame.layoutSpec,
      pageSizes = frame.state.pageSizes,
      pageContentTops = frame.pageContentTopPrefixes,
      displayZoom = frame.displayZoom,
      density = frame.density,
      contentOriginY = contentOriginY,
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
    is EditorBringIntoViewTarget.TrackedItem ->
      state.trackedRanges.firstOrNull { it.id == target.id }?.rects?.takeIf { it.isNotEmpty() }
  }

private fun EditorScrollFrame.resolvePolicy(
  target: EditorBringIntoViewTarget,
  requestedPolicy: EditorBringIntoViewPolicy,
): EditorBringIntoViewPolicy =
  when {
    requestedPolicy == EditorBringIntoViewPolicy.Typewriter &&
      target == EditorBringIntoViewTarget.CurrentSelectionHead &&
      autoScrollPolicy.typewriterActive -> EditorBringIntoViewPolicy.Typewriter
    requestedPolicy == EditorBringIntoViewPolicy.Typewriter -> EditorBringIntoViewPolicy.CursorGuard
    else -> requestedPolicy
  }

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
  policy: EditorBringIntoViewPolicy,
  currentScroll: Float,
  rect: VerticalSpan,
  visibleArea: EditorVisibleArea,
  autoScrollPolicy: EditorAutoScrollPolicy,
  maximumScrollY: Float,
  oversizedMinimumVisibleHeight: Float?,
): Float? =
  when (policy) {
    EditorBringIntoViewPolicy.CursorGuard,
    EditorBringIntoViewPolicy.PointerCursorGuard ->
      resolveKeepVisibleScrollOffset(
        currentScroll = currentScroll,
        targetTopInContent = rect.top,
        targetBottomInContent = rect.bottom,
        visibleArea = visibleArea,
        maximumScrollY = maximumScrollY,
        oversizedMinimumVisibleHeight = oversizedMinimumVisibleHeight,
      )

    EditorBringIntoViewPolicy.Typewriter ->
      resolveTypewriterScrollOffset(
        currentScroll = currentScroll,
        targetTopInContent = rect.top,
        targetBottomInContent = rect.bottom,
        visibleArea = visibleArea,
        position = autoScrollPolicy.typewriterPosition,
        maximumScrollY = maximumScrollY,
      )

    EditorBringIntoViewPolicy.Reveal ->
      resolveRevealScrollOffset(
        currentScroll = currentScroll,
        targetTopInContent = rect.top,
        targetBottomInContent = rect.bottom,
        visibleArea = visibleArea,
        maximumScrollY = maximumScrollY,
      )
  }
