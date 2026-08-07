package co.typie.screen.editor.editor.subpane.comments

import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.scroll.toPageRectsTarget
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorCommentsSessionTest {
  private val rects =
    listOf(PageRect(pageIdx = 0, rect = Rect(x = 0f, y = 100f, width = 80f, height = 20f)))

  @Test
  fun `comment reveal waits for active range publication`() {
    val inactiveRange = commentRange(group = COMMENT_RANGE_GROUP)
    val activeRange = commentRange(group = ACTIVE_COMMENT_RANGE_GROUP)

    assertNull(listOf(inactiveRange).commentThreadScrollTarget(inactiveRange.id))
    assertEquals(
      rects.toPageRectsTarget(),
      listOf(activeRange).commentThreadScrollTarget(activeRange.id),
    )
  }

  private fun commentRange(group: String): TrackedRange =
    TrackedRange(
      id = "comment-1",
      group = group,
      anchor = Position(node = "text", offset = 0, affinity = Affinity.Downstream),
      head = Position(node = "text", offset = 4, affinity = Affinity.Downstream),
      metadata = "",
      rects = rects,
      text = "test",
    )
}
