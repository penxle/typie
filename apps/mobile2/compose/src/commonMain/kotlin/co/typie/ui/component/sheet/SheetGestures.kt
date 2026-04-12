package co.typie.ui.component.sheet

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Velocity
import kotlin.math.abs

private const val NESTED_RELEASE_VELOCITY_EPSILON = 0.5f

internal data class SheetDragState(val heightPx: Float, val offsetPx: Float)

internal data class SheetScrollConsumption(val nextState: SheetDragState, val consumedDeltaY: Float)

internal data class SheetGestureSnapshot(
  val enabled: Boolean,
  val isResolving: Boolean,
  val resolvedDetents: List<ResolvedSheetDetent>,
  val currentDetentId: SheetDetentId?,
  val sheetState: SheetDragState,
  val minDetentHeightPx: Float,
  val maxDetentHeightPx: Float,
)

@Composable
internal fun rememberSheetGestureHandler(
  density: Density,
  snapshot: () -> SheetGestureSnapshot,
  onDragStateChange: (SheetDragState) -> Unit,
  onSettleOrDismiss: (Float, SheetGestureSnapshot) -> Unit,
): SheetGestureHandler {
  val currentSnapshot = rememberUpdatedState(snapshot)
  val currentOnDragStateChange = rememberUpdatedState(onDragStateChange)
  val currentOnSettleOrDismiss = rememberUpdatedState(onSettleOrDismiss)

  return remember(density) {
    SheetGestureHandler(
      density = density,
      snapshot = { currentSnapshot.value() },
      onDragStateChange = { currentOnDragStateChange.value(it) },
      onSettleOrDismiss = { velocity, current -> currentOnSettleOrDismiss.value(velocity, current) },
    )
  }
}

internal class SheetGestureHandler(
  private val density: Density,
  private val snapshot: () -> SheetGestureSnapshot,
  private val onDragStateChange: (SheetDragState) -> Unit,
  private val onSettleOrDismiss: (Float, SheetGestureSnapshot) -> Unit,
) {
  private var nestedFlingSettledBySheet = false

  fun onDrag(delta: Float) {
    val current = snapshot()
    if (!canConsumeScroll(current)) return
    consumeDelta(current = current, delta = delta, trackUpperBoundaryOverflow = true)
  }

  fun onDragStopped(velocity: Float) {
    val current = snapshot()
    if (!canConsumeScroll(current) || current.resolvedDetents.isEmpty()) return
    onSettleOrDismiss(velocity, current)
  }

  val boundaryFlingHandoff: (Float) -> Boolean = { velocity ->
    val current = snapshot()
    if (!canHandOffBoundaryFling(current, velocity)) {
      false
    } else {
      nestedFlingSettledBySheet = true
      onSettleOrDismiss(velocity, current)
      true
    }
  }

  val nestedScrollConnection: NestedScrollConnection =
    object : NestedScrollConnection {
      override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
        val current = snapshot()
        if (!canConsumeScroll(current) || available.y >= 0f || current.maxDetentHeightPx <= 0f) {
          return Offset.Zero
        }
        return consumeNestedScrollDelta(
          current = current,
          delta = available.y,
          trackUpperBoundaryOverflow = false,
        )
      }

      override fun onPostScroll(
        consumed: Offset,
        available: Offset,
        source: NestedScrollSource,
      ): Offset {
        val current = snapshot()
        if (
          !canConsumeScroll(current) || available.y == 0f || source != NestedScrollSource.UserInput
        ) {
          return Offset.Zero
        }
        return consumeNestedScrollDelta(
          current = current,
          delta = available.y,
          trackUpperBoundaryOverflow = false,
        )
      }

      override suspend fun onPreFling(available: Velocity): Velocity {
        nestedFlingSettledBySheet = false
        val current = snapshot()
        if (!canHandleNestedRelease(current)) {
          return Velocity.Zero
        }
        val shouldSettle = shouldSettleNestedRelease(current)
        val hasAvailableVelocity = abs(available.y) > NESTED_RELEASE_VELOCITY_EPSILON
        if (!shouldSettle || !hasAvailableVelocity) {
          return Velocity.Zero
        }

        nestedFlingSettledBySheet = true
        val settledVelocity = available.y
        onSettleOrDismiss(settledVelocity, current)
        return Velocity(x = 0f, y = settledVelocity)
      }

      override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
        val current = snapshot()
        if (!canHandleNestedRelease(current)) {
          return Velocity.Zero
        }
        val shouldSettle = shouldSettleNestedRelease(current)
        if (nestedFlingSettledBySheet) {
          nestedFlingSettledBySheet = false
          return Velocity.Zero
        }
        if (shouldSettle) {
          onSettleOrDismiss(
            available.y.takeIf { abs(it) > NESTED_RELEASE_VELOCITY_EPSILON }
              ?: consumed.y.takeIf { abs(it) > NESTED_RELEASE_VELOCITY_EPSILON }
              ?: 0f,
            current,
          )
        }
        return Velocity.Zero
      }
    }

  private fun canConsumeScroll(current: SheetGestureSnapshot): Boolean =
    current.enabled && !current.isResolving

  private fun canHandleNestedRelease(current: SheetGestureSnapshot): Boolean =
    current.enabled && !current.isResolving && current.resolvedDetents.isNotEmpty()

  private fun canHandOffBoundaryFling(current: SheetGestureSnapshot, velocity: Float): Boolean =
    current.enabled &&
      !current.isResolving &&
      current.resolvedDetents.isNotEmpty() &&
      shouldHandOffSheetNestedChildFlingToSheet(velocity)

  private fun consumeNestedScrollDelta(
    current: SheetGestureSnapshot,
    delta: Float,
    trackUpperBoundaryOverflow: Boolean,
  ): Offset =
    consumeDelta(
      current = current,
      delta = delta,
      trackUpperBoundaryOverflow = trackUpperBoundaryOverflow,
    ) ?: Offset.Zero

  private fun consumeDelta(
    current: SheetGestureSnapshot,
    delta: Float,
    trackUpperBoundaryOverflow: Boolean,
  ): Offset? {
    val consumption =
      consumeSheetScrollDelta(
        currentState = current.sheetState,
        delta = delta,
        minHeightPx = current.minDetentHeightPx,
        maxHeightPx = current.maxDetentHeightPx,
        trackUpperBoundaryOverflow = trackUpperBoundaryOverflow,
      ) ?: return null
    onDragStateChange(consumption.nextState)
    return Offset(0f, consumption.consumedDeltaY)
  }

  private fun shouldSettleNestedRelease(current: SheetGestureSnapshot): Boolean =
    shouldHandleSheetNestedPreFling(
      currentDetentHeightPx = currentDetentHeightPx(current),
      sheetHeightPx = current.sheetState.heightPx,
      dragOffsetPx = current.sheetState.offsetPx,
    )

  private fun currentDetentHeightPx(current: SheetGestureSnapshot): Float? =
    current.resolvedDetents
      .firstOrNull { it.id == current.currentDetentId }
      ?.let { with(density) { it.height.toPx() } }
}

