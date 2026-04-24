package co.typie.editor.viewport

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.currentCompositeKeyHashCode
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.toString
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.editor.EditorViewportAnchor
import co.typie.editor.body.resolvePaginatedPageGap
import co.typie.editor.ffi.Size as PageSize
import kotlin.math.max

private const val EditorViewportWheelPanScale = 10f
private const val EditorViewportWheelZoomScale = 10f

private class ViewportPositionHolder(initial: Offset) : ViewModel() {
  var position by mutableStateOf(initial)
}

@Composable
internal fun rememberEditorViewportState(
  key: String? = null,
  initialScrollOffset: Offset = Offset.Zero,
): EditorViewportState {
  val resolvedKey = key ?: currentCompositeKeyHashCode.toString(36)
  val holder =
    viewModel<ViewportPositionHolder>(key = resolvedKey) {
      ViewportPositionHolder(initialScrollOffset)
    }
  val state = remember(resolvedKey) { EditorViewportState(initialScrollOffset = holder.position) }

  LaunchedEffect(state, holder) {
    snapshotFlow { state.persistedScrollOffset }.collect { holder.position = it }
  }

  return state
}

@Stable
internal class EditorViewportState(initialScrollOffset: Offset = Offset.Zero) {
  private var pendingRestoredScrollOffset: Offset? = initialScrollOffset.takeIf {
    it != Offset.Zero
  }

  var scrollOffset by mutableStateOf(Offset.Zero)
    private set

  var viewportSize by mutableStateOf(Size.Zero)
    private set

  var contentSize by mutableStateOf(Size.Zero)
    private set

  var isTransforming by mutableStateOf(false)
    private set

  var lastScrollRevision by mutableIntStateOf(0)
    private set

  var lastScrollWasAuto by mutableStateOf(false)
    private set

  private var isScrollableInteractionInProgress by mutableStateOf(false)
  private var isScrollbarDragInProgress by mutableStateOf(false)

  val isDirectManipulationInProgress: Boolean
    get() = isScrollableInteractionInProgress || isScrollbarDragInProgress

  val persistedScrollOffset: Offset
    get() = pendingRestoredScrollOffset ?: scrollOffset

  val maxScrollX: Float
    get() = (contentSize.width - viewportSize.width).coerceAtLeast(0f)

  val maxScrollY: Float
    get() = (contentSize.height - viewportSize.height).coerceAtLeast(0f)

  fun updateViewportSize(size: Size) {
    val resolvedSize = size.coerceAtLeastZero()
    if (viewportSize == resolvedSize) {
      return
    }

    viewportSize = resolvedSize
    if (pendingRestoredScrollOffset != null) {
      if (applyPendingRestoredScrollOffset()) {
        return
      }
      return
    }
    if (applyPendingRestoredScrollOffset()) {
      return
    }
    clampScrollOffset()
  }

  fun updateContentSize(size: Size) {
    val resolvedSize = size.coerceAtLeastZero()
    if (contentSize == resolvedSize) {
      return
    }

    contentSize = resolvedSize
    if (pendingRestoredScrollOffset != null) {
      if (applyPendingRestoredScrollOffset()) {
        return
      }
      return
    }
    if (applyPendingRestoredScrollOffset()) {
      return
    }
    clampScrollOffset()
  }

  fun consumePan(delta: Offset, isAutoScroll: Boolean = false): Offset {
    if (isTransforming) {
      return Offset.Zero
    }

    pendingRestoredScrollOffset = null
    val nextScrollOffset = scrollOffset + delta
    val resolvedScrollOffset = nextScrollOffset.coerceToBounds()
    val consumedDelta = resolvedScrollOffset - scrollOffset
    if (consumedDelta == Offset.Zero) {
      return Offset.Zero
    }

    setScrollOffset(
      nextScrollOffset = resolvedScrollOffset,
      isAutoScroll = isAutoScroll,
      emitScrollEvent = true,
    )
    return consumedDelta
  }

  fun scrollToY(targetY: Float, isAutoScroll: Boolean = false) {
    pendingRestoredScrollOffset = null
    val resolvedY = targetY.coerceIn(0f, maxScrollY)
    if (scrollOffset.y == resolvedY) {
      return
    }

    setScrollOffset(
      nextScrollOffset = Offset(scrollOffset.x, resolvedY),
      isAutoScroll = isAutoScroll,
      emitScrollEvent = true,
    )
  }

  fun scrollTo(offset: Offset, isAutoScroll: Boolean = false) {
    pendingRestoredScrollOffset = null
    val resolvedScrollOffset = offset.coerceToBounds()
    if (scrollOffset == resolvedScrollOffset) {
      return
    }

    setScrollOffset(
      nextScrollOffset = resolvedScrollOffset,
      isAutoScroll = isAutoScroll,
      emitScrollEvent = true,
    )
  }

  fun dispatchDeltaY(deltaY: Float, isAutoScroll: Boolean = false): Float {
    val beforeY = scrollOffset.y
    scrollToY(targetY = beforeY + deltaY, isAutoScroll = isAutoScroll)
    return scrollOffset.y - beforeY
  }

  fun beginTransform() {
    isTransforming = true
  }

  fun endTransform() {
    isTransforming = false
  }

  fun updateScrollableInteractionInProgress(inProgress: Boolean) {
    if (isScrollableInteractionInProgress == inProgress) {
      return
    }

    isScrollableInteractionInProgress = inProgress
  }

