package co.typie.editor

import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain

class EditorTickSnapshotTest {
  @Test
  fun appliedSnapshotContainsMaterializedIme() = runTest {
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
      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }

      assertEquals("hello", editor.appliedState.ime?.text)
      assertEquals(1L, editor.appliedState.version)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }
}
