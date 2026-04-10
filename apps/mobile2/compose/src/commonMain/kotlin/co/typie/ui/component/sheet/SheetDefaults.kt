package co.typie.ui.component.sheet

import androidx.compose.animation.core.EaseInCubic
import androidx.compose.animation.core.EaseOutExpo
import androidx.compose.animation.core.Easing
import androidx.compose.ui.unit.dp

object SheetDefaults {
  const val EnterDuration = 320
  const val ExitDuration = 240
  const val HeightAnimationDuration = 220
  const val DismissVelocityThreshold = 1000f
  const val DetentSnapVelocityThreshold = 240f
  const val DismissThresholdFraction = 0.32f
  val DetentSnapThreshold = 32.dp
  val EnterEasing: Easing = EaseOutExpo
  val ExitEasing: Easing = EaseInCubic
}
