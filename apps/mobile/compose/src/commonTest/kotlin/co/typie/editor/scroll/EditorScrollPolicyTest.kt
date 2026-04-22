package co.typie.editor.scroll

import co.typie.editor.body.EditorMeasuredSize
import co.typie.editor.body.EditorVisibleArea
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
  fun `resolved policy keeps keep-visible mode active when typewriter is disabled`() {
    val policy =
      resolveEditorScrollPolicy(
        visibleArea =
          EditorVisibleArea(
            viewport = EditorMeasuredSize(width = 720f, height = 900f),
            topInset = 120f,
            imeInset = 100f,
            toolbarTop = 756f,
          ),
        intrinsicBottomSpace = 20f,
        typewriterEnabled = false,
        typewriterPosition = 0.5f,
        cursorHeight = 20f,
      )

    assertEquals(EditorScrollMode.KeepCursorVisible, policy.mode)
    assertEquals(0.5f, policy.typewriterPosition, FloatTolerance)
    assertEquals(EditorScrollRange(top = 180f, bottom = 696f), policy.keepVisibleRange)
    assertEquals(428f, requireNotNull(policy.typewriterTargetTop), FloatTolerance)
    assertEquals(448f, requireNotNull(policy.typewriterTargetBottom), FloatTolerance)
    assertEquals(432f, policy.typewriterBottomPadding, FloatTolerance)
  }

  @Test
  fun `typewriter policy scrolls cursor top to the configured viewport position`() {
    val target =
      resolveTypewriterScrollTarget(
        currentScroll = 400f,
        cursorTopInContent = 1068f,
        cursorBottomInContent = 1100f,
        visibleArea = testVisibleArea(),
        position = 0.5f,
      )

    assertEquals(666f, requireNotNull(target), FloatTolerance)
  }

  @Test
  fun `resolved policy switches to typewriter mode when enabled`() {
    val policy =
      resolveEditorScrollPolicy(
        visibleArea = testVisibleArea(),
        intrinsicBottomSpace = 20f,
        typewriterEnabled = true,
        typewriterPosition = 0.25f,
        cursorHeight = 32f,
      )

    assertEquals(EditorScrollMode.Typewriter, policy.mode)
    assertEquals(0.25f, policy.typewriterPosition, FloatTolerance)
    assertEquals(241f, requireNotNull(policy.typewriterTargetTop), FloatTolerance)
    assertEquals(273f, requireNotNull(policy.typewriterTargetBottom), FloatTolerance)
  }

  private fun testVisibleArea(): EditorVisibleArea =
    EditorVisibleArea(
      viewport = EditorMeasuredSize(width = 720f, height = 900f),
      topInset = 80f,
      imeInset = 100f,
      toolbarTop = 756f,
    )
}
