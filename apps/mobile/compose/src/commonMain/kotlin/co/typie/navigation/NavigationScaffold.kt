package co.typie.navigation

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import co.typie.ui.component.bottombar.BottomBar
import co.typie.ui.component.bottombar.BottomBarState
import co.typie.ui.component.topbar.TopBar
import co.typie.ui.component.topbar.TopBarState
import co.typie.ui.theme.AppTheme

private const val STATUS_BAR_PRESENTED_ENTER_ALPHA = 0.65f
private const val STATUS_BAR_PRESENTED_EXIT_ALPHA = 0.35f

internal fun resolveAdaptiveStatusBarPresented(
  previous: Boolean,
  enabled: Boolean,
  alpha: Float,
): Boolean =
  when {
    !enabled -> false
    alpha >= STATUS_BAR_PRESENTED_ENTER_ALPHA -> true
    alpha <= STATUS_BAR_PRESENTED_EXIT_ALPHA -> false
    else -> previous
  }

@Composable
fun NavigationScaffold(
  navigator: Navigator,
  topBarState: TopBarState,
  bottomBarState: BottomBarState? = null,
  modifier: Modifier = Modifier,
  overlay: @Composable BoxScope.() -> Unit = {},
  content: @Composable () -> Unit,
) {
  val defaultForegroundStyle =
    defaultNavigationTopBarLuminanceStyle(AppTheme.themeMode).foregroundStyle
  var previousAdaptiveStatusBarPresented by remember { mutableStateOf(false) }
  val adaptiveStatusBarPresented =
    resolveAdaptiveStatusBarPresented(
      previous = previousAdaptiveStatusBarPresented,
      enabled = topBarState.enabled,
      alpha = topBarState.animatedAlpha,
    )
  SideEffect { previousAdaptiveStatusBarPresented = adaptiveStatusBarPresented }
  val statusBarForegroundStyle =
    if (adaptiveStatusBarPresented) {
      topBarState.adaptiveForegroundStyle ?: defaultForegroundStyle
    } else {
      defaultForegroundStyle
    }
  PublishNavigationStatusBarAppearance(statusBarForegroundStyle)

  Box(modifier.fillMaxSize()) {
    Box(Modifier.fillMaxSize()) { content() }

    CompositionLocalProvider(Nav provides navigator) { TopBar(state = topBarState) }

    if (bottomBarState != null) {
      BottomBar(state = bottomBarState)
    }

    overlay()
  }
}
