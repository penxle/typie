package co.typie.editor.input

import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.test.runTest

class EditorImeNotificationFlowTest {
  private fun key(offset: Int, paused: Boolean = false): EditorImeNotifyKey {
    val position = Position(node = "n", offset = offset, affinity = Affinity.Downstream)
    return EditorImeNotifyKey(
      selection = Selection(anchor = position, head = position),
      cursor = null,
      ime = null,
      paused = paused,
    )
  }

  @Test
  fun `paused emissions are suppressed and resume delivers the trailing state once`() = runTest {
    val emitted =
      flowOf(
          key(0),
          key(1, paused = true),
          key(2, paused = true),
          key(2, paused = false),
        )
        .imeNotificationEvents()
        .toList()

    assertEquals(listOf(key(2, paused = false)), emitted)
  }

  @Test
  fun `unpaused changes emit after the initial state and duplicates are deduplicated`() = runTest {
    val emitted = flowOf(key(0), key(1), key(1), key(2)).imeNotificationEvents().toList()

    assertEquals(listOf(key(1), key(2)), emitted)
  }
}
