package co.typie.editor.sync.ws

import co.typie.editor.ffi.Editor
import co.typie.editor.ffi.GraphIngest
import co.typie.editor.ffi.Viewport
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertSame

private class FakeGraphIngest : GraphIngest {
  val appended = mutableListOf<ByteArray>()
  var abortCount = 0
    private set

  var finishCount = 0
    private set

  override fun appendChunk(data: ByteArray) {
    appended.add(data)
  }

  override fun totalBytes(): Long = appended.sumOf { it.size }.toLong()

  override fun abort() {
    abortCount += 1
  }

  override fun finish(viewport: Viewport): Editor {
    finishCount += 1
    return TODO("fake ingest does not build a real editor")
  }

  override fun finishWithPending(pendingEncoded: ByteArray, viewport: Viewport): Editor {
    finishCount += 1
    return TODO("fake ingest does not build a real editor")
  }
}

private fun chunk(bytes: ByteArray = byteArrayOf(1)) =
  AttachEvent.SnapshotChunkEvent(rowId = "r", seq = 0, offset = 0, bytes = bytes)

private fun end(seq: String = "0-1") =
  AttachEvent.SnapshotEndEvent(seq = seq, heads = ByteArray(0), durableHeads = ByteArray(0))

class DocumentGraphLoaderTest {
  @Test
  fun restart_aborts_old_generation_handle_and_begins_new_handle_exactly_once() {
    val handles = mutableListOf<FakeGraphIngest>()
    val loader = DocumentGraphLoader { FakeGraphIngest().also(handles::add) }

    assertNull(loader.handle(chunk()))
    val first = handles.single()
    assertEquals(0, first.abortCount)

    assertNull(loader.handle(AttachEvent.SnapshotRestart))

    assertEquals(2, handles.size)
    assertEquals(1, first.abortCount)
    val second = handles[1]
    assertEquals(0, second.abortCount)

    assertNull(loader.handle(chunk()))
    assertEquals(2, handles.size)
    assertEquals(1, second.appended.size)
  }

  @Test
  fun reload_after_transferred_does_not_abort_handle_and_enters_new_generation_on_next_chunk() {
    val handles = mutableListOf<FakeGraphIngest>()
    val loader = DocumentGraphLoader { FakeGraphIngest().also(handles::add) }

    assertNull(loader.handle(chunk()))
    val firstLoaded = loader.handle(end())
    assertIs<DocumentGraphLoaderEvent.Loaded>(firstLoaded)
    assertEquals(0, firstLoaded.generation)
    val firstHandle = handles.single()

    assertNull(loader.handle(AttachEvent.ReloadEvent))
    assertEquals(0, firstHandle.abortCount)

    assertNull(loader.handle(chunk()))
    assertEquals(2, handles.size)
    assertEquals(0, firstHandle.abortCount)

    val secondLoaded = loader.handle(end())
    assertIs<DocumentGraphLoaderEvent.Loaded>(secondLoaded)
    assertEquals(1, secondLoaded.generation)
    assertSame(handles[1], secondLoaded.handle)
    assertEquals(0, firstHandle.abortCount)
  }

  @Test
  fun duplicate_snapshot_end_for_same_generation_is_ignored() {
    val handles = mutableListOf<FakeGraphIngest>()
    val loader = DocumentGraphLoader { FakeGraphIngest().also(handles::add) }

    assertNull(loader.handle(chunk()))
    val firstEnd = loader.handle(end())
    assertIs<DocumentGraphLoaderEvent.Loaded>(firstEnd)

    val duplicateEnd = loader.handle(end())
    assertNull(duplicateEnd)
    assertEquals(1, handles.size)
    assertEquals(0, handles.single().abortCount)
  }

  @Test
  fun handle_is_delivered_exactly_once_ready_to_finish_after_end() {
    val handles = mutableListOf<FakeGraphIngest>()
    val loader = DocumentGraphLoader { FakeGraphIngest().also(handles::add) }

    assertNull(loader.handle(chunk(byteArrayOf(1, 2, 3))))
    val loaded = loader.handle(end())
    assertIs<DocumentGraphLoaderEvent.Loaded>(loaded)

    val handle = assertIs<FakeGraphIngest>(loaded.handle)
    assertSame(handles.single(), handle)
    assertEquals(0, handle.abortCount)
    assertEquals(0, handle.finishCount)

    // Further loader events for this now-transferred generation must not touch the handle again,
    // leaving it in a state where the caller can finish it exactly once.
    assertNull(loader.handle(end()))
    assertEquals(0, handle.abortCount)
    assertEquals(0, handle.finishCount)
  }

  @Test
  fun restart_after_transferred_is_ignored() {
    val handles = mutableListOf<FakeGraphIngest>()
    val loader = DocumentGraphLoader { FakeGraphIngest().also(handles::add) }

    assertNull(loader.handle(chunk()))
    val loaded = loader.handle(end())
    assertIs<DocumentGraphLoaderEvent.Loaded>(loaded)
    val handle = handles.single()

    assertNull(loader.handle(AttachEvent.SnapshotRestart))
    assertEquals(1, handles.size)
    assertEquals(0, handle.abortCount)

    val duplicateEnd = loader.handle(end())
    assertNull(duplicateEnd)
    assertEquals(1, handles.size)
    assertEquals(0, handle.abortCount)
  }

  @Test
  fun permanent_error_while_receiving_aborts_handle_once_and_emits_failed() {
    val handles = mutableListOf<FakeGraphIngest>()
    val loader = DocumentGraphLoader { FakeGraphIngest().also(handles::add) }

    assertNull(loader.handle(chunk()))
    val handle = handles.single()

    val result = loader.handle(AttachEvent.PermanentErrorEvent("boom"))
    val failed = assertIs<DocumentGraphLoaderEvent.Failed>(result)
    assertEquals("boom", failed.code)
    assertEquals(1, handle.abortCount)
  }

  @Test
  fun cancel_while_receiving_aborts_handle_once() {
    val handles = mutableListOf<FakeGraphIngest>()
    val loader = DocumentGraphLoader { FakeGraphIngest().also(handles::add) }

    assertNull(loader.handle(chunk()))
    val handle = handles.single()

    loader.cancel()

    assertEquals(1, handle.abortCount)
  }

  @Test
  fun cancel_after_transferred_does_not_abort_handle() {
    val handles = mutableListOf<FakeGraphIngest>()
    val loader = DocumentGraphLoader { FakeGraphIngest().also(handles::add) }

    assertNull(loader.handle(chunk()))
    val loaded = loader.handle(end())
    assertIs<DocumentGraphLoaderEvent.Loaded>(loaded)
    val handle = handles.single()

    loader.cancel()

    assertEquals(0, handle.abortCount)
  }
}
