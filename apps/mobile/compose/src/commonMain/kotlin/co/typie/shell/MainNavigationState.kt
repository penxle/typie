package co.typie.shell

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.setValue
import co.typie.navigation.Navigator
import co.typie.route.Route
import co.typie.storage.Preference
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

internal const val NAV_SAVE_VERSION = 1

@Serializable
internal data class NavSaveState(
  val version: Int,
  val siteId: String?,
  val tab: Tab,
  val stacks: Map<Tab, List<Route>>,
)

class MainNavigationState
internal constructor(initialTab: Tab, val navigators: Map<Tab, Navigator>) {
  var currentTab by mutableStateOf(initialTab)

  companion object {
    fun initial(): MainNavigationState =
      MainNavigationState(
        initialTab = Tab.entries.first(),
        navigators = Tab.entries.associateWith { Navigator(it.route) },
      )
  }
}

internal fun encodeNavSaveState(state: MainNavigationState, siteId: String?): String =
  Json.encodeToString(
    NavSaveState.serializer(),
    NavSaveState(
      version = NAV_SAVE_VERSION,
      siteId = siteId,
      tab = state.currentTab,
      stacks = state.navigators.mapValues { (_, navigator) -> navigator.stack.toList() },
    ),
  )

internal fun decodeNavSaveState(encoded: String, siteId: String?): MainNavigationState? {
  val saved =
    runCatching { Json.decodeFromString(NavSaveState.serializer(), encoded) }.getOrNull()
      ?: return null
  if (saved.version != NAV_SAVE_VERSION) return null
  if (saved.siteId != siteId) return null
  val stacks =
    Tab.entries.associateWith { tab ->
      val stack = saved.stacks[tab] ?: return null
      if (stack.firstOrNull() != tab.route) return null
      stack
    }
  return MainNavigationState(
    initialTab = saved.tab,
    navigators = stacks.mapValues { (_, stack) -> Navigator(stack) },
  )
}

internal val MainNavigationStateSaver: Saver<MainNavigationState, String> =
  Saver(
    save = { encodeNavSaveState(it, Preference.siteId) },
    restore = { decodeNavSaveState(it, Preference.siteId) },
  )
