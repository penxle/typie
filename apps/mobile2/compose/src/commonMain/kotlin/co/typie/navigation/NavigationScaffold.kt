package co.typie.navigation

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import co.typie.ui.component.bottombar.BottomBar
import co.typie.ui.component.bottombar.BottomBarState
import co.typie.ui.component.topbar.TopBar
import co.typie.ui.component.topbar.TopBarState

@Composable
fun NavigationScaffold(
  navigator: Navigator,
  topBarState: TopBarState,
  bottomBarState: BottomBarState? = null,
  modifier: Modifier = Modifier,
  overlay: @Composable BoxScope.() -> Unit = {},
  content: @Composable () -> Unit,
) {
  Box(modifier.fillMaxSize()) {
    Box(Modifier.fillMaxSize()) { content() }

    CompositionLocalProvider(Nav provides navigator) { TopBar(state = topBarState) }

    if (bottomBarState != null) {
      BottomBar(state = bottomBarState)
    }

    overlay()
  }
}
