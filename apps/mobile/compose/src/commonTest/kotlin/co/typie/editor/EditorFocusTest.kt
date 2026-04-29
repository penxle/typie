package co.typie.editor

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers

class EditorFocusTest {
  @Test
  fun focus_before_focus_target_is_attached_is_ignored() {
    val editor = Editor(FakeFfiEditor(), CoroutineScope(Dispatchers.Unconfined))

    assertEquals(false, editor.focus())

    editor.dispose()
  }

  @Test
  fun focus_after_dispose_is_ignored() {
    val editor = Editor(FakeFfiEditor(), CoroutineScope(Dispatchers.Unconfined))
    editor.dispose()

    assertEquals(false, editor.focus())
  }
}
