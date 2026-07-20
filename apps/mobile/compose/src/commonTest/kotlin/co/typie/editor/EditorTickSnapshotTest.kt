package co.typie.editor

import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PlaceholderMetrics
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain

class EditorTickSnapshotTest {
  @Test
  fun tickViewStaysFreshWhileStateAwaitsRenderSettlement() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      var ime: Ime? = null
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          imeProvider = { _, _ -> ime },
        )
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.setImeSessionActive(true)
      editor.attachSurface(page = 0, handle = 1L, width = 100.0, height = 100.0, scaleFactor = 1.0)
      runCurrent()

      ime = Ime(text = "hello", windowStart = 0, selection = ImeRange(5, 5), composing = null)
      val edit = launch { editor.await { enqueue(Message.System(SystemEvent.Initialize)) } }
      runCurrent()

      assertEquals("hello", editor.tickIme?.text)
      assertEquals(0L, editor.state.version)
      assertNull(editor.ime)

      editor.dispose()
      runCurrent()

      assertEquals("hello", editor.tickIme?.text)
      assertEquals(0L, editor.state.version)
      assertTrue(edit.isCompleted)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun interleavedEnqueueTickPreservesSettleDelayedDocumentRevision() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      var events: List<EditorEvent> = emptyList()
      val fake = FakeFfiEditor(onTick = { events })
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.attachSurface(page = 0, handle = 1L, width = 100.0, height = 100.0, scaleFactor = 1.0)
      runCurrent()

      // A document edit whose commit is held back by the surface settle barrier.
      events =
        listOf(
          EditorEvent.StateChanged(fields = listOf(StateField.Doc)),
          EditorEvent.RenderInvalidated,
        )
      val edit = launch { editor.await { enqueue(Message.System(SystemEvent.Initialize)) } }
      runCurrent()
      assertEquals(0L, editor.state.documentRevision)

      // An enqueue-path tick interleaves while the edit is still settle-parked.
      events = emptyList()
      editor.enqueue(Message.System(SystemEvent.Initialize))
      runCurrent()

      editor.onPageSettled(page = 0, version = Long.MAX_VALUE)
      runCurrent()

      assertTrue(edit.isCompleted)
      assertEquals(
        1L,
        editor.state.documentRevision,
        "a settle-delayed document revision must survive an interleaved tick",
      )
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun interleavedEnqueueTickPreservesSettleDelayedPlaceholder() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      var events: List<EditorEvent> = emptyList()
      var placeholder: PlaceholderMetrics? = null
      val fake = FakeFfiEditor(onTick = { events }, placeholderProvider = { placeholder })
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.attachSurface(page = 0, handle = 1L, width = 100.0, height = 100.0, scaleFactor = 1.0)
      runCurrent()
      // Committed baseline so the version-0 recompute fallback is out of play.
      editor.await { enqueue(Message.System(SystemEvent.Initialize)) }

      placeholder =
        PlaceholderMetrics(
          pageIdx = 0,
          rect = Rect(x = 1f, y = 2f, width = 3f, height = 4f),
          fontSize = null,
          lineHeight = null,
          letterSpacing = null,
          align = null,
        )
      events =
        listOf(
          EditorEvent.StateChanged(fields = listOf(StateField.Placeholder)),
          EditorEvent.RenderInvalidated,
        )
      val edit = launch { editor.await { enqueue(Message.System(SystemEvent.Initialize)) } }
      runCurrent()

      events = emptyList()
      editor.enqueue(Message.System(SystemEvent.Initialize))
      runCurrent()

      editor.onPageSettled(page = 0, version = Long.MAX_VALUE)
      runCurrent()

      assertTrue(edit.isCompleted)
      assertEquals(
        placeholder,
        editor.state.placeholder,
        "a settle-delayed placeholder must survive an interleaved tick",
      )
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun tickViewMatchesStateWhenCommitIsImmediate() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      var ime: Ime? = null
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          imeProvider = { _, _ -> ime },
        )
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.setImeSessionActive(true)

      ime = Ime(text = "hello", windowStart = 0, selection = ImeRange(5, 5), composing = null)
      editor.await { enqueue(Message.System(SystemEvent.Initialize)) }

      assertEquals("hello", editor.state.ime?.text)
      assertEquals(editor.state.ime, editor.tickIme)
      assertEquals(1L, editor.state.version)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }
}
