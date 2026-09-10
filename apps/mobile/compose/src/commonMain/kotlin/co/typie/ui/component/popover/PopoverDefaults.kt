package co.typie.ui.component.popover

import androidx.compose.animation.core.EaseInOutCubic
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.Easing
import androidx.compose.ui.unit.dp
import co.typie.ui.theme.AppShapes

object PopoverDefaults {
  val ExpandedRadius = AppShapes.xl
  val PanePadding = 6.dp
  val ScreenPadding = 16.dp
  const val ForwardDuration = 240
  const val ReverseDuration = 180
  const val IndicatorDuration = 140
  const val ArmDelayMs = 180L

  val OpenEasing: Easing = EaseOutCubic
  val CloseEasing: Easing = EaseInOutCubic
}
