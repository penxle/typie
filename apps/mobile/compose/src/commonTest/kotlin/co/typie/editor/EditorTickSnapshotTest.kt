package co.typie.editor

import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
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
