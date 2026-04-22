package co.typie.screen.editor.editor.scroll

import co.typie.screen.editor.editor.layout.EditorMeasuredSize
import co.typie.screen.editor.editor.layout.EditorVisibleArea
import kotlin.test.Test
import kotlin.test.assertEquals

private const val FloatTolerance = 0.01f

class EditorScrollPolicyTest {
  @Test
  fun `keep-visible policy scrolls down when cursor enters the lower scroll margin`() {
    val target =
      resolveKeepVisibleScrollTarget(
        currentScroll = 400f,
        cursorTopInContent = 1068f,
        cursorBottomInContent = 1100f,
        visibleArea = testVisibleArea(),
      )

    assertEquals(404f, target)
  }

  @Test
  fun `keep-visible policy does not scroll up before the cursor enters the visible editor margin`() {
    val target =
      resolveKeepVisibleScrollTarget(
        currentScroll = 240f,
        cursorTopInContent = 420f,
        cursorBottomInContent = 448f,
        visibleArea = testVisibleArea(),
      )

    assertEquals(null, target)
  }

  @Test
  fun `keep-visible policy scrolls up only after the cursor enters the visible viewport guard`() {
    val target =
      resolveKeepVisibleScrollTarget(
        currentScroll = 240f,
        cursorTopInContent = 379f,
        cursorBottomInContent = 407f,
        visibleArea = testVisibleArea(),
      )

    assertEquals(239f, target)
  }

  @Test
  fun `resolved policy keeps typewriter padding separate from active keep-visible mode`() {
    val policy =
      resolveEditorScrollPolicy(
        visibleArea =
          EditorVisibleArea(
            viewport = EditorMeasuredSize(width = 720f, height = 900f),
            topInset = 120f,
            imeInset = 100f,
            toolbarTop = 756f,
          ),
        defaultBottomPadding = 168f,
      )

    assertEquals(EditorScrollMode.KeepCursorVisible, policy.mode)
    assertEquals(EditorScrollRange(top = 180f, bottom = 696f), policy.keepVisibleRange)
    assertEquals(368.72f, policy.typewriterRange.top, FloatTolerance)
    assertEquals(424.72f, policy.typewriterRange.bottom, FloatTolerance)
    assertEquals(299.28f, policy.typewriterBottomPadding, FloatTolerance)
  }

  private fun testVisibleArea(): EditorVisibleArea =
    EditorVisibleArea(
      viewport = EditorMeasuredSize(width = 720f, height = 900f),
      topInset = 80f,
      imeInset = 100f,
      toolbarTop = 756f,
    )
}
