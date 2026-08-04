package co.typie.navigation

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.platform.testTag
import co.typie.ui.component.TypieProgressiveBlurEffect
import co.typie.ui.component.topbar.LocalTopBarAnimationSource
import co.typie.ui.component.topbar.TopBarDefaults
import dev.chrisbanes.haze.ExperimentalHazeApi
import dev.chrisbanes.haze.HazeState
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
@OptIn(ExperimentalHazeApi::class)
internal fun NavigationTopBarBackdrop(
  hazeState: HazeState,
  style: () -> NavigationTopBarBackdropStyle,
  modifier: Modifier = Modifier,
) {
  val animationSource = LocalTopBarAnimationSource.current
  val animatedAlpha = animationSource?.animatedAlpha ?: 0f
  if (animatedAlpha <= 0f) return
  val styleState = rememberUpdatedState(style)

  val topPadding = TopBarDefaults.topPadding()
  val blurEffect = remember {
    TypieProgressiveBlurEffect(
      blurRadius = TopBarDefaults.BlurRadius,
      progressiveBrush = navigationTopBarProgressiveBrush(Color.Black),
      fallbackProgressive = TopBarDefaults.hazeProgressive(),
      backgroundColor = { styleState.value().background },
    )
  }
  val backdropModifier =
    modifier
      .fillMaxWidth()
      .height(topPadding + TopBarDefaults.Height + TopBarDefaults.BlurFadeHeight)
      .testTag(NavigationTopBarBackdropTestTag)
      .graphicsLayer {
        val resolvedStyle = styleState.value()
        alpha = animatedAlpha * resolvedStyle.presence
        translationY = (animationSource?.animatedTranslationY ?: 0f) * size.height
      }

  Box(modifier = backdropModifier) {
    Column(modifier = Modifier.fillMaxWidth().hazeEffect(hazeState) { visualEffect = blurEffect }) {
      Spacer(Modifier.fillMaxWidth().height(topPadding + TopBarDefaults.Height))
      Spacer(Modifier.height(TopBarDefaults.BlurFadeHeight))
    }
    Column(modifier = Modifier.fillMaxWidth()) {
      Spacer(
        Modifier.fillMaxWidth().height(topPadding).drawBehind {
          drawRect(styleState.value().background.copy(alpha = TopBarDefaults.FadeOpacity))
        }
      )
      Spacer(
        Modifier.fillMaxWidth()
          .height(TopBarDefaults.Height + TopBarDefaults.BlurFadeHeight)
          .drawBehind {
            val fadeColor = styleState.value().background.copy(alpha = TopBarDefaults.FadeOpacity)
            drawRect(navigationTopBarProgressiveBrush(fadeColor))
          }
      )
    }
  }
}

private fun navigationTopBarProgressiveBrush(color: Color): Brush {
  val stops =
    Array(NavigationTopBarFadeSamples + 1) { index ->
      val t = index / NavigationTopBarFadeSamples.toFloat()
      val alpha = color.alpha * (1f - TopBarDefaults.BlurFadeEasing.transform(t))
      t to color.copy(alpha = alpha)
    }

  return Brush.verticalGradient(colorStops = stops)
}
