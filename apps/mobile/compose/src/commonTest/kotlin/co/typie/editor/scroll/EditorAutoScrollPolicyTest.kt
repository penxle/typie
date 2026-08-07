package co.typie.editor.scroll

import androidx.compose.ui.geometry.Size
import co.typie.editor.VerticalSpan
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

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
  fun `keep-visible policy ignores a one-pixel guard correction`() {
    val offset =
      resolveKeepVisibleScrollOffset(
        currentScroll = 240f,
        targetTopInContent = 379f,
        targetBottomInContent = 407f,
        visibleArea = testVisibleArea(),
      )

    assertEquals(null, offset)
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
  fun `keep-visible policy aligns oversized target below viewport bottom to lower guard`() {
    val offset =
      resolveKeepVisibleScrollOffset(
        currentScroll = 300f,
        targetTopInContent = 1000f,
        targetBottomInContent = 1700f,
        visibleArea = testVisibleArea(),
      )

    assertEquals(960f, offset)
  }

  @Test
  fun `keep-visible policy does not scroll when oversized target covers guarded visible area`() {
    val offset =
      resolveKeepVisibleScrollOffset(
        currentScroll = 300f,
        targetTopInContent = 200f,
        targetBottomInContent = 1200f,
        visibleArea = testVisibleArea(),
      )

    assertEquals(null, offset)
  }

  @Test
  fun `keep-visible policy does not scroll when oversized target meets either guard edge`() {
    assertEquals(
      null,
      resolveKeepVisibleScrollOffset(
        currentScroll = 300f,
        targetTopInContent = 440f,
        targetBottomInContent = 1200f,
        visibleArea = testVisibleArea(),
      ),
    )
    assertEquals(
      null,
      resolveKeepVisibleScrollOffset(
        currentScroll = 300f,
        targetTopInContent = 0f,
        targetBottomInContent = 1040f,
        visibleArea = testVisibleArea(),
      ),
    )
  }

  @Test
  fun `keep-visible policy aligns oversized target to violated edge within old slack`() {
    val offset =
      resolveKeepVisibleScrollOffset(
        currentScroll = 300f,
        targetTopInContent = 460f,
        targetBottomInContent = 1200f,
        visibleArea = testVisibleArea(),
      )

    assertEquals(460f, offset)
  }

  @Test
  fun `keep-visible policy clamps oversized target at document edge`() {
    val offset =
      resolveKeepVisibleScrollOffset(
        currentScroll = 10f,
        targetTopInContent = 0f,
        targetBottomInContent = 251f,
        visibleArea =
          EditorVisibleArea(viewport = Size(width = 720f, height = 400f), topInset = 30f),
        maximumScrollY = 400f,
      )

    assertEquals(0f, offset)
  }

  @Test
  fun `keep-visible policy aligns oversized target above viewport top to upper guard`() {
    val offset =
      resolveKeepVisibleScrollOffset(
        currentScroll = 1000f,
        targetTopInContent = 500f,
        targetBottomInContent = 1200f,
        visibleArea = testVisibleArea(),
      )

    assertEquals(360f, offset)
  }

  @Test
  fun `resolved policy keeps typewriter inactive when the preference is disabled`() {
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

    assertFalse(policy.typewriterActive)
    assertEquals(0.5f, policy.typewriterPosition, FloatTolerance)
    assertEquals(
      VerticalSpan(top = 180f, bottom = 740f),
      resolveKeepVisibleRange(
        EditorVisibleArea(
          viewport = Size(width = 720f, height = 900f),
          topInset = 120f,
          imeInset = 100f,
        )
      ),
    )
    assertEquals(450f, requireNotNull(policy.targetTop), FloatTolerance)
    assertEquals(470f, requireNotNull(policy.targetBottom), FloatTolerance)
    assertEquals(140f, policy.bottomPadding, FloatTolerance)
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
  fun `typewriter policy falls back to lower cursor guard for oversized target below viewport`() {
    val offset =
      resolveTypewriterScrollOffset(
        currentScroll = 300f,
        targetTopInContent = 1000f,
        targetBottomInContent = 1700f,
        visibleArea = testVisibleArea(),
        position = 0.5f,
      )

    assertEquals(960f, requireNotNull(offset), FloatTolerance)
  }

  @Test
  fun `typewriter policy keeps viewport when oversized target spans both guard edges`() {
    val offset =
      resolveTypewriterScrollOffset(
        currentScroll = 300f,
        targetTopInContent = 200f,
        targetBottomInContent = 1200f,
        visibleArea = testVisibleArea(),
        position = 0.5f,
      )

    assertEquals(null, offset)
  }

  @Test
  fun `resolved policy activates typewriter when enabled`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea = testVisibleArea(),
        baseBottomSpace = 20f,
        typewriterEnabled = true,
        typewriterPosition = 0.25f,
        targetLineHeight = 32f,
      )

    assertTrue(policy.typewriterActive)
    assertEquals(0.25f, policy.typewriterPosition, FloatTolerance)
    assertEquals(252f, requireNotNull(policy.targetTop), FloatTolerance)
    assertEquals(284f, requireNotNull(policy.targetBottom), FloatTolerance)
    assertEquals(596f, policy.bottomPadding, FloatTolerance)
  }

  @Test
  fun `typewriter bottom padding uses intrinsic layout bottom space`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea = testVisibleArea(),
        baseBottomSpace = 20f,
        typewriterEnabled = true,
        typewriterPosition = 0.25f,
        targetLineHeight = 32f,
      )

    assertTrue(policy.typewriterActive)
    assertEquals(596f, policy.bottomPadding, FloatTolerance)
  }

  @Test
  fun `typewriter bottom padding preserves keep-visible floor above bottom occlusion`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 720f, height = 900f),
            topInset = 80f,
            bottomOcclusionInset = 400f,
          ),
        baseBottomSpace = 20f,
        typewriterEnabled = true,
        typewriterPosition = 0.9f,
        targetLineHeight = 32f,
      )

    assertTrue(policy.typewriterActive)
    assertEquals(440f, policy.bottomPadding, FloatTolerance)
  }

  @Test
  fun `typewriter bottom padding keeps minimum when intrinsic space is sufficient`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea = testVisibleArea(),
        baseBottomSpace = 1000f,
        typewriterEnabled = true,
        typewriterPosition = 1f,
        targetLineHeight = 32f,
      )

    assertTrue(policy.typewriterActive)
    assertEquals(48f, policy.bottomPadding, FloatTolerance)
  }

  @Test
  fun `keep-visible bottom padding respects paginated intrinsic bottom space`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea =
          EditorVisibleArea(viewport = Size(width = 720f, height = 900f), topInset = 120f),
        baseBottomSpace = 40f,
      )

    assertFalse(policy.typewriterActive)
    assertEquals(20f, policy.bottomPadding, FloatTolerance)
  }

  @Test
  fun `page edge reveal bottom padding can exceed cursor policy padding`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea = testVisibleArea(),
        baseBottomSpace = 180f,
        pageBottomRevealPadding = 100f,
      )

    assertFalse(policy.typewriterActive)
    assertEquals(100f, policy.bottomPadding, FloatTolerance)
  }

  @Test
  fun `bottom scroll reserve can increase padding beyond the editor visible area`() {
    val policy =
      resolveEditorAutoScrollPolicy(
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 720f, height = 900f),
            topInset = 80f,
            bottomOcclusionInset = 180f,
          ),
        bottomScrollReserveArea =
          EditorVisibleArea(
            viewport = Size(width = 720f, height = 900f),
            topInset = 80f,
            bottomOcclusionInset = 260f,
          ),
        baseBottomSpace = 20f,
      )

    assertEquals(
      VerticalSpan(top = 140f, bottom = 660f),
      resolveKeepVisibleRange(
        EditorVisibleArea(
          viewport = Size(width = 720f, height = 900f),
          topInset = 80f,
          bottomOcclusionInset = 180f,
        )
      ),
    )
    assertEquals(300f, policy.bottomPadding, FloatTolerance)
  }

  private fun testVisibleArea(): EditorVisibleArea =
    EditorVisibleArea(viewport = Size(width = 720f, height = 900f), topInset = 80f, imeInset = 100f)
}
