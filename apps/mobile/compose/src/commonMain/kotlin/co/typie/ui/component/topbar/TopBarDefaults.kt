package co.typie.ui.component.topbar

import androidx.compose.animation.core.Easing
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.safeDrawingHorizontal
import co.typie.ext.statusBars
import co.typie.ui.component.SmootherstepEasing
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import dev.chrisbanes.haze.HazeProgressive

object TopBarDefaults {
  val Height: Dp = 48.dp
  val HorizontalPadding: Dp = 20.dp
  val LandscapeTopPadding: Dp = 8.dp
  val SlotWidth: Dp = 44.dp
  val SlotGap: Dp = 12.dp
  val RevealOffset: Dp = 44.dp

  val BlurRadius: Dp = 6.dp
  val BlurFadeHeight: Dp = 16.dp
  val FadeOpacity: Float = 0.8f
  val ContentTopSpacing: Dp = 8.dp
  val BlurFadeEasing: Easing = SmootherstepEasing

  val RevealAnimationDuration: Int = 200
  val RevealFadeDuration: Int = 150
  val VisibilityAnimationDuration: Int = 200
  val VisibilityFadeDuration: Int = 150

  val ButtonShape: Shape = AppShapes.circle
  val ButtonSize: Dp = SlotWidth
  val ButtonIconSize: Dp = 20.dp

  val TitleHeight: Dp = SlotWidth
  val TitleIconSize: Dp = 18.dp
  val TitleHorizontalPadding: Dp = 14.dp
  val TitleIconGap: Dp = 10.dp

  fun hazeProgressive(): HazeProgressive =
    HazeProgressive.verticalGradient(
      easing = BlurFadeEasing,
      startIntensity = 1f,
      endIntensity = 0f,
    )

  @Composable
  fun topPadding(): Dp {
    val direction = LocalLayoutDirection.current
    val statusTop = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
    val horizontalSafeArea = WindowInsets.safeDrawingHorizontal.asPaddingValues()
    val hasHorizontalSafeArea =
      horizontalSafeArea.calculateLeftPadding(direction) > 0.dp ||
        horizontalSafeArea.calculateRightPadding(direction) > 0.dp

    return if (statusTop == 0.dp && hasHorizontalSafeArea) LandscapeTopPadding else statusTop
  }

  @Composable fun topPaddingValues(): PaddingValues = PaddingValues(top = topPadding())

  @Composable fun controlBackgroundColor(): Color = AppTheme.colors.surfaceDefault

  @Composable fun controlBorderColor(): Color = AppTheme.colors.borderEmphasis
}
