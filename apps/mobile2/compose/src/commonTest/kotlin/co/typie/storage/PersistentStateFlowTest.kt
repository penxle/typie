package co.typie.storage

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest

class PersistentStateFlowTest {
  @Test
  fun `value read returns initial value`() {
    val flow = PersistentStateFlow(initialValue = "hello") {}
    assertEquals("hello", flow.value)
  }

  @Test
  fun `value write updates flow and calls persist`() {
    var persisted: String? = null
    val flow = PersistentStateFlow(initialValue = "a") { persisted = it }

    flow.value = "b"

    assertEquals("b", flow.value)
    assertEquals("b", persisted)
  }

  @Test
  fun `compareAndSet persists on success`() {
    var persisted: String? = null
    val flow = PersistentStateFlow(initialValue = "a") { persisted = it }

    val result = flow.compareAndSet("a", "b")

    assertTrue(result)
    assertEquals("b", flow.value)
    assertEquals("b", persisted)
  }

  @Test
  fun `compareAndSet does not persist on failure`() {
    var persistCount = 0
    val flow = PersistentStateFlow(initialValue = "a") { persistCount++ }

    val result = flow.compareAndSet("wrong", "b")

    assertFalse(result)
    assertEquals("a", flow.value)
    assertEquals(0, persistCount)
  }

  @Test
  fun `emit updates value and persists`() = runTest {
    var persisted: Int? = null
    val flow = PersistentStateFlow(initialValue = 0) { persisted = it }

    flow.emit(42)

    assertEquals(42, flow.value)
    assertEquals(42, persisted)
  }

  @Test
  fun `tryEmit updates value and persists`() {
    var persisted: Int? = null
    val flow = PersistentStateFlow(initialValue = 0) { persisted = it }

    val result = flow.tryEmit(42)

    assertTrue(result)
    assertEquals(42, flow.value)
    assertEquals(42, persisted)
  }

  @Test
  fun `setting same value does not persist again`() {
    var persistCount = 0
    val flow = PersistentStateFlow(initialValue = "a") { persistCount++ }

    flow.value = "a"

    assertEquals(0, persistCount)
  }

  @Test
  fun `collect receives value changes`() = runTest {
    val flow = PersistentStateFlow(initialValue = 0) {}
    flow.value = 7
    assertEquals(7, flow.first())
  }
}
