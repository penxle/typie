package co.typie.ext

import androidx.compose.foundation.gestures.ScrollableState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.isActive

internal data class EdgeAutoScrollPlan(
  val verticalDirection: Float,
  val verticalSpeedPxPerSec: Float,
  val horizontalDirection: Float,
  val horizontalSpeedPxPerSec: Float,
) {
  val isNoOp: Boolean
    get() = verticalDirection == 0f && horizontalDirection == 0f

  companion object {
    val NoOp: EdgeAutoScrollPlan =
      EdgeAutoScrollPlan(
        verticalDirection = 0f,
        verticalSpeedPxPerSec = 0f,
        horizontalDirection = 0f,
        horizontalSpeedPxPerSec = 0f,
      )
  }
}

internal fun insetEdgeAutoScrollViewportRect(
  viewport: Rect,
  topInsetPx: Float = 0f,
  bottomInsetPx: Float = 0f,
): Rect {
  val insetTop = (viewport.top + topInsetPx).coerceAtMost(viewport.bottom)
  val insetBottom = (viewport.bottom - bottomInsetPx).coerceAtLeast(insetTop)
  return Rect(left = viewport.left, top = insetTop, right = viewport.right, bottom = insetBottom)
}

internal fun computeEdgeAutoScrollPlan(
  pointer: Offset,
  insetViewport: Rect,
  edgeThresholdPx: Float,
  minSpeedPxPerSec: Float,
  maxSpeedPxPerSec: Float,
): EdgeAutoScrollPlan {
  if (insetViewport.width <= 0f || insetViewport.height <= 0f) return EdgeAutoScrollPlan.NoOp

  val (verticalDirection, verticalSpeed) =
    axisPlan(
      pointerLocal = pointer.y - insetViewport.top,
      axisLength = insetViewport.height,
      thresholdPx = edgeThresholdPx,
      minSpeedPxPerSec = minSpeedPxPerSec,
      maxSpeedPxPerSec = maxSpeedPxPerSec,
    )
  val (horizontalDirection, horizontalSpeed) =
    axisPlan(
      pointerLocal = pointer.x - insetViewport.left,
      axisLength = insetViewport.width,
      thresholdPx = edgeThresholdPx,
      minSpeedPxPerSec = minSpeedPxPerSec,
      maxSpeedPxPerSec = maxSpeedPxPerSec,
    )
  return EdgeAutoScrollPlan(
    verticalDirection = verticalDirection,
    verticalSpeedPxPerSec = verticalSpeed,
    horizontalDirection = horizontalDirection,
    horizontalSpeedPxPerSec = horizontalSpeed,
  )
}

private fun axisPlan(
  pointerLocal: Float,
  axisLength: Float,
  thresholdPx: Float,
  minSpeedPxPerSec: Float,
  maxSpeedPxPerSec: Float,
): Pair<Float, Float> {
  val nearMinEdge = pointerLocal < thresholdPx
  val nearMaxEdge = pointerLocal > axisLength - thresholdPx

  if (!nearMinEdge && !nearMaxEdge) return 0f to 0f

  val (direction, edgeDistance) =
    if (nearMinEdge) {
      -1f to pointerLocal.coerceAtLeast(0f)
    } else {
      1f to (axisLength - pointerLocal).coerceAtLeast(0f)
    }
  val clampedDistance = edgeDistance.coerceAtMost(thresholdPx)
  val proximity = 1f - (clampedDistance / thresholdPx)
  val speed = minSpeedPxPerSec + proximity * (maxSpeedPxPerSec - minSpeedPxPerSec)
  return direction to speed
}

private val EdgeThreshold = 30.dp
// Speeds below express dp-per-second; multiplied by each frame's delta time to yield pixels per
// frame.
private val MinScrollSpeed = 240.dp
private val MaxScrollSpeed = 960.dp

@Stable
class EdgeAutoScrollController
internal constructor(
  internal val verticalScrollableState: ScrollableState?,
  internal val horizontalScrollableState: ScrollableState?,
) {
  // Write a window-space Offset to drive edge auto-scroll; write null to halt it.
  var pointer: Offset? by mutableStateOf(null)

  var scrollEpoch: Long by mutableLongStateOf(0L)
    private set

  internal var viewport: Rect? by mutableStateOf(null)
  internal var enabled: Boolean by mutableStateOf(true)

  internal fun bumpScrollEpoch() {
    scrollEpoch++
  }
}

