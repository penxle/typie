package co.typie.screen.editor.editor.subpane.comments

import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.scroll.EditorBringIntoViewTarget
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class EditorCommentsSessionTest {
  private val rects =
    listOf(PageRect(pageIdx = 0, rect = Rect(x = 0f, y = 100f, width = 80f, height = 20f)))

  @Test
  fun `comment reveal waits for active range publication`() {
    val inactiveRange = commentRange(group = COMMENT_RANGE_GROUP)
    val activeRange = commentRange(group = ACTIVE_COMMENT_RANGE_GROUP)

    assertNull(listOf(inactiveRange).commentThreadScrollTarget(inactiveRange.id))
    assertEquals(
      EditorBringIntoViewTarget.TrackedItem(activeRange.id),
      listOf(activeRange).commentThreadScrollTarget(activeRange.id),
    )
    assertEquals(
      EditorBringIntoViewTarget.TrackedItem(activeRange.id),
      listOf(activeRange.copy(rects = emptyList())).commentThreadScrollTarget(activeRange.id),
    )
  }

  @Test
  fun `activating the same comment thread again records a new reveal intent`() {
    val state = CommentThreadState()

    state.activateThread("comment-1")
    val firstActivation = state.activationRevision
    state.activateThread("comment-1")

    assertTrue(state.activationRevision > firstActivation)
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