  fun updateScrollbarDragInProgress(inProgress: Boolean) {
    if (isScrollbarDragInProgress == inProgress) {
      return
    }

    isScrollbarDragInProgress = inProgress
  }

  private fun clampScrollOffset() {
    setScrollOffset(
      nextScrollOffset = scrollOffset.coerceToBounds(),
      isAutoScroll = null,
      emitScrollEvent = false,
    )
  }

  private fun applyPendingRestoredScrollOffset(): Boolean {
    val pendingScrollOffset = pendingRestoredScrollOffset ?: return false
    if (!hasResolvedBounds()) {
      return false
    }

    pendingRestoredScrollOffset = null
    setScrollOffset(
      nextScrollOffset = pendingScrollOffset.coerceToBounds(),
      isAutoScroll = null,
      emitScrollEvent = false,
    )
    return true
  }

  private fun setScrollOffset(
    nextScrollOffset: Offset,
    isAutoScroll: Boolean?,
    emitScrollEvent: Boolean,
  ) {
    if (scrollOffset == nextScrollOffset) {
      return
    }

    scrollOffset = nextScrollOffset
    if (!emitScrollEvent || isAutoScroll == null) {
      return
    }

    lastScrollWasAuto = isAutoScroll
    lastScrollRevision += 1
  }

  private fun hasResolvedBounds(): Boolean =
    viewportSize.width > 0f &&
      viewportSize.height > 0f &&
      contentSize.width > 0f &&
      contentSize.height > 0f

  private fun Offset.coerceToBounds(): Offset =
    Offset(x = x.coerceIn(0f, maxScrollX), y = y.coerceIn(0f, maxScrollY))

  private fun Size.coerceAtLeastZero(): Size =
    Size(width = max(width, 0f), height = max(height, 0f))
}

internal fun consumeEditorViewportTouchPan(
  viewportState: EditorViewportState,
  deltaPx: Offset,
  density: Float,
): Offset {
  if (density <= 0f) {
    return Offset.Zero
  }

  val viewportDelta = Offset(x = -deltaPx.x / density, y = -deltaPx.y / density)
  val consumed = viewportState.consumePan(delta = viewportDelta)
  if (consumed == Offset.Zero) {
    return Offset.Zero
  }
  return Offset(x = -consumed.x * density, y = -consumed.y * density)
}

internal fun consumeEditorViewportWheelPan(
  viewportState: EditorViewportState,
  scrollDelta: Offset,
): Offset =
  viewportState.consumePan(
    delta =
      Offset(
        x = scrollDelta.x * EditorViewportWheelPanScale,
        y = scrollDelta.y * EditorViewportWheelPanScale,
      )
  )

internal fun normalizeEditorViewportWheelZoomDelta(delta: Float): Float =
  if (delta.isFinite()) {
    delta * EditorViewportWheelZoomScale
  } else {
    0f
  }

internal data class EditorZoomViewportScrollTarget(
  val horizontalScroll: Float,
  val verticalScroll: Float,
)

internal fun resolveZoomViewportScrollTarget(
  anchor: EditorViewportAnchor,
  focalX: Float,
  focalY: Float,
  displayZoom: Float,
  currentHorizontalScroll: Float,
  currentVerticalScroll: Float,
  pageSizes: List<PageSize>,
): EditorZoomViewportScrollTarget? {
  if (anchor.page !in pageSizes.indices) {
    return null
  }

  val effectiveDisplayZoom =
    if (displayZoom.isFinite() && displayZoom > 0f) {
      displayZoom
    } else {
      1f
    }
  val anchorX = anchor.x * effectiveDisplayZoom
  val anchorY =
    resolveZoomedPageTop(
      page = anchor.page,
      pageSizes = pageSizes,
      displayZoom = effectiveDisplayZoom,
    ) + anchor.y * effectiveDisplayZoom

  return EditorZoomViewportScrollTarget(
    horizontalScroll = currentHorizontalScroll + anchorX - focalX,
    verticalScroll = currentVerticalScroll + anchorY - focalY,
  )
}

internal fun syncViewportToZoomAnchor(
  viewportState: EditorViewportState,
  pageSizes: List<PageSize>,
  anchor: EditorViewportAnchor,
  focalX: Float,
  focalY: Float,
  displayZoom: Float,
  isAutoScroll: Boolean = false,
) {
  val currentHorizontalScroll = viewportState.scrollOffset.x
  val currentVerticalScroll = viewportState.scrollOffset.y
  val target =
    resolveZoomViewportScrollTarget(
      anchor = anchor,
      focalX = focalX,
      focalY = focalY,
      displayZoom = displayZoom,
      currentHorizontalScroll = currentHorizontalScroll,
      currentVerticalScroll = currentVerticalScroll,
      pageSizes = pageSizes,
    ) ?: return

  viewportState.scrollTo(
    offset = Offset(x = target.horizontalScroll, y = target.verticalScroll),
    isAutoScroll = isAutoScroll,
  )
}

private fun resolveZoomedPageTop(page: Int, pageSizes: List<PageSize>, displayZoom: Float): Float {
  var top = 0f
  val pageGap = resolvePaginatedPageGap(displayZoom)
  repeat(page) { index ->
    top += pageSizes[index].height * displayZoom
    if (index < pageSizes.lastIndex) {
      top += pageGap
    }
  }
  return top
}
