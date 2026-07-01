package co.typie.editor.scroll

import androidx.compose.ui.geometry.Size
import co.typie.editor.VerticalSpan
import kotlin.test.Test
import kotlin.test.assertEquals

private const val FloatTolerance = 0.01f

class EditorAutoScrollPolicyTest {
  @Test
  fun `keep-visible policy scrolls down when cursor enters the lower scroll margin`() {
    val offset =
      resolveKeepVisibleScrollOffset(
        currentScroll = 400f,
        targetTopInContent = 1112f,
        targetBottomInContent = 1144f,
        visibleArea = testVisibleArea(),
      )

    assertEquals(404f, offset)
  }

  @Test
  fun `keep-visible policy does not scroll up before the cursor enters the visible editor margin`() {
    val offset =
      resolveKeepVisibleScrollOffset(
        currentScroll = 240f,
        targetTopInContent = 420f,
        targetBottomInContent = 448f,
        visibleArea = testVisibleArea(),
      )

    assertEquals(null, offset)
  }

  @Test
  fun `keep-visible policy scrolls up only after the cursor enters the visible viewport guard`() {
    val offset =
      resolveKeepVisibleScrollOffset(
        currentScroll = 240f,
        targetTopInContent = 379f,
        targetBottomInContent = 407f,
        visibleArea = testVisibleArea(),
      )

    assertEquals(239f, offset)
  }

  @Test
  fun `keep-visible policy centers target when visible area is too narrow for guard margins`() {
    val offset =
      resolveKeepVisibleScrollOffset(
        currentScroll = 300f,
        targetTopInContent = 500f,
        targetBottomInContent = 520f,
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 720f, height = 300f),
            topInset = 140f,
            bottomOcclusionInset = 110f,
          ),
      )

    assertEquals(345f, offset)
  }

  @Test
  fun `resolved policy keeps keep-visible mode active when typewriter is disabled`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 720f, height = 900f),
            topInset = 120f,
            imeInset = 100f,
          ),
        baseBottomSpace = 20f,
        typewriterEnabled = false,
        typewriterPosition = 0.5f,
        targetLineHeight = 20f,
      )

    assertEquals(EditorAutoScrollMode.KeepCursorVisible, policy.mode)
    assertEquals(0.5f, policy.typewriterPosition, FloatTolerance)
    assertEquals(VerticalSpan(top = 180f, bottom = 740f), policy.keepVisibleRange)
    assertEquals(450f, requireNotNull(policy.targetTop), FloatTolerance)
    assertEquals(470f, requireNotNull(policy.targetBottom), FloatTolerance)
    assertEquals(140f, policy.bottomSpacerHeight, FloatTolerance)
  }

  @Test
  fun `typewriter policy scrolls target top to the configured viewport position`() {
    val offset =
      resolveTypewriterScrollOffset(
        currentScroll = 400f,
        targetTopInContent = 1068f,
        targetBottomInContent = 1100f,
        visibleArea = testVisibleArea(),
        position = 0.5f,
      )

    assertEquals(644f, requireNotNull(offset), FloatTolerance)
  }

  @Test
  fun `resolved policy switches to typewriter mode when enabled`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea = testVisibleArea(),
        baseBottomSpace = 20f,
        typewriterEnabled = true,
        typewriterPosition = 0.25f,
        targetLineHeight = 32f,
      )

    assertEquals(EditorAutoScrollMode.Typewriter, policy.mode)
    assertEquals(0.25f, policy.typewriterPosition, FloatTolerance)
    assertEquals(252f, requireNotNull(policy.targetTop), FloatTolerance)
    assertEquals(284f, requireNotNull(policy.targetBottom), FloatTolerance)
    assertEquals(596f, policy.bottomSpacerHeight, FloatTolerance)
  }

  @Test
  fun `typewriter bottom padding can use actual space available below the cursor line top`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea = testVisibleArea(),
        baseBottomSpace = 20f,
        distanceToPagesBottom = 520f,
        typewriterEnabled = true,
        typewriterPosition = 0.25f,
        targetLineHeight = 32f,
      )

    assertEquals(EditorAutoScrollMode.Typewriter, policy.mode)
    assertEquals(128f, policy.bottomSpacerHeight, FloatTolerance)
  }

  @Test
  fun `keep-visible bottom padding respects paginated intrinsic bottom space`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea =
          EditorVisibleArea(viewport = Size(width = 720f, height = 900f), topInset = 120f),
        baseBottomSpace = 40f,
      )

    assertEquals(EditorAutoScrollMode.KeepCursorVisible, policy.mode)
    assertEquals(20f, policy.bottomSpacerHeight, FloatTolerance)
  }

  @Test
  fun `page edge reveal bottom padding can exceed cursor policy padding`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea = testVisibleArea(),
        baseBottomSpace = 180f,
        pageBottomRevealSpacerHeight = 100f,
      )

    assertEquals(EditorAutoScrollMode.KeepCursorVisible, policy.mode)
    assertEquals(100f, policy.bottomSpacerHeight, FloatTolerance)
  }

  @Test
  fun `bottom spacer can reserve more space than the currently visible editor area`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 720f, height = 900f),
            topInset = 80f,
            bottomOcclusionInset = 180f,
          ),
        bottomSpacerVisibleArea =
          EditorVisibleArea(
            viewport = Size(width = 720f, height = 900f),
            topInset = 80f,
            bottomOcclusionInset = 260f,
          ),
        baseBottomSpace = 20f,
      )

    assertEquals(VerticalSpan(top = 140f, bottom = 660f), policy.keepVisibleRange)
    assertEquals(300f, policy.bottomSpacerHeight, FloatTolerance)
  }

  private fun testVisibleArea(): EditorVisibleArea =
    EditorVisibleArea(viewport = Size(width = 720f, height = 900f), topInset = 80f, imeInset = 100f)
}
