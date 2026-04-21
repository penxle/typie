package co.typie.ui.component.sheet

import androidx.compose.animation.core.AnimationSpec
import androidx.compose.foundation.gestures.AnchoredDraggableState
import androidx.compose.foundation.gestures.animateTo
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Velocity
import androidx.compose.ui.unit.dp
import kotlin.math.abs

@Composable
internal fun rememberSheetNestedScrollConnection(
  anchoredState: AnchoredDraggableState<Int>,
  visibleAnchors: List<SheetAnchor>,
  containerHeightPx: Float,
  hiddenValue: Int,
  animationSpec: AnimationSpec<Float>,
): NestedScrollConnection {
  val density = LocalDensity.current
  val topVisibleOffset = visibleAnchors.minOfOrNull(SheetAnchor::offset) ?: containerHeightPx
  val velocityThresholdPx = with(density) { 125.dp.toPx() }
  val anchors =
    remember(visibleAnchors, containerHeightPx, hiddenValue) {
      visibleAnchors + SheetAnchor(value = hiddenValue, offset = containerHeightPx)
    }

  return remember(
    anchoredState,
    visibleAnchors,
    containerHeightPx,
    topVisibleOffset,
    velocityThresholdPx,
    anchors,
    animationSpec,
  ) {
    object : NestedScrollConnection {
      override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
        if (source != NestedScrollSource.UserInput || visibleAnchors.isEmpty()) {
          return Offset.Zero
        }

        val currentOffset = anchoredState.offset.takeUnless(Float::isNaN) ?: containerHeightPx
        val shouldConsume =
          shouldSheetConsumeNestedPreScroll(
            currentOffset = currentOffset,
            topStopOffset = topVisibleOffset,
            availableY = available.y,
          )
        if (!shouldConsume) {
          return Offset.Zero
        }

        return Offset(0f, anchoredState.dispatchRawDelta(available.y))
      }

      override fun onPostScroll(
        consumed: Offset,
        available: Offset,
        source: NestedScrollSource,
      ): Offset {
        if (source != NestedScrollSource.UserInput || visibleAnchors.isEmpty()) {
          return Offset.Zero
        }

        val currentOffset = anchoredState.offset.takeUnless(Float::isNaN) ?: containerHeightPx
        val shouldConsume =
          shouldSheetConsumeNestedPostScroll(
            currentOffset = currentOffset,
            topStopOffset = topVisibleOffset,
            availableY = available.y,
          )
        if (!shouldConsume) {
          return Offset.Zero
        }

        return Offset(0f, anchoredState.dispatchRawDelta(available.y))
      }

      override suspend fun onPreFling(available: Velocity): Velocity {
        if (visibleAnchors.isEmpty()) {
          return Velocity.Zero
        }

        val currentOffset = anchoredState.offset.takeUnless(Float::isNaN) ?: containerHeightPx
        val shouldSettle =
          shouldSheetConsumeNestedPreScroll(
            currentOffset = currentOffset,
            topStopOffset = topVisibleOffset,
            availableY = available.y,
          )
        if (!shouldSettle) {
          return Velocity.Zero
        }

        val targetValue =
          resolveSheetFlingTargetValue(
            anchors = anchors,
            currentOffset = currentOffset,
            velocity = available.y,
            velocityThreshold = velocityThresholdPx,
          )
        anchoredState.animateTo(targetValue, animationSpec)
        return available
      }

      override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
        if (visibleAnchors.isEmpty()) {
          return Velocity.Zero
        }

        val currentOffset = anchoredState.offset.takeUnless(Float::isNaN) ?: containerHeightPx
        val shouldSettle =
          shouldSheetSettleAfterNestedGesture(
            currentOffset = currentOffset,
            topStopOffset = topVisibleOffset,
          )
        if (!shouldSettle) {
          return Velocity.Zero
        }

        anchoredState.settle(animationSpec)
        return available
      }
    }
  }
}

internal fun shouldSheetConsumeNestedPreScroll(
  currentOffset: Float,
  topStopOffset: Float,
  availableY: Float,
  tolerancePx: Float = SheetAnchorTolerancePx,
): Boolean {
  if (currentOffset.isNaN() || availableY == 0f) {
    return false
  }

  return availableY < 0f && currentOffset > topStopOffset + tolerancePx
}

internal fun shouldSheetConsumeNestedPostScroll(
  currentOffset: Float,
  topStopOffset: Float,
  availableY: Float,
  tolerancePx: Float = SheetAnchorTolerancePx,
): Boolean {
  if (currentOffset.isNaN() || availableY <= 0f) {
    return false
  }

  return currentOffset >= topStopOffset - tolerancePx
}

internal fun shouldSheetSettleAfterNestedGesture(
  currentOffset: Float,
  topStopOffset: Float,
  tolerancePx: Float = SheetAnchorTolerancePx,
): Boolean {
  if (currentOffset.isNaN()) {
    return false
  }

  return currentOffset > topStopOffset + tolerancePx
}

internal fun resolveSheetFlingTargetValue(
  anchors: List<SheetAnchor>,
  currentOffset: Float,
  velocity: Float,
  velocityThreshold: Float,
  positionalThreshold: (Float) -> Float = { distance -> distance / 2f },
): Int {
  require(anchors.isNotEmpty()) { "anchors must not be empty" }
  require(!currentOffset.isNaN()) { "currentOffset must not be NaN" }

  val orderedAnchors = anchors.sortedBy(SheetAnchor::offset)
  if (abs(velocity) == 0f) {
    return closestSheetAnchor(orderedAnchors, currentOffset).value
  }

  if (abs(velocity) >= abs(velocityThreshold)) {
    return nextDirectionalSheetAnchor(orderedAnchors, currentOffset, searchUpwards = velocity > 0f)
      .value
  }

  val left = closestDirectionalSheetAnchor(orderedAnchors, currentOffset, searchUpwards = false)
  val right = closestDirectionalSheetAnchor(orderedAnchors, currentOffset, searchUpwards = true)
  val distance = abs(left.offset - right.offset)
  val relativeThreshold = abs(positionalThreshold(distance))
  val closestAnchorFromStart = if (velocity > 0f) left.offset else right.offset
  val relativePosition = abs(closestAnchorFromStart - currentOffset)

  return if (relativePosition >= relativeThreshold) {
    if (velocity > 0f) right.value else left.value
  } else {
    if (velocity > 0f) left.value else right.value
  }
}

private fun closestSheetAnchor(anchors: List<SheetAnchor>, position: Float): SheetAnchor {
  return anchors.minBy { anchor -> abs(position - anchor.offset) }
}

private fun closestDirectionalSheetAnchor(
  anchors: List<SheetAnchor>,
  position: Float,
  searchUpwards: Boolean,
): SheetAnchor {
  return anchors
    .filter { anchor ->
      if (searchUpwards) {
        anchor.offset >= position
      } else {
        anchor.offset <= position
      }
    }
    .minByOrNull { anchor -> abs(anchor.offset - position) }
    ?: closestSheetAnchor(anchors, position)
}

private fun nextDirectionalSheetAnchor(
  anchors: List<SheetAnchor>,
  position: Float,
  searchUpwards: Boolean,
  tolerancePx: Float = SheetAnchorTolerancePx,
): SheetAnchor {
  return anchors
    .filter { anchor ->
      if (searchUpwards) {
        anchor.offset > position + tolerancePx
      } else {
        anchor.offset < position - tolerancePx
      }
    }
    .minByOrNull { anchor -> abs(anchor.offset - position) }
    ?: closestDirectionalSheetAnchor(anchors, position, searchUpwards)
}
