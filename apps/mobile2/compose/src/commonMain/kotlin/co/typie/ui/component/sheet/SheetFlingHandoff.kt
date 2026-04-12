package co.typie.ui.component.sheet

import androidx.compose.foundation.gestures.FlingBehavior
import androidx.compose.foundation.gestures.ScrollScope
import androidx.compose.foundation.gestures.ScrollableDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.staticCompositionLocalOf
import kotlin.math.abs

internal const val SHEET_BOUNDARY_FLING_HANDOFF_VELOCITY_THRESHOLD = 2400f

internal val LocalSheetBoundaryFlingHandoff =
  staticCompositionLocalOf<(Float) -> Boolean> { { false } }

@Composable
internal fun rememberSheetBoundaryHandoffFlingBehavior(
  isAtSheetDismissBoundary: () -> Boolean = { true }
): FlingBehavior {
  val defaultFlingBehavior = ScrollableDefaults.flingBehavior()
  val handoff = rememberUpdatedState(LocalSheetBoundaryFlingHandoff.current)
  val isAtDismissBoundary = rememberUpdatedState(isAtSheetDismissBoundary)

  return remember(defaultFlingBehavior) {
    object : FlingBehavior {
      override suspend fun ScrollScope.performFling(initialVelocity: Float): Float {
        val remainingVelocity = with(defaultFlingBehavior) { performFling(initialVelocity) }
        if (!shouldAttemptSheetBoundaryHandoff(remainingVelocity, isAtDismissBoundary.value())) {
          return remainingVelocity
        }
        if (handoff.value(remainingVelocity)) {
          return 0f
        }
        return remainingVelocity
      }
    }
  }
}

internal fun shouldAttemptSheetBoundaryHandoff(
  remainingVelocity: Float,
  isAtDismissBoundary: Boolean,
): Boolean = isAtDismissBoundary && abs(remainingVelocity) > 0.5f

internal fun shouldHandOffSheetNestedChildFlingToSheet(velocity: Float): Boolean =
  velocity >= SHEET_BOUNDARY_FLING_HANDOFF_VELOCITY_THRESHOLD
