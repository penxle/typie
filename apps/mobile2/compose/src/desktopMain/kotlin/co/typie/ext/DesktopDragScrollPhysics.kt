package co.typie.ext

import co.typie.ui.component.sheet.SHEET_BOUNDARY_FLING_HANDOFF_VELOCITY_THRESHOLD
import kotlin.math.abs
import kotlin.math.sign

internal const val FLING_VELOCITY_MULTIPLIER = 0.72f
internal const val CONSUMPTION_EPSILON = 0.5f

private const val BOUNDARY_FLING_ELASTIC_OVERSCROLL_IMPULSE_MULTIPLIER = 0.02f
private const val BOUNDARY_FLING_ELASTIC_OVERSCROLL_MAX_DELTA = 72f

internal enum class DragScrollFlingMode {
  ScrollableContent,
  DirectBridge,
}

internal enum class DragScrollBoundaryFlingOutcome {
  ContinueElasticOverscroll,
  HandOffToAncestor,
  Stop,
}

internal fun resolveDragScrollAncestorHandoffVelocity(
  pointerVelocity: Float,
  mode: DragScrollFlingMode,
): Float =
  when (mode) {
    DragScrollFlingMode.ScrollableContent -> -pointerVelocity
    DragScrollFlingMode.DirectBridge -> pointerVelocity
  }

internal fun resolveDragScrollBoundaryFlingOutcome(
  availableVelocity: Float,
  boundaryUnconsumedScrollDelta: Float,
  overscrollEnabled: Boolean,
): DragScrollBoundaryFlingOutcome =
  when {
    availableVelocity >= SHEET_BOUNDARY_FLING_HANDOFF_VELOCITY_THRESHOLD &&
      boundaryUnconsumedScrollDelta < -CONSUMPTION_EPSILON ->
      DragScrollBoundaryFlingOutcome.HandOffToAncestor
    overscrollEnabled -> DragScrollBoundaryFlingOutcome.ContinueElasticOverscroll
    else -> DragScrollBoundaryFlingOutcome.Stop
  }

internal fun resolveDragScrollBoundaryElasticOverscrollDelta(
  availableVelocity: Float,
  boundaryUnconsumedScrollDelta: Float,
): Float {
  if (
    abs(availableVelocity) <= CONSUMPTION_EPSILON ||
      abs(boundaryUnconsumedScrollDelta) <= CONSUMPTION_EPSILON
  ) {
    return 0f
  }

  val magnitude =
    (abs(availableVelocity) * BOUNDARY_FLING_ELASTIC_OVERSCROLL_IMPULSE_MULTIPLIER).coerceAtMost(
      BOUNDARY_FLING_ELASTIC_OVERSCROLL_MAX_DELTA
    )
  return magnitude * sign(boundaryUnconsumedScrollDelta)
}

internal fun shouldHandOffDragScrollFlingToAncestorImmediately(
  ancestorParticipated: Boolean,
  localParticipated: Boolean,
  ancestorConsumedLastSample: Boolean,
  localConsumedLastSample: Boolean,
): Boolean =
  when {
    ancestorConsumedLastSample && !localConsumedLastSample -> true
    ancestorParticipated && !localParticipated -> true
    else -> false
  }

internal fun shouldAllowDragScrollFlingAncestorHandoff(
  ancestorParticipated: Boolean,
  ancestorConsumedLastSample: Boolean,
  localConsumedLastSample: Boolean,
): Boolean = ancestorParticipated && ancestorConsumedLastSample && !localConsumedLastSample

internal fun shouldCancelDragScrollDecayForAncestorHandoff(
  ancestorConsumedPointerDelta: Float,
  localConsumedPointerDelta: Float,
  unconsumedScrollDelta: Float,
): Boolean =
  abs(ancestorConsumedPointerDelta) > CONSUMPTION_EPSILON &&
    abs(localConsumedPointerDelta) <= CONSUMPTION_EPSILON &&
    abs(unconsumedScrollDelta) <= CONSUMPTION_EPSILON

internal fun shouldDispatchDragScrollPostFlingToAncestor(
  ancestorParticipated: Boolean,
  ancestorConsumedDuringFling: Boolean,
  availableVelocity: Float,
): Boolean =
  ancestorParticipated ||
    ancestorConsumedDuringFling ||
    abs(availableVelocity) > CONSUMPTION_EPSILON
