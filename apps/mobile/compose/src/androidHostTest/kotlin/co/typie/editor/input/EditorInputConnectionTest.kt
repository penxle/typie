package co.typie.editor.input

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Message
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.sync.createTestDocumentEditingSession
import co.typie.platform.NoopClipboard
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

class EditorInputConnectionTest {
  @Test
  fun `detached input node rejects a pending batch from its old connection`() {
    val fixture = InputNodeFixture()
    val dispatched = mutableListOf<List<Message>>()
    val batch = fixture.batchFromCurrentConnection(dispatched)

    try {
      batch.beginBatchEdit()
      batch.enqueue(FlatImeOp.Compose("한"))

      fixture.node.onDetach()
      batch.endBatchEdit()

      assertTrue(dispatched.isEmpty())
    } finally {
      fixture.close()
    }
  }

  @Test
  fun `input policy restart rejects a pending batch from its old connection`() {
    val fixture = InputNodeFixture()
    val dispatched = mutableListOf<List<Message>>()
    val batch = fixture.batchFromCurrentConnection(dispatched)

    try {
      batch.beginBatchEdit()
      batch.enqueue(FlatImeOp.Compose("한"))

      fixture.node.updateInputPolicy(enabled = false, suppressSoftwareKeyboard = false)
      batch.endBatchEdit()

      assertTrue(dispatched.isEmpty())
    } finally {
      fixture.close()
    }
  }

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

  private class InputNodeFixture {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    private val editor = Editor(FakeFfiEditor(), scope)
    private val session = createTestDocumentEditingSession(editor, scope)
    val node =
      EditorInputNode(
        session = session,
        uiState = EditorUiState(),
        platform = Platform.Android,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        enabled = true,
        suppressSoftwareKeyboard = false,
        clipboard = NoopClipboard,
        incomingContentHandler = NoopEditorIncomingContentHandler,
      )
    private val generationField =
      EditorInputNode::class.java.getDeclaredField("imeSessionGeneration").apply {
        isAccessible = true
      }

    fun batchFromCurrentConnection(dispatched: MutableList<List<Message>>): ImeEditBatch {
      val generationAtStart = generationField.getInt(node)
      return ImeEditBatch(
        isSessionCurrent = { generationField.getInt(node) == generationAtStart },
        dispatch = { dispatched += it },
      )
    }

    fun close() {
      session.stop()
      scope.cancel()
    }
  }
}
