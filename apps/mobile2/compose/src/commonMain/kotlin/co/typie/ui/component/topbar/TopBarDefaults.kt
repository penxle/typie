package co.typie.ui.component.topbar

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import dev.chrisbanes.haze.HazeProgressive

object TopBarDefaults {
  val Height: Dp = 48.dp
  val HorizontalPadding: Dp = 20.dp
  val SlotWidth: Dp = 44.dp
  val SlotGap: Dp = 12.dp
  val RevealOffset: Dp = 44.dp

  val BlurRadius: Dp = 18.dp
  val BlurFadeHeight: Dp = 16.dp
  val ContentTopSpacing: Dp = 8.dp

  val RevealAnimationDuration: Int = 200
  val RevealFadeDuration: Int = 150
  val VisibilityAnimationDuration: Int = 200
  val VisibilityFadeDuration: Int = 150

  val ButtonShape: Shape = AppShapes.circle
  val ButtonSize: Dp = SlotWidth
  val ButtonIconSize: Dp = 18.dp

  val TitleHeight: Dp = SlotWidth
  val TitleIconSize: Dp = 18.dp
  val TitleHorizontalPadding: Dp = 14.dp
  val TitleIconGap: Dp = 10.dp

  fun hazeProgressive(): HazeProgressive =
    HazeProgressive.verticalGradient(startIntensity = 1f, endIntensity = 0f)

  @Composable fun controlBackgroundColor(): Color = AppTheme.colors.surfaceRaised

  @Composable fun controlBorderColor(): Color = AppTheme.colors.borderStrong

  @Composable
  fun controlShadowModifier(shape: Shape = AppShapes.circle): Modifier =
    Modifier.shadow(
      elevation = 4.dp,
      shape = shape,
      ambientColor = AppTheme.colors.shadowAmbient,
      spotColor = AppTheme.colors.shadow,
    )
}
