package co.typie.navigation

import androidx.compose.animation.core.EaseOutQuint
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.platform.testTag
import co.typie.ui.component.topbar.LocalTopBarAnimationSource
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.component.typieProgressiveBlur
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.HazeInput
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.hazeEffect
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow

internal const val NavigationTopBarBackdropTestTag = "navigation-top-bar-backdrop"
internal const val NavigationSceneSurfaceCompositeTestTag = "navigation-scene-surface-composite"

private const val NAVIGATION_TOP_BAR_FADE_SAMPLES = 48
private const val NAVIGATION_TOP_BAR_LUMINANCE_ANIMATION_DURATION_MILLIS = 900
private const val NAVIGATION_TOP_BAR_LUMINANCE_SAMPLE_DELAY_MILLIS = 100L
private const val NAVIGATION_TOP_BAR_BLUR_ANIMATION_DURATION_MILLIS = 250

internal data class NavigationTopBarBackdropStyle(val background: Color, val presence: Float)

internal sealed interface NavigationTopBarLuminanceMode {
  data class Live(val key: Any) : NavigationTopBarLuminanceMode

  data class Frozen(val contentLuminance: NavigationTopBarContentLuminance) :
    NavigationTopBarLuminanceMode
}

internal data class NavigationTopBarMeasuredContentLuminance(
  val token: NavigationTopBarSampleToken,
  val contentLuminance: NavigationTopBarContentLuminance,
)

internal data class NavigationTopBarTransitionAppearance(
  val blurEnabled: Boolean,
  val contentLuminance: NavigationTopBarContentLuminance,
)

