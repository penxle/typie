package co.typie.ext

import androidx.compose.foundation.gestures.ScrollableState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

object EdgeAutoScrollDefaults {
  const val FrameDurationMs = 16L
  val EdgeThreshold = 30.dp
  val MinScrollSpeed = 4.dp
  val MaxScrollSpeed = 16.dp
}

@Composable
fun rememberEdgeAutoScrollState(
  verticalScrollableState: ScrollableState? = null,
  horizontalScrollableState: ScrollableState? = null,
  edgeThreshold: Dp = EdgeAutoScrollDefaults.EdgeThreshold,
  minScrollSpeed: Dp = EdgeAutoScrollDefaults.MinScrollSpeed,
  maxScrollSpeed: Dp = EdgeAutoScrollDefaults.MaxScrollSpeed,
  frameDurationMs: Long = EdgeAutoScrollDefaults.FrameDurationMs,
): EdgeAutoScrollState {
  val scope = rememberCoroutineScope()
  val density = LocalDensity.current
  val edgeThresholdPx = with(density) { edgeThreshold.toPx() }
  val minScrollSpeedPx = with(density) { minScrollSpeed.toPx() }
  val maxScrollSpeedPx = with(density) { maxScrollSpeed.toPx() }

  return remember(scope, edgeThresholdPx, minScrollSpeedPx, maxScrollSpeedPx, frameDurationMs) {
      EdgeAutoScrollState(
        scope = scope,
        edgeThresholdPx = edgeThresholdPx,
        minScrollSpeedPx = minScrollSpeedPx,
        maxScrollSpeedPx = maxScrollSpeedPx,
        frameDurationMs = frameDurationMs,
      )
    }
    .also { state ->
      state.verticalScrollableState = verticalScrollableState
      state.horizontalScrollableState = horizontalScrollableState
    }
}

@Composable
fun Modifier.edgeAutoScroll(
  state: EdgeAutoScrollState,
  enabled: Boolean = true,
  viewportTopInset: Dp = 0.dp,
  viewportBottomInset: Dp = 0.dp,
): Modifier {
  val density = LocalDensity.current
  val viewportTopInsetPx = with(density) { viewportTopInset.toPx() }
  val viewportBottomInsetPx = with(density) { viewportBottomInset.toPx() }

  SideEffect { state.setEnabled(enabled) }

  DisposableEffect(state) { onDispose { state.detach() } }

  return this.onGloballyPositioned { coordinates ->
    val position = coordinates.positionInWindow()
    val size = coordinates.size
    state.updateViewportRect(
      insetViewportRect(
        viewportRect =
          Rect(
            left = position.x,
            top = position.y,
            right = position.x + size.width,
            bottom = position.y + size.height,
          ),
        topInsetPx = viewportTopInsetPx,
        bottomInsetPx = viewportBottomInsetPx,
      )
    )
  }
}

internal fun insetViewportRect(
  viewportRect: Rect,
  topInsetPx: Float = 0f,
  bottomInsetPx: Float = 0f,
): Rect {
  val insetTop = (viewportRect.top + topInsetPx).coerceAtMost(viewportRect.bottom)
  val insetBottom = (viewportRect.bottom - bottomInsetPx).coerceAtLeast(insetTop)
  return Rect(
    left = viewportRect.left,
    top = insetTop,
    right = viewportRect.right,
    bottom = insetBottom,
  )
}

