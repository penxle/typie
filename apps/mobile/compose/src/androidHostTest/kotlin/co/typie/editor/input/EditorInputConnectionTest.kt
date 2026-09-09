package co.typie.editor.input

import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.SetComposingTextCommand
import androidx.compose.ui.text.input.SetSelectionCommand
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
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
  fun `composing reach follows text inserted earlier in the batch`() {
    val ime =
      Ime(text = "\u2028\u2029", windowStart = 0, selection = ImeRange(1, 1), composing = null)
    val dispatched = mutableListOf<List<Message>>()
    val batch = ImeEditBatch(isSessionCurrent = { true }, currentIme = { ime }) { dispatched += it }
    val text = "a".repeat(100)
    batch.beginBatchEdit()
    batch.enqueue(CommitTextCommand(text, 1))
    batch.setComposingRegion(100, 101)
    batch.endBatchEdit()
    assertEquals<List<List<Message>>>(
      listOf(
        listOf(
          Message.TextInput(
            listOf(FlatImeOp.ReplaceSelection(text), FlatImeOp.SetComposition(100, 101))
          )
        )
      ),
      dispatched,
    )
  }

  @Test
  fun `selection conversion sees earlier edits in the Android batch`() {
    val ime =
      Ime(text = "\u2028\u2029", windowStart = 0, selection = ImeRange(1, 1), composing = null)
    val dispatched = mutableListOf<List<Message>>()
    val batch = ImeEditBatch(isSessionCurrent = { true }, currentIme = { ime }) { dispatched += it }
    batch.beginBatchEdit()
    batch.enqueue(CommitTextCommand("😀", 1))
    batch.enqueue(SetSelectionCommand(1, 3))
    batch.enqueue(CommitTextCommand("X", 1))
    assertTrue(dispatched.isEmpty())
    batch.endBatchEdit()
    assertEquals<List<List<Message>>>(
      listOf(
        listOf(
          Message.TextInput(
            listOf(
              FlatImeOp.ReplaceSelection("😀"),
              FlatImeOp.SetSelection(1, 2),
              FlatImeOp.ReplaceSelection("X"),
            )
          )
        )
      ),
      dispatched,
    )
  }

  @Test
  fun `detached input node rejects a pending batch from its old connection`() {
    val fixture = InputNodeFixture()
    val dispatched = mutableListOf<List<Message>>()
    val batch = fixture.batchFromCurrentConnection(dispatched)

    try {
      batch.beginBatchEdit()
      batch.enqueue(SetComposingTextCommand("한", 1))

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
      batch.enqueue(SetComposingTextCommand("한", 1))

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
    val batch =
      ImeEditBatch(isSessionCurrent = { false }, currentIme = { null }) { dispatched += it }

    batch.beginBatchEdit()
    batch.enqueue(SetComposingTextCommand("한", 1))
    batch.closeConnection()

    assertTrue(dispatched.isEmpty())
  }

  @Test
  fun `current connection still flushes on close`() {
    val dispatched = mutableListOf<List<Message>>()
    val batch =
      ImeEditBatch(isSessionCurrent = { true }, currentIme = { null }) { dispatched += it }

    batch.beginBatchEdit()
    batch.enqueue(SetComposingTextCommand("한", 1))
    batch.closeConnection()

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
        currentIme = { null },
        dispatch = { dispatched += it },
      )
    }

    fun close() {
      session.stop()
      scope.cancel()
    }
  }
}
