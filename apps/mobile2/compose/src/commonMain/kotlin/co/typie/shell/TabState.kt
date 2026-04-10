package co.typie.shell

import androidx.compose.runtime.compositionLocalOf
import co.typie.route.Route

enum class Tab(val route: Route) {
  Home(Route.Home), Space(Route.Space), Notes(Route.Notes), More(Route.More),
}

class TabState(
  val currentTab: Tab,
  val onSelectTab: (Tab) -> Unit,
)

val LocalTabState = compositionLocalOf<TabState> { error("LocalTabState not provided") }
