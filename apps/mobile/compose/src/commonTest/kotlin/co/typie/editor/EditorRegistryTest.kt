package co.typie.editor

import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Size
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.test.runTest

private class StubFfiEditor : co.typie.editor.ffi.Editor {
  override fun enqueue(message: Message) = Unit

  override fun tick(): List<EditorEvent> = emptyList()

  override fun attachSurface(
    page: Int,
    handle: Long,
    width: Double,
    height: Double,
    scaleFactor: Double,
  ) = error("not used")

  override fun detachSurface(page: Int) = error("not used")

  override fun resizeSurface(page: Int, width: Double, height: Double, scaleFactor: Double) =
    error("not used")

  override fun renderSurface(page: Int) = error("not used")

  override fun cursor(): CursorMetrics? = null

  override fun selection(): Selection = error("not used")

  override fun pageSizes(): List<Size> = emptyList()

  override fun ime(beforeLimit: Int, afterLimit: Int): Ime = error("not used")

  override fun inspectState(options: InspectStateOptions?): String = ""

  override fun inspectStateAsMacro(): String = ""
}

class EditorRegistryTest {
  private fun makeEditor(): Editor = Editor(StubFfiEditor(), CoroutineScope(Dispatchers.Unconfined))

  @Test
  fun registered_editor_appears_in_snapshot() = runTest {
    val editor = makeEditor()
    EditorRegistry.register(editor)
    try {
      assertTrue(EditorRegistry.snapshot().contains(editor))
    } finally {
      EditorRegistry.unregister(editor)
    }
  }

  @Test
  fun unregistered_editor_leaves_snapshot() = runTest {
    val editor = makeEditor()
    EditorRegistry.register(editor)
    EditorRegistry.unregister(editor)

    assertFalse(EditorRegistry.snapshot().contains(editor))
  }

  @Test
  fun concurrent_register_unregister_is_consistent() = runTest {
    val editors = List(20) { makeEditor() }
    val jobs = editors.map { e ->
      async(Dispatchers.Default) {
        EditorRegistry.register(e)
        EditorRegistry.unregister(e)
      }
    }
    jobs.awaitAll()

    val snap = EditorRegistry.snapshot()
    for (e in editors) {
      assertFalse(snap.contains(e), "editor $e unexpectedly in snapshot")
    }
  }
}
