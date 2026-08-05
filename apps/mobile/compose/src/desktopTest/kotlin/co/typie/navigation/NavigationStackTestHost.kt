package co.typie.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import co.typie.route.Route
import co.typie.ui.component.bottombar.BottomBarState
import co.typie.ui.component.topbar.TopBarState
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode

@Composable
internal fun NavigationStackTestHost(
  navigator: Navigator,
  topBarState: TopBarState,
  bottomBarState: BottomBarState? = null,
  modifier: Modifier = Modifier,
  foregroundInteractive: Boolean = true,
  content: @Composable (Route) -> Unit,
) {
  CompositionLocalProvider(LocalThemeMode provides ResolvedThemeMode.Light) {
    NavigationStack(
      navigator = navigator,
      topBarState = topBarState,
      bottomBarState = bottomBarState,
      modifier = modifier,
      foregroundInteractive = foregroundInteractive,
      content = content,
    )
  }
}