internal fun resolveNavigationTopBarTransitionAppearance(
  themeMode: ResolvedThemeMode,
  committed: Boolean,
  sourceBlurEnabled: Boolean,
  sourceContentLuminance: NavigationTopBarContentLuminance,
  destinationBlurEnabled: Boolean,
  destinationContentLuminance: NavigationTopBarContentLuminance?,
): NavigationTopBarTransitionAppearance =
  if (committed) {
    NavigationTopBarTransitionAppearance(
      blurEnabled = destinationBlurEnabled,
      contentLuminance =
        destinationContentLuminance ?: defaultNavigationTopBarContentLuminance(themeMode),
    )
  } else {
    NavigationTopBarTransitionAppearance(
      blurEnabled = sourceBlurEnabled,
      contentLuminance = sourceContentLuminance,
    )
  }

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
  style: () -> NavigationTopBarBackdropStyle,
  luminanceMode: NavigationTopBarLuminanceMode,
  themeMode: ResolvedThemeMode,
  modifier: Modifier = Modifier,
  blurEnabled: Boolean = true,
  sampleRequests: Flow<Unit> = emptyFlow(),
  onMeasuredContentLuminance: (NavigationTopBarMeasuredContentLuminance) -> Unit = {},
) {
  val animationSource = LocalTopBarAnimationSource.current
  val animatedAlpha = animationSource?.animatedAlpha ?: 0f
  if (animatedAlpha <= 0f) return
  val styleState = rememberUpdatedState(style)

  val topPadding = TopBarDefaults.topPadding()
  val activeSampleToken =
    remember(themeMode, luminanceMode) {
      (luminanceMode as? NavigationTopBarLuminanceMode.Live)?.let {
        NavigationTopBarSampleToken(it.key, themeMode)
      }
    }
  val activeSampleTokenState = rememberUpdatedState(activeSampleToken)
  var resolvedContentLuminance by
    remember(themeMode) { mutableStateOf(defaultNavigationTopBarContentLuminance(themeMode)) }
  var pendingMeasurement by
    remember(themeMode) { mutableStateOf<NavigationTopBarMeasuredContentLuminance?>(null) }
  val onMeasuredContentLuminanceState = rememberUpdatedState(onMeasuredContentLuminance)
  LaunchedEffect(themeMode, luminanceMode) {
    val frozenMode = luminanceMode as? NavigationTopBarLuminanceMode.Frozen ?: return@LaunchedEffect
    pendingMeasurement = null
    resolvedContentLuminance = frozenMode.contentLuminance
  }
  LaunchedEffect(pendingMeasurement, activeSampleToken) {
    val measurement = pendingMeasurement ?: return@LaunchedEffect
    if (measurement.token != activeSampleToken) return@LaunchedEffect
    val delayMillis =
      navigationTopBarLuminanceTransitionDelayMillis(
        from = resolvedContentLuminance,
        to = measurement.contentLuminance,
      )
    if (delayMillis > 0L) delay(delayMillis)
    if (pendingMeasurement != measurement || measurement.token != activeSampleToken) {
      return@LaunchedEffect
    }
    resolvedContentLuminance = measurement.contentLuminance
    onMeasuredContentLuminanceState.value(measurement)
  }

  val resolvedLuminanceStyle =
    navigationTopBarLuminanceStyle(
      themeMode = themeMode,
      contentLuminance = resolvedContentLuminance,
    )

  val backdropOpacity by
    animateFloatAsState(
      targetValue = resolvedLuminanceStyle.backdropOpacity,
      animationSpec =
        tween(
          durationMillis = NAVIGATION_TOP_BAR_LUMINANCE_ANIMATION_DURATION_MILLIS,
          easing = EaseOutQuint,
        ),
      label = "navigation-top-bar-backdrop-opacity",
    )
  val darkTintOpacity by
    animateFloatAsState(
      targetValue = resolvedLuminanceStyle.darkTintOpacity,
      animationSpec =
        tween(
          durationMillis = NAVIGATION_TOP_BAR_LUMINANCE_ANIMATION_DURATION_MILLIS,
          easing = EaseOutQuint,
        ),
      label = "navigation-top-bar-dark-tint-opacity",
    )

  val onLuminancePixels =
    rememberUpdatedState<(NavigationTopBarSampleToken, IntArray, Int) -> Unit> {
      token,
      pixels,
      width ->
      if (token != activeSampleTokenState.value) {
        return@rememberUpdatedState
      }
      val contentLuminance =
        classifyNavigationTopBarContentLuminance(
          themeMode = token.themeMode,
          sample = analyzeNavigationTopBarPixels(pixels = pixels, width = width),
          current = resolvedContentLuminance,
        )
      pendingMeasurement = NavigationTopBarMeasuredContentLuminance(token, contentLuminance)
    }
  val samplingEffect =
    remember(topPadding) {
      NavigationTopBarSamplingEffect(
        sampleTopInset = topPadding,
        sampleHeight = TopBarDefaults.Height,
        backgroundColor = { styleState.value().background },
        samplingEnabled = { activeSampleTokenState.value != null },
        sampleToken = { activeSampleTokenState.value },
        onPixels = { token, pixels, width -> onLuminancePixels.value(token, pixels, width) },
      )
    }
  LaunchedEffect(themeMode, samplingEffect, luminanceMode) {
    if (luminanceMode is NavigationTopBarLuminanceMode.Live) {
      samplingEffect.requestSample()
    }
  }
  val hazeInput = remember(hazeState) { HazeInput.Sources(hazeState) }
  val blurProgressiveBrush = remember { navigationTopBarProgressiveBrush(Color.Black) }
  val blurModifier =
    typieProgressiveBlur(
      hazeState = hazeState,
      radius = TopBarDefaults.BlurRadius,
      progressiveBrush = blurProgressiveBrush,
      fallbackProgressive = TopBarDefaults.hazeProgressive(),
      backdropColor = { styleState.value().background },
    )
  LaunchedEffect(samplingEffect, sampleRequests, luminanceMode) {
    if (luminanceMode !is NavigationTopBarLuminanceMode.Live) return@LaunchedEffect
    sampleRequests.collect {
      delay(NAVIGATION_TOP_BAR_LUMINANCE_SAMPLE_DELAY_MILLIS)
      samplingEffect.requestSample()
    }
  }
  val blurAlpha by
    animateFloatAsState(
      targetValue = if (blurEnabled) 1f else 0f,
      animationSpec =
        tween(
          durationMillis = NAVIGATION_TOP_BAR_BLUR_ANIMATION_DURATION_MILLIS,
          easing = EaseOutQuint,
        ),
      label = "navigation-top-bar-blur-alpha",
    )
  val backdropModifier =
    modifier
      .fillMaxWidth()
      .height(topPadding + TopBarDefaults.Height + TopBarDefaults.BackdropFadeHeight)
      .testTag(NavigationTopBarBackdropTestTag)
      .graphicsLayer {
        val resolvedStyle = styleState.value()
        alpha = animatedAlpha * resolvedStyle.presence
        translationY = (animationSource?.animatedTranslationY ?: 0f) * size.height
      }

  Box(modifier = backdropModifier) {
    Column(
      modifier =
        Modifier.fillMaxWidth()
          .hazeEffect(factory = samplingEffect, input = hazeInput, style = Unit)
    ) {
      Spacer(Modifier.fillMaxWidth().height(topPadding + TopBarDefaults.Height))
      Spacer(Modifier.height(TopBarDefaults.BackdropFadeHeight))
    }
    if (blurAlpha > 0f) {
      Column(
        modifier = Modifier.fillMaxWidth().graphicsLayer { alpha = blurAlpha }.then(blurModifier)
      ) {
        Spacer(Modifier.fillMaxWidth().height(topPadding + TopBarDefaults.Height))
        Spacer(Modifier.height(TopBarDefaults.BackdropFadeHeight))
      }
    }
    Column(modifier = Modifier.fillMaxWidth()) {
      Spacer(
        Modifier.fillMaxWidth().height(topPadding).drawBehind {
          drawRect(styleState.value().background.copy(alpha = backdropOpacity))
          drawRect(Color.Black.copy(alpha = darkTintOpacity))
        }
      )
      Spacer(
        Modifier.fillMaxWidth()
          .height(TopBarDefaults.Height + TopBarDefaults.BackdropFadeHeight)
          .drawBehind {
            drawRect(
              navigationTopBarProgressiveBrush(
                styleState.value().background.copy(alpha = backdropOpacity)
              )
            )
            drawRect(navigationTopBarProgressiveBrush(Color.Black.copy(alpha = darkTintOpacity)))
          }
      )
    }
  }
}

private fun navigationTopBarProgressiveBrush(color: Color): Brush {
  val stops =
    Array(NAVIGATION_TOP_BAR_FADE_SAMPLES + 1) { index ->
      val t = index / NAVIGATION_TOP_BAR_FADE_SAMPLES.toFloat()
      val alpha = color.alpha * (1f - TopBarDefaults.BackdropFadeEasing.transform(t))
      t to color.copy(alpha = alpha)
    }

  return Brush.verticalGradient(colorStops = stops)
}
