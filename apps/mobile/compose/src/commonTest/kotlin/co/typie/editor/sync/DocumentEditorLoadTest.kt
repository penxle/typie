package co.typie.editor.sync

import co.typie.editor.ffi.Editor as FfiEditor
import co.typie.editor.ffi.GraphIngest
import co.typie.editor.ffi.ThemeVariant
import co.typie.editor.ffi.Viewport
import co.typie.editor.sync.ws.DocumentSyncBaseline
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.test.runTest

private class ExpectedFinishFailure : RuntimeException()

private class FailingGraphIngest : GraphIngest {
  var abortCount = 0
    private set

  var finishCount = 0
    private set

  override fun appendChunk(data: ByteArray) = Unit

  override fun totalBytes(): Long = 0

  override fun abort() {
    abortCount += 1
  }

  override fun finish(viewport: Viewport): FfiEditor = error("unexpected finish")

  override fun finishWithPending(pendingEncoded: ByteArray, viewport: Viewport): FfiEditor {
    finishCount += 1
    throw ExpectedFinishFailure()
  }
}

class DocumentEditorLoadTest {
  @Test
  fun closingLoadCancelsReadyEditorWaiter() = runTest {
    val load =
      DocumentEditorLoad(
        ingest = FailingGraphIngest(),
        initialBaseline =
          DocumentSyncBaseline(seq = "1", heads = ByteArray(0), durableHeads = ByteArray(0)),
        pending = emptyList(),
        parentScope = this,
        onEditorError = { _, _ -> },
      )
    val ready = async(start = CoroutineStart.UNDISPATCHED) { load.awaitReadyEditor() }

    assertFalse(ready.isCompleted)

    load.close()

    assertFailsWith<CancellationException> { ready.await() }
  }

  @Test
  fun repeatedAwaitMemoizesTerminalFailureWithoutRefinishingIngest() = runTest {
    val ingest = FailingGraphIngest()
    val load =
      DocumentEditorLoad(
        ingest = ingest,
        initialBaseline =
          DocumentSyncBaseline(seq = "1", heads = ByteArray(0), durableHeads = ByteArray(0)),
        pending = emptyList(),
        parentScope = this,
        onEditorError = { _, _ -> },
      )
    val viewport = Viewport(width = 100f, height = 100f, scaleFactor = 1.0)

    try {
      assertFailsWith<ExpectedFinishFailure> {
        load.awaitEditor(viewport = viewport, themeVariant = ThemeVariant.LightWhite)
      }
      assertFailsWith<ExpectedFinishFailure> {
        load.awaitEditor(viewport = viewport, themeVariant = ThemeVariant.LightWhite)
      }

      assertEquals(1, ingest.finishCount)
      assertEquals(0, ingest.abortCount)
    } finally {
      load.close()
    }
  }
}
