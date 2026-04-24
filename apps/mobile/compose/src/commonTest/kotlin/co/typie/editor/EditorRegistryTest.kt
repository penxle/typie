package co.typie.editor

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.test.runTest

class EditorRegistryTest {
  private fun makeEditor(): Editor = Editor(FakeFfiEditor(), CoroutineScope(Dispatchers.Unconfined))

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
