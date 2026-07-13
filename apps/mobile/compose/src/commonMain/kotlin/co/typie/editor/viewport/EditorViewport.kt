package co.typie.editor.viewport

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.tween
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
import androidx.compose.runtime.snapshots.Snapshot
import androidx.compose.runtime.toString
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.editor.EditorViewportAnchor
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolveMeasuredPageLength
import co.typie.editor.body.resolvePageContentTop
import co.typie.editor.ffi.Size as PageSize
import kotlin.math.abs
import kotlin.math.max

private const val EditorViewportWheelPanScale = 10f
private const val EditorViewportWheelZoomScale = 10f
private const val EditorViewportSmoothScrollDurationMillis = 260
private val EditorViewportSmoothScrollSpec =
  tween<Float>(durationMillis = EditorViewportSmoothScrollDurationMillis, easing = EaseOutCubic)

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
  private var retainedTransformScrollTarget: Offset? = null

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

  val effectiveTransformScrollTarget: Offset
    get() = retainedTransformScrollTarget ?: scrollOffset

  val maxScrollX: Float
    get() = (contentSize.width - viewportSize.width).coerceAtLeast(0f)

  val maxScrollY: Float
    get() = (contentSize.height - viewportSize.height).coerceAtLeast(0f)

  fun updateMeasuredBounds(viewportSize: Size, contentSize: Size): Boolean {
    val resolvedViewportSize = viewportSize.coerceAtLeastZero()
    val resolvedContentSize = contentSize.coerceAtLeastZero()
    var viewportSizeChanged = false
    Snapshot.withoutReadObservation {
      viewportSizeChanged = this.viewportSize != resolvedViewportSize
      if (this.viewportSize == resolvedViewportSize && this.contentSize == resolvedContentSize) {
        if (pendingRestoredScrollOffset == null) {
          applyRetainedTransformScrollTarget()
        }
        return@withoutReadObservation
      }

      this.viewportSize = resolvedViewportSize
      this.contentSize = resolvedContentSize
      if (pendingRestoredScrollOffset != null) {
        if (applyPendingRestoredScrollOffset()) {
          return@withoutReadObservation
        }
        return@withoutReadObservation
      }
      if (applyPendingRestoredScrollOffset()) {
        return@withoutReadObservation
      }
      if (applyRetainedTransformScrollTarget()) {
        return@withoutReadObservation
      }
      clampScrollOffset()
    }
    return viewportSizeChanged
  }

  fun consumePan(delta: Offset, isAutoScroll: Boolean = false): Offset {
    if (isTransforming) {
      return Offset.Zero
    }

    retainedTransformScrollTarget = null
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
    invalidateRetainedTransformScrollTargetAfterTransform()
    pendingRestoredScrollOffset = null
    val resolvedY = resolveScrollY(targetY)
    if (scrollOffset.y == resolvedY) {
      return
    }

    setScrollOffset(
      nextScrollOffset = Offset(scrollOffset.x.coerceIn(0f, maxScrollX), resolvedY),
      isAutoScroll = isAutoScroll,
      emitScrollEvent = true,
    )
  }

  suspend fun animateScrollToY(targetY: Float, isAutoScroll: Boolean = false) {
    invalidateRetainedTransformScrollTargetAfterTransform()
    pendingRestoredScrollOffset = null
    val resolvedY = resolveScrollY(targetY)
    if (scrollOffset.y == resolvedY) {
      return
    }

    val animation = Animatable(scrollOffset.y)
    animation.animateTo(resolvedY, EditorViewportSmoothScrollSpec) {
      scrollToY(targetY = value, isAutoScroll = isAutoScroll)
    }
    scrollToY(targetY = resolvedY, isAutoScroll = isAutoScroll)
  }

  fun scrollTo(offset: Offset, isAutoScroll: Boolean = false) {
    invalidateRetainedTransformScrollTargetAfterTransform()
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

  fun scrollToTransformTarget(offset: Offset, retainUntilMeasuredBounds: Boolean) {
    pendingRestoredScrollOffset = null
    if (retainUntilMeasuredBounds || retainedTransformScrollTarget != null) {
      retainedTransformScrollTarget = offset
    }

    val resolvedScrollOffset = offset.coerceToBounds()
    if (scrollOffset == resolvedScrollOffset) {
      return
    }

    setScrollOffset(
      nextScrollOffset = resolvedScrollOffset,
      isAutoScroll = false,
      emitScrollEvent = true,
    )
  }

  fun beginTransform() {
    retainedTransformScrollTarget = null
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
      isAutoScroll = true,
      emitScrollEvent = true,
    )
  }

  private fun applyPendingRestoredScrollOffset(): Boolean {
    val pendingScrollOffset = pendingRestoredScrollOffset ?: return false
    if (!hasResolvedBounds()) {
      return false
    }

    pendingRestoredScrollOffset = null
    retainedTransformScrollTarget = null
    setScrollOffset(
      nextScrollOffset = pendingScrollOffset.coerceToBounds(),
      isAutoScroll = null,
      emitScrollEvent = false,
    )
    return true
  }

  private fun applyRetainedTransformScrollTarget(): Boolean {
    val retainedScrollTarget = retainedTransformScrollTarget ?: return false
    if (!hasResolvedBounds()) {
      return false
    }

    retainedTransformScrollTarget = null
    setScrollOffset(
      nextScrollOffset = retainedScrollTarget.coerceToBounds(),
      isAutoScroll = false,
      emitScrollEvent = true,
    )
    return true
  }

  private fun invalidateRetainedTransformScrollTargetAfterTransform() {
    if (!isTransforming) {
      retainedTransformScrollTarget = null
    }
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

  private fun resolveScrollY(targetY: Float): Float =
    when {
      !targetY.isFinite() -> scrollOffset.y
      else -> targetY.coerceIn(0f, maxScrollY)
    }

  private fun Offset.coerceToBounds(): Offset =
    Offset(x = x.coerceIn(0f, maxScrollX), y = y.coerceIn(0f, maxScrollY))

  private fun Size.coerceAtLeastZero(): Size =
    Size(width = max(width, 0f), height = max(height, 0f))
}

