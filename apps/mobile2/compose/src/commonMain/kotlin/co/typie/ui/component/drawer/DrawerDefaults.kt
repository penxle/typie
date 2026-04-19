package co.typie.ui.component.drawer

import androidx.compose.animation.core.SpringSpec
import androidx.compose.animation.core.spring
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

object DrawerDefaults {
  val MaxWidth: Dp = 340.dp
  const val WidthFraction: Float = 0.85f
  val EdgeHitSlop: Dp = 20.dp
  const val ScrimAlpha: Float = 0.5f

  val AnimationSpec: SpringSpec<Float> = spring(stiffness = 500f)
}