@Stable
class EdgeAutoScrollState
internal constructor(
  private val scope: CoroutineScope,
  private val edgeThresholdPx: Float,
  private val minScrollSpeedPx: Float,
  private val maxScrollSpeedPx: Float,
  private val frameDurationMs: Long,
) {
  internal var verticalScrollableState: ScrollableState? = null
  internal var horizontalScrollableState: ScrollableState? = null

  private var autoScrollJob: Job? = null
  private var viewportRect: Rect? = null
  private var pointerPosition: Offset? = null
  private var enabled = true
  private var verticalDirection = 0f
  private var horizontalDirection = 0f
  private var verticalEdgeDistance = 0f
  private var horizontalEdgeDistance = 0f
  private var onAutoScroll: (() -> Unit)? = null

  fun update(pointerPosition: Offset, onAutoScroll: (() -> Unit)? = null) {
    this.pointerPosition = pointerPosition
    this.onAutoScroll = onAutoScroll
    reevaluate()
  }

  fun stop() {
    pointerPosition = null
    onAutoScroll = null
    stopAutoScroll()
  }

  internal fun setEnabled(enabled: Boolean) {
    this.enabled = enabled
    if (!enabled) {
      stop()
      return
    }

    reevaluate()
  }

  internal fun updateViewportRect(viewportRect: Rect?) {
    this.viewportRect = viewportRect
    reevaluate()
  }

  internal fun detach() {
    viewportRect = null
    stop()
  }

  private fun reevaluate() {
    val viewportRect = viewportRect
    val pointerPosition = pointerPosition
    if (!enabled || viewportRect == null || pointerPosition == null) {
      stopAutoScroll()
      return
    }

    val localX = pointerPosition.x - viewportRect.left
    val localY = pointerPosition.y - viewportRect.top
    val bottomThreshold = viewportRect.height - edgeThresholdPx
    val rightThreshold = viewportRect.width - edgeThresholdPx

    if (localY < edgeThresholdPx) {
      verticalEdgeDistance = localY.coerceIn(0f, edgeThresholdPx)
      verticalDirection = -1f
    } else if (localY > bottomThreshold) {
      verticalEdgeDistance = (viewportRect.height - localY).coerceIn(0f, edgeThresholdPx)
      verticalDirection = 1f
    } else {
      verticalDirection = 0f
    }

    if (localX < edgeThresholdPx) {
      horizontalEdgeDistance = localX.coerceIn(0f, edgeThresholdPx)
      horizontalDirection = -1f
    } else if (localX > rightThreshold) {
      horizontalEdgeDistance = (viewportRect.width - localX).coerceIn(0f, edgeThresholdPx)
      horizontalDirection = 1f
    } else {
      horizontalDirection = 0f
    }

    val shouldAutoScroll =
      (verticalDirection != 0f && verticalScrollableState != null) ||
        (horizontalDirection != 0f && horizontalScrollableState != null)

    if (shouldAutoScroll) {
      startAutoScroll()
    } else {
      stopAutoScroll()
    }
  }

  private fun startAutoScroll() {
    if (autoScrollJob != null) {
      return
    }

    autoScrollJob = scope.launch {
      while (isActive) {
        var didScroll = false

        val verticalScrollableState = verticalScrollableState
        if (
          verticalDirection != 0f &&
            verticalScrollableState != null &&
            canScroll(verticalScrollableState, verticalDirection)
        ) {
          didScroll =
            scrollAxis(
              scrollableState = verticalScrollableState,
              direction = verticalDirection,
              edgeDistance = verticalEdgeDistance,
            ) || didScroll
        }

        val horizontalScrollableState = horizontalScrollableState
        if (
          horizontalDirection != 0f &&
            horizontalScrollableState != null &&
            canScroll(horizontalScrollableState, horizontalDirection)
        ) {
          didScroll =
            scrollAxis(
              scrollableState = horizontalScrollableState,
              direction = horizontalDirection,
              edgeDistance = horizontalEdgeDistance,
            ) || didScroll
        }

        if (didScroll) {
          onAutoScroll?.invoke()
        }

        delay(frameDurationMs)
      }
    }
  }

  private fun stopAutoScroll() {
    autoScrollJob?.cancel()
    autoScrollJob = null
    verticalDirection = 0f
    horizontalDirection = 0f
    verticalEdgeDistance = 0f
    horizontalEdgeDistance = 0f
  }

  private fun scrollAxis(
    scrollableState: ScrollableState,
    direction: Float,
    edgeDistance: Float,
  ): Boolean {
    val proximity = 1f - (edgeDistance / edgeThresholdPx).coerceIn(0f, 1f)
    val scrollSpeed = minScrollSpeedPx + proximity * (maxScrollSpeedPx - minScrollSpeedPx)
    val consumed = scrollableState.dispatchRawDelta(direction * scrollSpeed)
    return consumed != 0f
  }

  private fun canScroll(scrollableState: ScrollableState, direction: Float): Boolean {
    return if (direction < 0f) {
      scrollableState.canScrollBackward
    } else {
      scrollableState.canScrollForward
    }
  }
}
