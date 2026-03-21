package co.typie.ui.component.popover

import androidx.compose.animation.core.EaseOutExpo
import androidx.compose.animation.core.Easing
import androidx.compose.ui.unit.dp

object PopoverDefaults {
  val ExpandedRadius = 22.dp
  val PanePadding = 6.dp
  val ScreenPadding = 16.dp
  const val ForwardDuration = 320
  const val ReverseDuration = 240
  const val IndicatorDuration = 140
  const val ArmDelayMs = 250L
  val ArmDistance = 9.dp

  val PopoverEasing: Easing = EaseOutExpo
}
