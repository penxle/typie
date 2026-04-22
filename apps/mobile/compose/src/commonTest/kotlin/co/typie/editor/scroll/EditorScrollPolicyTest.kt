package co.typie.editor.scroll

import androidx.compose.ui.geometry.Size
import co.typie.editor.VerticalSpan
import kotlin.test.Test
import kotlin.test.assertEquals

private const val FloatTolerance = 0.01f

class EditorScrollPolicyTest {
  @Test
  fun `keep-visible policy scrolls down when cursor enters the lower scroll margin`() {
    val target =
      resolveKeepVisibleScrollTarget(
        currentScroll = 400f,
        cursorTopInContent = 1112f,
        cursorBottomInContent = 1144f,
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
            viewport = Size(width = 720f, height = 900f),
            topInset = 120f,
            imeInset = 100f,
          ),
        baseBottomSpace = 20f,
        typewriterEnabled = false,
        typewriterPosition = 0.5f,
        cursorHeight = 20f,
      )

    assertEquals(EditorScrollMode.KeepCursorVisible, policy.mode)
    assertEquals(0.5f, policy.typewriterPosition, FloatTolerance)
    assertEquals(VerticalSpan(top = 180f, bottom = 740f), policy.keepVisibleRange)
    assertEquals(450f, requireNotNull(policy.typewriterTargetTop), FloatTolerance)
    assertEquals(470f, requireNotNull(policy.typewriterTargetBottom), FloatTolerance)
    assertEquals(140f, policy.bottomSpacerHeight, FloatTolerance)
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

    assertEquals(644f, requireNotNull(target), FloatTolerance)
  }

  @Test
  fun `resolved policy switches to typewriter mode when enabled`() {
    val policy =
      resolveEditorScrollPolicy(
        visibleArea = testVisibleArea(),
        baseBottomSpace = 20f,
        typewriterEnabled = true,
        typewriterPosition = 0.25f,
        cursorHeight = 32f,
      )

    assertEquals(EditorScrollMode.Typewriter, policy.mode)
    assertEquals(0.25f, policy.typewriterPosition, FloatTolerance)
    assertEquals(252f, requireNotNull(policy.typewriterTargetTop), FloatTolerance)
    assertEquals(284f, requireNotNull(policy.typewriterTargetBottom), FloatTolerance)
    assertEquals(596f, policy.bottomSpacerHeight, FloatTolerance)
  }

  @Test
  fun `typewriter bottom padding can use actual space available below the cursor top`() {
    val policy =
      resolveEditorScrollPolicy(
        visibleArea = testVisibleArea(),
        baseBottomSpace = 20f,
        distanceToPagesBottom = 520f,
        typewriterEnabled = true,
        typewriterPosition = 0.25f,
        cursorHeight = 32f,
      )

    assertEquals(EditorScrollMode.Typewriter, policy.mode)
    assertEquals(128f, policy.bottomSpacerHeight, FloatTolerance)
  }

  @Test
  fun `keep-visible bottom padding respects paginated intrinsic bottom space`() {
    val policy =
      resolveEditorScrollPolicy(
        visibleArea =
          EditorVisibleArea(viewport = Size(width = 720f, height = 900f), topInset = 120f),
        baseBottomSpace = 40f,
      )

    assertEquals(EditorScrollMode.KeepCursorVisible, policy.mode)
    assertEquals(20f, policy.bottomSpacerHeight, FloatTolerance)
  }

  @Test
  fun `page edge reveal bottom padding can exceed cursor policy padding`() {
    val policy =
      resolveEditorScrollPolicy(
        visibleArea = testVisibleArea(),
        baseBottomSpace = 180f,
        pageBottomRevealSpacerHeight = 100f,
      )

    assertEquals(EditorScrollMode.KeepCursorVisible, policy.mode)
    assertEquals(100f, policy.bottomSpacerHeight, FloatTolerance)
  }

  private fun testVisibleArea(): EditorVisibleArea =
    EditorVisibleArea(viewport = Size(width = 720f, height = 900f), topInset = 80f, imeInset = 100f)
}
