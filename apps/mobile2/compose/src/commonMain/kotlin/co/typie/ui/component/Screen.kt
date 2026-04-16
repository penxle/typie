package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
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
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp
import co.typie.contract.Loadable
import co.typie.contract.LoadableState
import co.typie.ext.imePadding
import co.typie.ext.navigationBars
import co.typie.ext.navigationBarsPadding
import co.typie.ext.plus
import co.typie.ext.statusBars
import co.typie.ext.statusBarsPadding
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
import dev.chrisbanes.haze.hazeEffect
import dev.chrisbanes.haze.hazeSource

private val MaxContentWidth = 600.dp

@Composable
fun Screen(
  loadable: Loadable<*>? = null,
  background: Color = AppTheme.colors.surfaceBase,
  contentPadding: PaddingValues = PaddingValues(horizontal = 16.dp),
  content: @Composable (contentPadding: PaddingValues) -> Unit,
) {
  val hazeState = remember { HazeState() }

  val topBarState = LocalTopBarState.current
  val hasTopBar = topBarState != null && topBarState.enabled && topBarState.visible

  val nav = Nav.current
  val dialog = LocalDialog.current

  val contentPadding =
    if (hasTopBar) {
      PaddingValues(
        top =
          TopBarDefaults.Height + TopBarDefaults.BlurFadeHeight + TopBarDefaults.ContentTopSpacing
      ) +
        contentPadding +
        WindowInsets.statusBars.asPaddingValues() +
        WindowInsets.navigationBars.asPaddingValues()
    } else {
      contentPadding + WindowInsets.navigationBars.asPaddingValues()
    }

  LaunchedEffect(loadable?.state) {
    if (loadable?.state is LoadableState.Error) {
      dialog.error(nav = nav, onRetry = { loadable.refetch() })
    }
  }

  Box(Modifier.fillMaxSize().background(background).imePadding()) {
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
      Column(
        modifier =
          Modifier.fillMaxWidth()
            .align(Alignment.TopCenter)
            .graphicsLayer {
              alpha = topBarAlpha
              translationY = topBarTranslationY * size.height
            }
            .hazeEffect(hazeState) {
              backgroundColor = background
              blurRadius = TopBarDefaults.BlurRadius
              progressive = TopBarDefaults.hazeProgressive()
            }
      ) {
        Spacer(Modifier.fillMaxWidth().statusBarsPadding().height(TopBarDefaults.Height))
        Spacer(Modifier.height(TopBarDefaults.BlurFadeHeight))
      }
    }

    val bottomBarAlpha = LocalBottomBarAnimationSource.current?.animatedAlpha ?: 0f
    val bottomBarTranslationY = LocalBottomBarAnimationSource.current?.animatedTranslationY ?: 0f
    if (bottomBarAlpha > 0f) {
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
