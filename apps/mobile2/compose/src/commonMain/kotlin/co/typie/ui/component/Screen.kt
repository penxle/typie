package co.typie.ui.component

import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.ext.imeOrNavigationBarsPadding
import co.typie.ext.navigationBarsPadding
import co.typie.ext.plus
import co.typie.ext.statusBars
import co.typie.ui.component.bottombar.BottomBarDefaults
import co.typie.ui.component.bottombar.LocalBottomBarAnimationSource
import co.typie.ui.component.topbar.LocalTopBarAnimationSource
import co.typie.ui.component.topbar.LocalTopBarState
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.hazeEffect
import dev.chrisbanes.haze.hazeSource

private val MaxContentWidth = 600.dp

@Composable
fun Screen(
  loading: Boolean = false,
  background: Color = AppTheme.colors.surfaceBase,
  contentPadding: PaddingValues = PaddingValues(horizontal = 16.dp),
  imeAware: Boolean = false,
  bottomBar: (@Composable BoxScope.() -> Unit)? = null,
  content: @Composable (contentPadding: PaddingValues) -> Unit,
) {
  val topBarState = LocalTopBarState.current
  val hasTopBar = topBarState != null && topBarState.enabled && topBarState.visible
  val statusBarTop = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
  val adjustedContentPadding =
    if (hasTopBar) {
      contentPadding +
        PaddingValues(
          top =
            statusBarTop +
              TopBarDefaults.Height +
              TopBarDefaults.BlurFadeHeight +
              TopBarDefaults.ContentTopSpacing
        )
    } else {
      contentPadding
    }

  Box(Modifier.fillMaxSize().background(background)) {
    val hazeState = remember { HazeState() }

    Box(
      modifier = Modifier.fillMaxSize().hazeSource(hazeState),
      contentAlignment = Alignment.TopCenter,
    ) {
      Box(Modifier.widthIn(max = MaxContentWidth).fillMaxSize()) {
        Skeleton(enabled = loading) {
          var bottomBarHeight by remember { mutableIntStateOf(0) }
          val density = LocalDensity.current
          val bottomBarPadding = PaddingValues(bottom = with(density) { bottomBarHeight.toDp() })
          val resolvedContentPadding = adjustedContentPadding + bottomBarPadding

          Box(
            modifier =
              Modifier.fillMaxSize()
                .then(if (imeAware) Modifier.imeOrNavigationBarsPadding() else Modifier)
          ) {
            content(resolvedContentPadding)

            if (bottomBar != null) {
              Box(
                modifier =
                  Modifier.align(Alignment.BottomCenter)
                    .fillMaxWidth()
                    .then(if (!imeAware) Modifier.navigationBarsPadding() else Modifier)
                    .onSizeChanged { bottomBarHeight = it.height }
              ) {
                bottomBar()
              }
            }
          }
        }
      }
    }

    val topBarAnimation = LocalTopBarAnimationSource.current
    val topBarEnabled = topBarState != null && topBarState.enabled
    val topBarVisAlpha = topBarAnimation?.animatedVisibilityAlpha ?: 0f
    val topBarVisOffsetY = topBarAnimation?.animatedVisibilityOffsetY ?: 0f
    if (topBarEnabled && topBarVisAlpha > 0f) {
      Column(
        modifier =
          Modifier.fillMaxWidth()
            .align(Alignment.TopStart)
            .graphicsLayer {
              alpha = topBarVisAlpha
              translationY = topBarVisOffsetY * size.height
            }
            .hazeEffect(hazeState) {
              backgroundColor = background
              blurRadius = TopBarDefaults.BlurRadius * (topBarState?.blurFactor ?: 1f)
              progressive = TopBarDefaults.hazeProgressive()
            }
      ) {
        Spacer(
          Modifier.fillMaxWidth()
            .windowInsetsPadding(WindowInsets.statusBars)
            .height(TopBarDefaults.Height)
        )
        Spacer(Modifier.height(TopBarDefaults.BlurFadeHeight))
      }
    }

    val bottomBarAnimation = LocalBottomBarAnimationSource.current
    val bgAlpha = bottomBarAnimation?.animatedAlpha ?: 0f
    val bgTranslationY = bottomBarAnimation?.animatedTranslationY ?: 0f
    if (bgAlpha > 0f) {
      val fadeColor = background.copy(alpha = BottomBarDefaults.FadeOpacity)
      Column(
        modifier =
          Modifier.fillMaxWidth().align(Alignment.BottomStart).graphicsLayer {
            alpha = bgAlpha
            translationY = bgTranslationY
          }
      ) {
        Spacer(
          Modifier.fillMaxWidth()
            .height(BottomBarDefaults.FadeHeight)
            .background(
              Brush.verticalGradient(colors = listOf(fadeColor.copy(alpha = 0f), fadeColor))
            )
        )
        Spacer(
          Modifier.fillMaxWidth()
            .background(fadeColor)
            .navigationBarsPadding()
            .height(BottomBarDefaults.BarAreaHeight)
        )
      }
    }
  }
}
