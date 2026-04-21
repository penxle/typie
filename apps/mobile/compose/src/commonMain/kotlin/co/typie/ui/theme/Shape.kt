package co.typie.ui.theme

import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ui.shape.SquircleShape

object AppShapes {
  val sm: Dp = 6.dp
  val md: Dp = 12.dp
  val lg: Dp = 16.dp
  val xl: Dp = 24.dp
  val full: Dp = 999.dp

  val circle: Shape = CircleShape

  fun rounded(radius: Dp): Shape = RoundedCornerShape(radius)

  fun squircle(radius: Dp): Shape = SquircleShape(radius)
}