internal fun resolveVisualSheetDragOffsetPx(dragOffsetPx: Float): Float =
  dragOffsetPx.coerceAtLeast(0f)

internal fun resolveConsumedSheetScrollDeltaY(
  currentHeightPx: Float,
  currentOffsetPx: Float,
  nextState: SheetDragState,
): Float = (currentHeightPx - nextState.heightPx) + (nextState.offsetPx - currentOffsetPx)

internal fun consumeSheetScrollDelta(
  currentState: SheetDragState,
  delta: Float,
  minHeightPx: Float,
  maxHeightPx: Float,
  trackUpperBoundaryOverflow: Boolean = false,
): SheetScrollConsumption? {
  val nextState =
    consumeSheetDragDelta(
      currentHeightPx = currentState.heightPx,
      currentOffsetPx = currentState.offsetPx,
      delta = delta,
      minHeightPx = minHeightPx,
      maxHeightPx = maxHeightPx,
      trackUpperBoundaryOverflow = trackUpperBoundaryOverflow,
    )
  val consumedDeltaY =
    resolveConsumedSheetScrollDeltaY(
      currentHeightPx = currentState.heightPx,
      currentOffsetPx = currentState.offsetPx,
      nextState = nextState,
    )
  return if (abs(consumedDeltaY) <= 0.5f) {
    null
  } else {
    SheetScrollConsumption(nextState = nextState, consumedDeltaY = consumedDeltaY)
  }
}

internal fun shouldHandleSheetNestedPreFling(
  currentDetentHeightPx: Float?,
  sheetHeightPx: Float,
  dragOffsetPx: Float,
): Boolean {
  if (abs(dragOffsetPx) > 0.5f) {
    return true
  }

  val detentHeightPx = currentDetentHeightPx ?: return false
  return abs(sheetHeightPx - detentHeightPx) > 0.5f
}

internal fun consumeSheetDragDelta(
  currentHeightPx: Float,
  currentOffsetPx: Float,
  delta: Float,
  minHeightPx: Float,
  maxHeightPx: Float,
  trackUpperBoundaryOverflow: Boolean = false,
): SheetDragState {
  if (delta == 0f) {
    return SheetDragState(heightPx = currentHeightPx, offsetPx = currentOffsetPx)
  }

  var nextHeightPx = currentHeightPx
  var nextOffsetPx = currentOffsetPx

  if (delta > 0f) {
    var remainingDelta = delta

    if (nextOffsetPx < 0f) {
      val offsetRecovery = minOf(remainingDelta, -nextOffsetPx)
      nextOffsetPx += offsetRecovery
      remainingDelta -= offsetRecovery
    }

    val collapsibleHeight = (nextHeightPx - minHeightPx).coerceAtLeast(0f)
    val heightDelta = minOf(remainingDelta, collapsibleHeight)
    nextHeightPx -= heightDelta
    remainingDelta -= heightDelta

    if (remainingDelta > 0f) {
      nextOffsetPx += remainingDelta
    }
  } else {
    var remainingDelta = -delta

    if (nextOffsetPx > 0f) {
      val offsetRecovery = minOf(remainingDelta, nextOffsetPx)
      nextOffsetPx -= offsetRecovery
      remainingDelta -= offsetRecovery
    }

    if (remainingDelta > 0f) {
      val expandableHeight = (maxHeightPx - nextHeightPx).coerceAtLeast(0f)
      val heightDelta = minOf(remainingDelta, expandableHeight)
      nextHeightPx += heightDelta
      remainingDelta -= heightDelta
    }

    if (trackUpperBoundaryOverflow && remainingDelta > 0f) {
      nextOffsetPx -= remainingDelta
    }
  }

  return SheetDragState(
    heightPx = nextHeightPx.coerceAtLeast(0f),
    offsetPx =
      if (trackUpperBoundaryOverflow || currentOffsetPx < 0f || nextOffsetPx >= 0f) {
        nextOffsetPx
      } else {
        0f
      },
  )
}
