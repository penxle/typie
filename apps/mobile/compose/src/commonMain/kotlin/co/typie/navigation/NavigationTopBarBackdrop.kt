package co.typie.navigation

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.platform.testTag
import co.typie.ui.component.SmootherstepEasing
import co.typie.ui.component.topbar.LocalTopBarAnimationSource
import co.typie.ui.component.topbar.TopBarDefaults
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.blur.blurEffect
import dev.chrisbanes.haze.hazeEffect

internal const val NavigationTopBarBackdropTestTag = "navigation-top-bar-backdrop"
internal const val NavigationSceneSurfaceCompositeTestTag = "navigation-scene-surface-composite"

private const val NavigationTopBarFadeSamples = 48

internal data class NavigationTopBarBackdropStyle(val background: Color, val presence: Float)

internal fun resolveNavigationTopBarBackdropStyle(
  behindBackground: Color?,
  behindPresence: Float,
  mainBackground: Color?,
  mainPresence: Float,
  mainWeight: Float,
  fallbackBackground: Color,
): NavigationTopBarBackdropStyle {
  val resolvedMainWeight = mainWeight.coerceIn(0f, 1f)
  val behind = behindBackground ?: fallbackBackground
  val main = mainBackground ?: fallbackBackground

  return NavigationTopBarBackdropStyle(
    background = lerp(behind, main, resolvedMainWeight),
    presence =
      (behindPresence.coerceIn(0f, 1f) * (1f - resolvedMainWeight) +
          mainPresence.coerceIn(0f, 1f) * resolvedMainWeight)
        .coerceIn(0f, 1f),
  )
}

@Composable
internal fun NavigationTopBarBackdrop(
  hazeState: HazeState,
  style: NavigationTopBarBackdropStyle,
  modifier: Modifier = Modifier,
) {
  val animationSource = LocalTopBarAnimationSource.current
  val alpha = (animationSource?.animatedAlpha ?: 0f) * style.presence
  if (alpha <= 0f) return

  val topPadding = TopBarDefaults.topPadding()
  val backdropModifier =
    modifier
      .fillMaxWidth()
      .height(topPadding + TopBarDefaults.Height + TopBarDefaults.BlurFadeHeight)
      .testTag(NavigationTopBarBackdropTestTag)
      .graphicsLayer {
        this.alpha = alpha
        translationY = (animationSource?.animatedTranslationY ?: 0f) * size.height
      }

  val fadeColor = style.background.copy(alpha = TopBarDefaults.FadeOpacity)
  Box(modifier = backdropModifier) {
    Column(
      modifier =
        Modifier.fillMaxWidth().hazeEffect(hazeState) {
          blurEffect {
            backgroundColor = style.background
            blurRadius = TopBarDefaults.BlurRadius
            progressive = TopBarDefaults.hazeProgressive()
          }
        }
    ) {
      Spacer(Modifier.fillMaxWidth().height(topPadding + TopBarDefaults.Height))
      Spacer(Modifier.height(TopBarDefaults.BlurFadeHeight))
    }
    Column(modifier = Modifier.fillMaxWidth()) {
      Spacer(Modifier.fillMaxWidth().height(topPadding).background(fadeColor))
      Spacer(
        Modifier.fillMaxWidth()
          .height(TopBarDefaults.Height + TopBarDefaults.BlurFadeHeight)
          .background(navigationTopBarFadeBrush(fadeColor))
      )
    }
  }
}

private fun navigationTopBarFadeBrush(color: Color): Brush {
  val stops =
    Array(NavigationTopBarFadeSamples + 1) { index ->
      val t = index / NavigationTopBarFadeSamples.toFloat()
      val alpha = color.alpha * (1f - SmootherstepEasing.transform(t))
      t to color.copy(alpha = alpha)
    }

  return Brush.verticalGradient(colorStops = stops)
}