@Composable
fun rememberEdgeAutoScrollController(
  verticalScrollableState: ScrollableState? = null,
  horizontalScrollableState: ScrollableState? = null,
): EdgeAutoScrollController =
  remember(verticalScrollableState, horizontalScrollableState) {
    EdgeAutoScrollController(
      verticalScrollableState = verticalScrollableState,
      horizontalScrollableState = horizontalScrollableState,
    )
  }

@Composable
fun Modifier.edgeAutoScroll(
  controller: EdgeAutoScrollController,
  enabled: Boolean = true,
  viewportTopInset: Dp = 0.dp,
  viewportBottomInset: Dp = 0.dp,
): Modifier {
  val density = LocalDensity.current
  val topInsetPx = with(density) { viewportTopInset.toPx() }
  val bottomInsetPx = with(density) { viewportBottomInset.toPx() }
  val edgeThresholdPx = with(density) { EdgeThreshold.toPx() }
  val minSpeedPx = with(density) { MinScrollSpeed.toPx() }
  val maxSpeedPx = with(density) { MaxScrollSpeed.toPx() }

  SideEffect { controller.enabled = enabled }

  LaunchedEffect(controller, topInsetPx, bottomInsetPx, edgeThresholdPx, minSpeedPx, maxSpeedPx) {
    // Gate the loop on a coarse boolean so pointer movement within the edge zone does not
    // cancel and re-seed the coroutine every frame. The loop itself re-samples pointer and
    // viewport each tick to react to fresh positions.
    snapshotFlow {
        val viewport = controller.viewport
        val pointer = controller.pointer
        if (!controller.enabled || viewport == null || pointer == null) return@snapshotFlow false
        !computeEdgeAutoScrollPlan(
            pointer = pointer,
            insetViewport = insetEdgeAutoScrollViewportRect(viewport, topInsetPx, bottomInsetPx),
            edgeThresholdPx = edgeThresholdPx,
            minSpeedPxPerSec = minSpeedPx,
            maxSpeedPxPerSec = maxSpeedPx,
          )
          .isNoOp
      }
      .distinctUntilChanged()
      .collectLatest { shouldScroll ->
        if (!shouldScroll) return@collectLatest
        var lastFrameNanos = withFrameNanos { it }
        while (isActive) {
          val nowNanos = withFrameNanos { it }
          val dtSeconds = (nowNanos - lastFrameNanos) / 1_000_000_000f
          lastFrameNanos = nowNanos

          val viewport = controller.viewport ?: break
          val pointer = controller.pointer ?: break
          val plan =
            computeEdgeAutoScrollPlan(
              pointer = pointer,
              insetViewport = insetEdgeAutoScrollViewportRect(viewport, topInsetPx, bottomInsetPx),
              edgeThresholdPx = edgeThresholdPx,
              minSpeedPxPerSec = minSpeedPx,
              maxSpeedPxPerSec = maxSpeedPx,
            )
          if (plan.isNoOp) break

          val vertical = controller.verticalScrollableState
          val horizontal = controller.horizontalScrollableState
          var consumed = 0f
          if (
            plan.verticalDirection != 0f &&
              vertical != null &&
              vertical.canScroll(plan.verticalDirection)
          ) {
            consumed +=
              vertical.dispatchRawDelta(
                plan.verticalDirection * plan.verticalSpeedPxPerSec * dtSeconds
              )
          }
          if (
            plan.horizontalDirection != 0f &&
              horizontal != null &&
              horizontal.canScroll(plan.horizontalDirection)
          ) {
            consumed +=
              horizontal.dispatchRawDelta(
                plan.horizontalDirection * plan.horizontalSpeedPxPerSec * dtSeconds
              )
          }
          if (consumed != 0f) controller.bumpScrollEpoch()
        }
      }
  }

  return onGloballyPositioned { coordinates ->
    val position = coordinates.positionInWindow()
    val size = coordinates.size
    controller.viewport =
      Rect(
        left = position.x,
        top = position.y,
        right = position.x + size.width,
        bottom = position.y + size.height,
      )
  }
}

private fun ScrollableState.canScroll(direction: Float): Boolean =
  if (direction < 0f) canScrollBackward else canScrollForward
