package co.typie.shell

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import co.typie.graphql.Apollo
import co.typie.graphql.MainShell_SiteUpdateStream_Subscription
import co.typie.navigation.Nav
import co.typie.navigation.NavigationScaffold
import co.typie.navigation.NavigationStack
import co.typie.navigation.Navigator
import co.typie.route.Route
import co.typie.storage.Preference
import co.typie.ui.component.bottombar.BottomBarState
import co.typie.ui.component.drawer.Drawer
import co.typie.ui.component.drawer.LocalDrawer
import co.typie.ui.component.topbar.TopBarState
import kotlinx.coroutines.flow.collect

@Composable
fun MainShell(content: @Composable (Route) -> Unit) {
  var currentTab by remember { mutableStateOf(Tab.entries.first()) }
  val navigators = remember { Tab.entries.associateWith { Navigator(it.route) } }
  val activeNavigator = navigators[currentTab]!!

  val topBarState = remember { TopBarState() }
  val bottomBarState = remember { BottomBarState() }
  val drawer = remember { Drawer() }
  val drawerSwipeModifier = mainDrawerSwipeToOpenModifier(drawer, enabled = !activeNavigator.canPop)

  val siteId = Preference.siteId

  DisposableEffect(Unit) { onDispose { navigators.values.forEach { it.clear() } } }

  LaunchedEffect(currentTab) { if (drawer.isOpen) drawer.close() }

  LaunchedEffect(siteId) {
    if (siteId == null) {
      return@LaunchedEffect
    }

    Apollo.subscription(MainShell_SiteUpdateStream_Subscription(siteId = siteId))
      .retryOnError(true)
      .toFlow()
      .collect()
  }

  CompositionLocalProvider(
    LocalTabState provides TabState(currentTab = currentTab, onSelectTab = { currentTab = it }),
    LocalDrawer provides drawer,
    Nav provides activeNavigator,
  ) {
    Box(Modifier.fillMaxSize()) {
      NavigationScaffold(
        navigator = activeNavigator,
        topBarState = topBarState,
        bottomBarState = bottomBarState,
        modifier = drawerSwipeModifier,
      ) {
        Crossfade(
          targetState = currentTab,
          modifier = Modifier.fillMaxSize(),
          animationSpec = tween(200),
        ) { tab ->
          NavigationStack(
            navigator = navigators[tab]!!,
            topBarState = topBarState,
            bottomBarState = bottomBarState,
            content = content,
          )
        }
      }
      MainDrawerOverlay(drawer)
    }
  }
}

enum class Tab(val route: Route) {
  Home(Route.Home),
  Space(Route.Space),
  Notes(Route.Notes),
}

class TabState(val currentTab: Tab, val onSelectTab: (Tab) -> Unit)

val LocalTabState = compositionLocalOf<TabState> { error("LocalTabState not provided") }
