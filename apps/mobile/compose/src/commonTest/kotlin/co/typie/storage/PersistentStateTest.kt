package co.typie.storage

import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import kotlin.test.Test
import kotlin.test.assertEquals

class PersistentStateTest {
  @Test
  fun `value read returns initial value`() {
    val state = PersistentState(initialValue = "hello") {}
    assertEquals("hello", state.value)
  }

  @Test
  fun `value write updates state and calls persist`() {
    var persisted: String? = null
    val state = PersistentState(initialValue = "a") { persisted = it }

    state.value = "b"

    assertEquals("b", state.value)
    assertEquals("b", persisted)
  }

  @Test
  fun `setting same value does not persist again`() {
    var persistCount = 0
    val state = PersistentState(initialValue = "a") { persistCount++ }

    state.value = "a"

    assertEquals(0, persistCount)
  }

  @Test
  fun `component1 returns current value`() {
    val state = PersistentState(initialValue = 42) {}
    val (value) = state
    assertEquals(42, value)
  }

  @Test
  fun `component2 returns setter that updates value`() {
    var persisted: Int? = null
    val state = PersistentState(initialValue = 0) { persisted = it }
    val (_, setter) = state
    setter(7)
    assertEquals(7, state.value)
    assertEquals(7, persisted)
  }

  @Test
  fun `by delegation read and write`() {
    var persisted: String? = null
    var name by PersistentState(initialValue = "init") { persisted = it }

    assertEquals("init", name)

    name = "updated"

    assertEquals("updated", name)
    assertEquals("updated", persisted)
  }
}
