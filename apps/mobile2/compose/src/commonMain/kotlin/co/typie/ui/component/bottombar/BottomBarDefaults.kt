package co.typie.ui.component.bottombar

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.chrisbanes.haze.HazeProgressive

object BottomBarDefaults {
  val BottomPadding: Dp = 2.dp
  val PillHeight: Dp = 56.dp

  val BlurRadius: Dp = 18.dp
  val BlurFadeHeight: Dp = 80.dp
  val BarAreaHeight: Dp = PillHeight + BottomPadding

  fun hazeProgressive(): HazeProgressive =
    HazeProgressive.verticalGradient(
      startIntensity = 0f,
      endIntensity = 1f,
    )
}
