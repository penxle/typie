package co.typie.editor

import androidx.compose.ui.text.TextRange
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Revision
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.runTest

class EditorImeGeometryTest {
  @Test
  fun geometryUsesAppliedImeOffsetsBeforeTheFrameIsPublished() = runTest {
    val scope = backgroundScope
    val rect = PageRect(0, Rect(20f, 40f, 60f, 28f))
    var ime = Ime("にほん", 10, ImeRange(13, 13), ImeRange(10, 13))
    val calls = mutableListOf<Triple<Revision, Int, Int>>()
    val fake =
      FakeFfiEditor(
        imeProvider = { _, _ -> ime },
        firstRectForRangeProvider = { revision, start, end ->
          calls += Triple(revision, start, end)
          rect
        },
      )
    val editor = Editor(fake, scope)
    try {
      editor.setImeSessionActive(true)
      fake.applySnapshot(editor)
      assertNull(editor.publishedBundle)
      assertEquals(rect, editor.firstRectForRange(TextRange(0, 3)))
      assertEquals(Triple(Revision(editor.appliedState.version), 10, 13), calls.last())

      // A new window and a surrogate pair must use the latest applied UTF-16 mapping.
      ime = Ime("😀日本語", 20, ImeRange(24, 24), ImeRange(21, 24))
      fake.applySnapshot(editor)
      assertEquals(rect, editor.firstRectForRange(TextRange(2, 5)))
      assertEquals(Triple(Revision(editor.appliedState.version), 21, 24), calls.last())
      assertNull(editor.firstRectForRange(TextRange(0, 6)))
      assertEquals(2, calls.size)
    } finally {
      scope.cancel()
    }
  }
}
