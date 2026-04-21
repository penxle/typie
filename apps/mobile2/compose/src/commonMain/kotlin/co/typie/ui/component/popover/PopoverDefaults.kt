package co.typie.ui.component.popover

import androidx.compose.animation.core.EaseOutExpo
import androidx.compose.animation.core.Easing
import androidx.compose.ui.unit.dp
import co.typie.ui.theme.AppShapes

object PopoverDefaults {
  val ExpandedRadius = AppShapes.xl
  val PanePadding = 6.dp
  val ScreenPadding = 16.dp
  const val ForwardDuration = 320
  const val ReverseDuration = 240
  const val IndicatorDuration = 140
  const val ArmDelayMs = 180L

  val PopoverEasing: Easing = EaseOutExpo
}
