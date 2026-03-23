package co.typie.ui.component

import kotlin.math.abs

internal const val TooltipScrollVelocityThresholdPxPerSecond = 700f
internal const val TapGestureMovementTolerancePx = 6f

internal enum class TooltipGesturePhase {
  Tooltip,
  Scrub,
}

internal enum class TooltipGestureAction {
  BeginHorizontalScroll,
  BeginVerticalScroll,
  BeginScrub,
  ContinueScrub,
}

internal fun resolveTooltipGestureAction(
  phase: TooltipGesturePhase,
  velocityX: Float,
  velocityY: Float,
  velocityThresholdPxPerSecond: Float = TooltipScrollVelocityThresholdPxPerSecond,
): TooltipGestureAction {
  val absVelocityX = abs(velocityX)
  val absVelocityY = abs(velocityY)
  val isHorizontalVelocity = absVelocityX >= absVelocityY

  if (isHorizontalVelocity && absVelocityX >= velocityThresholdPxPerSecond) {
    return TooltipGestureAction.BeginHorizontalScroll
  }

  if (!isHorizontalVelocity && absVelocityY >= velocityThresholdPxPerSecond) {
    return TooltipGestureAction.BeginVerticalScroll
  }

  return if (phase == TooltipGesturePhase.Scrub) {
    TooltipGestureAction.ContinueScrub
  } else {
    TooltipGestureAction.BeginScrub
  }
}
