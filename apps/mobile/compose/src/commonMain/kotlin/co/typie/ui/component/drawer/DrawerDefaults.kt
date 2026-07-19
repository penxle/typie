package co.typie.ui.component.drawer

import androidx.compose.animation.core.AnimationSpec
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.tween
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

object DrawerDefaults {
  val MaxWidth: Dp = 340.dp
  const val WidthFraction: Float = 0.85f
  val EdgeHitSlop: Dp = 20.dp
  const val ScrimAlpha: Float = 0.5f

  // AndroidX support@83040cf Material3 NavigationDrawer: 50%, 400.dp/s, 256ms tween.
  const val PositionalThresholdFraction: Float = 0.5f
  val VelocityThreshold: Dp = 400.dp

  val AnimationSpec: AnimationSpec<Float> =
    tween(durationMillis = 256, easing = FastOutSlowInEasing)
}
