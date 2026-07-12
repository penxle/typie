package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusManager
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.unit.dp
import co.typie.contract.Loadable
import co.typie.contract.LoadableState
import co.typie.ext.navigationBars
import co.typie.ext.navigationBarsPadding
import co.typie.ext.plus
import co.typie.ext.safeDrawingHorizontal
import co.typie.navigation.Nav
import co.typie.ui.component.bottombar.BottomBarDefaults
import co.typie.ui.component.bottombar.LocalBottomBarAnimationSource
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.error
import co.typie.ui.component.topbar.LocalTopBarAnimationSource
import co.typie.ui.component.topbar.LocalTopBarState
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.blur.blurEffect
import dev.chrisbanes.haze.hazeEffect
import dev.chrisbanes.haze.hazeSource

private val MaxContentWidth = 600.dp
private const val OverlayFadeSamples = 48

@Composable
fun Screen(
  loadable: Loadable<*>? = null,
  refetchOnMount: Boolean = true,
  background: Color = AppTheme.colors.surfaceCanvas,
  contentPadding: PaddingValues = PaddingValues(horizontal = 16.dp),
  dismissFocusOnTapOutsideInput: Boolean = true,
  overlay: (@Composable BoxScope.() -> Unit)? = null,
  content: @Composable BoxScope.(contentPadding: PaddingValues) -> Unit,
) {
  val hazeState = remember { HazeState() }

  val topBarState = LocalTopBarState.current
  val hasTopBar = topBarState != null && topBarState.enabled && topBarState.visible

  val nav = if (loadable != null) Nav.current else null
  val dialog = LocalDialog.current
  val focusManager = LocalFocusManager.current

  val topBarPadding = TopBarDefaults.topPaddingValues()
  val horizontalSafePadding = WindowInsets.safeDrawingHorizontal.asPaddingValues()
  val navigationBarPadding =
    PaddingValues(bottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding())
  val contentPadding =
    if (hasTopBar) {
      PaddingValues(
        top =
          TopBarDefaults.Height + TopBarDefaults.BlurFadeHeight + TopBarDefaults.ContentTopSpacing
      ) + contentPadding + topBarPadding + horizontalSafePadding + navigationBarPadding
    } else {
      contentPadding + horizontalSafePadding + navigationBarPadding
    }
  val shouldRefetchOnMount =
    remember(loadable) { loadable != null && loadable.state !is LoadableState.Loading }

  LaunchedEffect(loadable?.state) {
    if (loadable?.state is LoadableState.Error) {
      dialog.error(nav = nav, onRetry = { loadable.refetch() })
    }
  }

  if (refetchOnMount && loadable != null) {
    LaunchedEffect(loadable) {
      if (shouldRefetchOnMount) {
        loadable.refetch()
      }
    }
  }

  Box(
    Modifier.fillMaxSize()
      .background(background)
      .clearFocusOnUnhandledTap(
        enabled = dismissFocusOnTapOutsideInput,
        focusManager = focusManager,
      )
  ) {
    Box(
      modifier = Modifier.fillMaxSize().hazeSource(hazeState).widthIn(max = MaxContentWidth),
      contentAlignment = Alignment.TopCenter,
    ) {
      Skeleton(enabled = loadable != null && loadable.state !is LoadableState.Success) {
        Box(modifier = Modifier.fillMaxSize()) { content(contentPadding) }
      }
    }

    val topBarAlpha = LocalTopBarAnimationSource.current?.animatedAlpha ?: 0f
    val topBarTranslationY = LocalTopBarAnimationSource.current?.animatedTranslationY ?: 0f
    if (topBarState?.enabled == true && topBarAlpha > 0f) {
      val topBarSolidHeight = 0.dp
      val topBarFadeHeight = TopBarDefaults.Height + TopBarDefaults.BlurFadeHeight
      val topBarOverlayModifier =
        Modifier.fillMaxWidth().align(Alignment.TopCenter).graphicsLayer {
          alpha = topBarAlpha
          translationY = topBarTranslationY * size.height
        }

      Column(
        modifier =
          topBarOverlayModifier.hazeEffect(hazeState) {
            blurEffect {
              backgroundColor = background
              blurRadius = TopBarDefaults.BlurRadius
              progressive = TopBarDefaults.hazeProgressive()
            }
          }
      ) {
        Spacer(Modifier.fillMaxWidth().height(TopBarDefaults.topPadding() + TopBarDefaults.Height))
        Spacer(Modifier.height(TopBarDefaults.BlurFadeHeight))
      }

      val fadeColor = background.copy(alpha = TopBarDefaults.FadeOpacity)
      Column(modifier = topBarOverlayModifier) {
        Spacer(
          Modifier.fillMaxWidth()
            .background(fadeColor)
            .height(TopBarDefaults.topPadding() + topBarSolidHeight)
        )
        Spacer(
          Modifier.fillMaxWidth()
            .height(topBarFadeHeight)
            .background(overlayFadeBrush(color = fadeColor, reverse = false))
        )
      }
    }

    val bottomBarAlpha = LocalBottomBarAnimationSource.current?.animatedAlpha ?: 0f
    val bottomBarTranslationY = LocalBottomBarAnimationSource.current?.animatedTranslationY ?: 0f
    if (bottomBarAlpha > 0f) {
      val bottomBarSolidHeight = BottomBarDefaults.BottomPadding
      val bottomBarFadeHeight = BottomBarDefaults.FadeHeight + BottomBarDefaults.PillHeight
      val fadeColor = background.copy(alpha = BottomBarDefaults.FadeOpacity)
      Column(
        modifier =
          Modifier.fillMaxWidth().align(Alignment.BottomCenter).graphicsLayer {
            alpha = bottomBarAlpha
            translationY = bottomBarTranslationY
          }
      ) {
        Spacer(
          Modifier.fillMaxWidth()
            .height(bottomBarFadeHeight)
            .background(overlayFadeBrush(color = fadeColor, reverse = true))
        )
        Spacer(
          Modifier.fillMaxWidth()
            .background(fadeColor)
            .navigationBarsPadding()
            .height(bottomBarSolidHeight)
        )
      }
    }

    overlay?.invoke(this)
  }
}

private fun Modifier.clearFocusOnUnhandledTap(
  enabled: Boolean,
  focusManager: FocusManager,
): Modifier =
  if (!enabled) {
    this
  } else {
    pointerInput(focusManager) {
      awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Final)
        if (down.isConsumed) {
          return@awaitEachGesture
        }

        val origin = down.position
        var isTapCandidate = true
        var pressed = true
        while (pressed) {
          val event = awaitPointerEvent(pass = PointerEventPass.Final)
          val change = event.changes.firstOrNull { it.id == down.id } ?: return@awaitEachGesture
          if (change.isConsumed) {
            return@awaitEachGesture
          }
          if ((change.position - origin).getDistance() > viewConfiguration.touchSlop) {
            isTapCandidate = false
          }
          if (!change.pressed) {
            pressed = false
          }
        }

        if (isTapCandidate) {
          focusManager.clearFocus()
        }
      }
    }
  }

private fun overlayFadeBrush(color: Color, reverse: Boolean): Brush {
  val stops =
    Array(OverlayFadeSamples + 1) { index ->
      val t = index / OverlayFadeSamples.toFloat()
      val eased = SmootherstepEasing.transform(t)
      val alpha = color.alpha * if (reverse) eased else 1f - eased
      t to color.copy(alpha = alpha)
    }

  return Brush.verticalGradient(colorStops = stops)
}
