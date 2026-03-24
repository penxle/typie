package co.typie.navigation

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import co.typie.ui.component.topbar.TopBar
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.component.topbar.TopBarState
import co.typie.ui.theme.AppTheme
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.hazeEffect
import dev.chrisbanes.haze.hazeSource

@Composable
fun NavigationScaffold(
  navigator: Navigator,
  topBarState: TopBarState,
  modifier: Modifier = Modifier,
  overlay: @Composable BoxScope.() -> Unit = {},
  content: @Composable () -> Unit,
) {
  val hazeState = remember { HazeState() }
  val colors = AppTheme.colors
  val enabledFactor by animateFloatAsState(
    targetValue = if (topBarState.enabled) 1f else 0f,
    animationSpec = tween(200),
  )

  Box(modifier.fillMaxSize()) {
    Box(Modifier.fillMaxSize().hazeSource(hazeState)) {
      content()
    }

    CompositionLocalProvider(Nav provides navigator) {
      Column(
        modifier = Modifier
          .fillMaxWidth()
          .align(Alignment.TopStart)
          .hazeEffect(hazeState) {
            backgroundColor = colors.surfaceDefault
            blurRadius = TopBarDefaults.BlurRadius * topBarState.blurFactor * enabledFactor
            progressive = TopBarDefaults.hazeProgressive()
          },
      ) {
        TopBar(state = topBarState)
        Spacer(Modifier.height(TopBarDefaults.BlurFadeHeight))
      }
    }

    overlay()
  }
}
