package co.typie.ui.component.topbar

import androidx.compose.runtime.Composable
import co.typie.route.Route
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class TopBarExplicitKeyOwnerCleanupTest {

  @Test
  fun clearRoute_removesSharedTrailingEntryOwnedByPoppedRoute() {
    val state = TopBarState()
    val sharedKey = Any()
    val owner = Route.Folder("child")
    val trailing: @Composable () -> Unit = {}

    state.setTrailing(sharedKey, trailing, owner)

    state.clearRoute(owner)

    assertFalse(state.trailingEntries.containsKey(sharedKey))
    assertEquals(TopBarState.NullKey, state.trailingKey)
    assertFalse(state.trailingOwners.containsKey(sharedKey))
  }

  @Test
  fun clearRoute_keepsSharedTrailingEntryWhenAnotherRouteReboundIt() {
    val state = TopBarState()
    val sharedKey = Any()
    val firstOwner = Route.Folder("child")
    val secondOwner = Route.Space
    val firstTrailing: @Composable () -> Unit = {}
    val secondTrailing: @Composable () -> Unit = {}

    state.setTrailing(sharedKey, firstTrailing, firstOwner)
    state.setTrailing(sharedKey, secondTrailing, secondOwner)

    state.clearRoute(firstOwner)

    assertTrue(state.trailingEntries.containsKey(sharedKey))
    assertTrue(state.trailingEntries.getValue(sharedKey) === secondTrailing)
    assertEquals(sharedKey, state.trailingKey)
  }

  @Test
  fun clearRoute_removesSharedLeadingEntryOwnedByPoppedRoute() {
    val state = TopBarState()
    val sharedKey = Any()
    val owner = Route.Space
    val leading: @Composable () -> Unit = {}

    state.setLeading(sharedKey, leading, owner)

    state.clearRoute(owner)

    assertFalse(state.leadingEntries.containsKey(sharedKey))
    assertEquals(TopBarState.NullKey, state.leadingKey)
    assertFalse(state.leadingOwners.containsKey(sharedKey))
  }

  @Test
  fun setTrailing_rebindsSharedExplicitKeyToNewInstanceWithoutChangingKeyOrOwner() {
    val state = TopBarState()
    val sharedKey = Any()
    val owner = Route.Space
    val firstInstance = Any()
    val secondInstance = Any()
    val firstTrailing: @Composable () -> Unit = {}
    val secondTrailing: @Composable () -> Unit = {}

    state.setTrailing(sharedKey, firstTrailing, owner, firstInstance)
    state.setTrailing(sharedKey, secondTrailing, owner, secondInstance)

    assertEquals(sharedKey, state.trailingKey)
    assertTrue(state.trailingEntries.getValue(sharedKey) === secondTrailing)
    assertEquals(owner, state.trailingOwners[sharedKey])
    assertEquals(secondInstance, state.trailingInstances[sharedKey])
    assertNotEquals(firstInstance, state.trailingInstances[sharedKey])
  }
}
