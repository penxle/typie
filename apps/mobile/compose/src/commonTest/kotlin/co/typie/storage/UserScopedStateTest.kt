package co.typie.storage

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

private class FakeStore {
  val map = mutableMapOf<String, Any?>()

  fun <T> storage(defaultValue: T, loadOverride: ((String) -> T)? = null): UserScopedStorage<T> =
    UserScopedStorage(
      load =
        loadOverride
          ?: { key ->
            if (map.containsKey(key)) @Suppress("UNCHECKED_CAST") (map[key] as T) else defaultValue
          },
      save = { key, value -> map[key] = value },
      exists = { key -> map.containsKey(key) },
      delete = { key -> map.remove(key) },
    )
}

class UserScopedStateTest {
  @Test
  fun switchUserIsolatesValuesBetweenUsers() {
    val store = FakeStore()
    val registry = UserScopedRegistry()
    val state = UserScopedState(baseKey = "k", defaultValue = "d", storage = store.storage("d"))
    registry.register(state)

    registry.switchUser("A")
    state.value = "a1"
    assertEquals("a1", store.map["k@A"])

    registry.switchUser("B")
    assertEquals("d", state.value)
    state.value = "b1"
    assertEquals("b1", store.map["k@B"])

    registry.switchUser("A")
    assertEquals("a1", state.value)
  }

  @Test
  fun switchUserNullResetsToDefaultAndSkipsPersistence() {
    val store = FakeStore()
    val state = UserScopedState(baseKey = "k", defaultValue = "d", storage = store.storage("d"))

    state.rebind("A")
    state.value = "a1"
    state.rebind(null)
    assertEquals("d", state.value)

    state.value = "ghost"
    assertFalse(store.map.containsKey("k"))
    assertEquals("a1", store.map["k@A"])
  }

  @Test
  fun migrationMovesFlatValueOnceAndDeletesFlatKey() {
    val store = FakeStore()
    val state = UserScopedState(baseKey = "k", defaultValue = "d", storage = store.storage("d"))
    store.map["k"] = "legacy"

    state.rebind("A")
    assertEquals("legacy", state.value)
    assertEquals("legacy", store.map["k@A"])
    assertFalse(store.map.containsKey("k"))

    store.map["k@A"] = "changed"
    state.rebind("A")
    assertEquals("changed", state.value)
  }

  @Test
  fun migrationSkipsCopyWhenUserKeyExists() {
    val store = FakeStore()
    val state = UserScopedState(baseKey = "k", defaultValue = "d", storage = store.storage("d"))
    store.map["k"] = "legacy"
    store.map["k@A"] = "mine"

    state.rebind("A")
    assertEquals("mine", state.value)
    assertFalse(store.map.containsKey("k"))
  }

  @Test
  fun writesBeforeAnySwitchStayInMemory() {
    val store = FakeStore()
    val state = UserScopedState(baseKey = "k", defaultValue = "d", storage = store.storage("d"))

    state.value = "early"
    assertEquals("early", state.value)
    assertTrue(store.map.isEmpty())
  }

  @Test
  fun migrationLoadFailureFallsBackToDefaultAndStillDeletesFlatKey() {
    val store = FakeStore()
    store.map["k"] = "corrupt"
    val throwingLoad: (String) -> String = { key ->
      if (key == "k") error("deserialization failed")
      else if (store.map.containsKey(key)) store.map[key] as String else "d"
    }
    val state =
      UserScopedState(baseKey = "k", defaultValue = "d", storage = store.storage("d", throwingLoad))

    state.rebind("A")
    assertEquals("d", state.value)
    assertFalse(store.map.containsKey("k"))
    assertNull(store.map["k@A"])
  }
}
