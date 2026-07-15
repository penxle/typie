package co.typie.shell

import co.typie.navigation.Navigator
import co.typie.route.Route
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlinx.serialization.json.Json

class MainNavigationStateTest {
  private fun stateWith(
    tab: Tab = Tab.Space,
    spaceStack: List<Route> = listOf(Route.Space, Route.Folder("F1"), Route.Editor("E1")),
  ): MainNavigationState =
    MainNavigationState(
      initialTab = tab,
      navigators =
        Tab.entries.associateWith { entry ->
          if (entry == Tab.Space) Navigator(spaceStack) else Navigator(entry.route)
        },
    )

  @Test
  fun roundTripRestoresTabAndStacks() {
    val encoded = encodeNavSaveState(stateWith(), siteId = "S1")
    val restored = assertNotNull(decodeNavSaveState(encoded, siteId = "S1"))

    assertEquals(Tab.Space, restored.currentTab)
    assertEquals(
      listOf(Route.Space, Route.Folder("F1"), Route.Editor("E1")),
      restored.navigators[Tab.Space]?.stack,
    )
    assertEquals(listOf<Route>(Route.Home), restored.navigators[Tab.Home]?.stack)
    assertEquals(listOf<Route>(Route.Notes), restored.navigators[Tab.Notes]?.stack)
  }

  @Test
  fun malformedJsonFallsBackToNull() {
    assertNull(decodeNavSaveState("not-json", siteId = "S1"))
    assertNull(decodeNavSaveState("""{"version":1}""", siteId = "S1"))
  }

  @Test
  fun versionMismatchFallsBackToNull() {
    val encoded =
      Json.encodeToString(
        NavSaveState.serializer(),
        NavSaveState(
          version = 999,
          siteId = "S1",
          tab = Tab.Home,
          stacks = Tab.entries.associateWith { listOf(it.route) },
        ),
      )
    assertNull(decodeNavSaveState(encoded, siteId = "S1"))
  }

  @Test
  fun siteIdMismatchFallsBackToNull() {
    val encoded = encodeNavSaveState(stateWith(), siteId = "S1")
    assertNull(decodeNavSaveState(encoded, siteId = "S2"))
    assertNull(decodeNavSaveState(encoded, siteId = null))
  }

  @Test
  fun invalidStackHeadFallsBackToNull() {
    val encoded =
      Json.encodeToString(
        NavSaveState.serializer(),
        NavSaveState(
          version = NAV_SAVE_VERSION,
          siteId = "S1",
          tab = Tab.Space,
          stacks =
            Tab.entries.associateWith { entry ->
              if (entry == Tab.Space) listOf(Route.Home, Route.Editor("E1"))
              else listOf(entry.route)
            },
        ),
      )
    assertNull(decodeNavSaveState(encoded, siteId = "S1"))
  }

  @Test
  fun missingTabFallsBackToNull() {
    val encoded =
      Json.encodeToString(
        NavSaveState.serializer(),
        NavSaveState(
          version = NAV_SAVE_VERSION,
          siteId = "S1",
          tab = Tab.Home,
          stacks = mapOf(Tab.Home to listOf(Route.Home)),
        ),
      )
    assertNull(decodeNavSaveState(encoded, siteId = "S1"))
  }
}
