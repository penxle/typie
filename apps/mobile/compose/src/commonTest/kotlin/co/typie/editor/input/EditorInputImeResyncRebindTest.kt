package co.typie.editor.input

import co.typie.editor.Editor
import co.typie.editor.EditorEventListener
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.EditorEvent
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.cancel

@OptIn(ExperimentalAtomicApi::class)
class EditorInputImeResyncRebindTest {
  @Test
  fun `rebinding subscription on session swap unsubscribes the previous editor`() {
    val scope = CoroutineScope(Dispatchers.Unconfined)
    val editor1 = Editor(FakeFfiEditor(), scope)
    val editor2 = Editor(FakeFfiEditor(), scope)
    val key = EditorEvent.ImeResyncRequired::class
    val listener: EditorEventListener<EditorEvent.ImeResyncRequired> = { _, _ -> }

    val unsubscribe1 = rebindImeResync(previous = null, target = editor1, listener = listener)
    assertEquals(1, editor1.listeners.load()[key]?.size)

    val unsubscribe2 =
      rebindImeResync(previous = unsubscribe1, target = editor2, listener = listener)
    assertTrue(editor1.listeners.load()[key].isNullOrEmpty())
    assertEquals(1, editor2.listeners.load()[key]?.size)

    unsubscribe2()
    assertTrue(editor2.listeners.load()[key].isNullOrEmpty())

    editor1.dispose()
    editor2.dispose()
    scope.cancel()
  }
}
