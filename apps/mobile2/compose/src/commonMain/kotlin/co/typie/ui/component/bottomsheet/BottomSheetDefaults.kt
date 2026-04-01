package co.typie.ui.component.bottomsheet

import androidx.compose.animation.core.EaseInCubic
import androidx.compose.animation.core.EaseOutExpo
import androidx.compose.animation.core.Easing
import androidx.compose.ui.unit.dp

object BottomSheetDefaults {
  val TopCornerRadius = 22.dp
  val HandleWidth = 36.dp
  val HandleHeight = 4.dp
  val HandleTopPadding = 8.dp
  const val MaxHeightFraction = 0.9f
  const val ScrimAlpha = 0.4f
  const val EnterDuration = 320
  const val ExitDuration = 240
  val EnterEasing: Easing = EaseOutExpo
  val ExitEasing: Easing = EaseInCubic
}