internal fun consumeEditorViewportTouchPan(
  viewportState: EditorViewportState,
  deltaPx: Offset,
  density: Float,
  canNavigateBack: Boolean = false,
): Offset {
  if (density <= 0f) {
    return Offset.Zero
  }
  if (viewportState.isTransforming) {
    return deltaPx
  }
  if (canNavigateBack && deltaPx.isDominantRightPan() && viewportState.scrollOffset.x <= 0f) {
    return Offset.Zero
  }

  val viewportDelta = Offset(x = -deltaPx.x / density, y = -deltaPx.y / density)
  val consumed = viewportState.consumePan(delta = viewportDelta)
  if (consumed == Offset.Zero) {
    return Offset.Zero
  }
  val consumedPx = Offset(x = -consumed.x * density, y = -consumed.y * density)
  return consumedPx.consumeCrossAxisDelta(deltaPx)
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

private fun Offset.consumeCrossAxisDelta(requested: Offset): Offset =
  if (abs(requested.y) >= abs(requested.x)) {
    if (y != 0f) copy(x = requested.x) else this
  } else {
    if (x != 0f) copy(y = requested.y) else this
  }

private fun Offset.isDominantRightPan(): Boolean = x > 0f && abs(x) > abs(y)

internal fun resolveZoomAnchorDisplayPosition(
  layoutSpec: EditorDocumentLayoutSpec.Paginated,
  anchor: EditorViewportAnchor,
  displayZoom: Float,
  viewportWidth: Float,
  pageSizes: List<PageSize>,
  density: Float = 0f,
): Offset? {
  if (anchor.page !in pageSizes.indices) {
    return null
  }

  val effectiveDisplayZoom =
    if (displayZoom.isFinite() && displayZoom > 0f) {
      displayZoom
    } else {
      1f
    }
  val pageTrackWidth =
    resolveMeasuredPageLength(
      length = layoutSpec.pageWidth,
      displayZoom = effectiveDisplayZoom,
      density = density,
    )
  val pageWidth =
    resolveMeasuredPageLength(
      length = pageSizes[anchor.page].width,
      displayZoom = effectiveDisplayZoom,
      density = density,
    )
  val contentWidth = max(viewportWidth, pageTrackWidth)
  val pageLeft = ((contentWidth - pageWidth) / 2f).coerceAtLeast(0f)
  val pageTop =
    layoutSpec.resolvePageContentTop(
      page = anchor.page,
      pageSizes = pageSizes,
      displayZoom = effectiveDisplayZoom,
      density = density,
    ) ?: return null

  return Offset(
    x = pageLeft + anchor.x * effectiveDisplayZoom,
    y = pageTop + anchor.y * effectiveDisplayZoom,
  )
}
