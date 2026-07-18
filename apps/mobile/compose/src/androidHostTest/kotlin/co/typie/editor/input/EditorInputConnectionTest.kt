package co.typie.editor.input

import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Message
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class EditorInputConnectionTest {
  @Test
  fun `stale connection drops pending batch instead of flushing on close`() {
    val dispatched = mutableListOf<List<Message>>()
    val batch = ImeEditBatch(isSessionCurrent = { false }) { dispatched += it }

    batch.beginBatchEdit()
    batch.enqueue(FlatImeOp.Compose("한"))
    batch.closeConnection(hasActiveComposition = true)

    assertTrue(dispatched.isEmpty())
  }

  @Test
  fun `current connection still flushes on close`() {
    val dispatched = mutableListOf<List<Message>>()
    val batch = ImeEditBatch(isSessionCurrent = { true }) { dispatched += it }

    batch.beginBatchEdit()
    batch.enqueue(FlatImeOp.Compose("한"))
    batch.closeConnection(hasActiveComposition = true)

    assertEquals(1, dispatched.size)
    val ops = (dispatched.single().single() as Message.TextInput).ops
    assertTrue(ops.contains(FlatImeOp.CommitAsIs))
  }
}
