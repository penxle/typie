package co.typie.screen.editor.editor.overlay

import androidx.compose.ui.geometry.Offset
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorScrollbarGesturePolicyTest {
  @Test
  fun `stationary vertical press stays pending before hold duration`() {
    assertEquals(
      EditorScrollbarDragDecision.Pending,
      resolveEditorScrollbarDragDecision(
        horizontal = false,
        directDragEnabled = true,
        displacement = Offset.Zero,
        touchSlop = 8f,
        elapsedMillis = 299L,
      ),
    )
  }

  @Test
  fun `horizontal movement within touch slop stays pending before hold duration`() {
    assertEquals(
      EditorScrollbarDragDecision.Pending,
      resolveEditorScrollbarDragDecision(
        horizontal = true,
        directDragEnabled = true,
        displacement = Offset(x = 4f, y = 2f),
        touchSlop = 8f,
        elapsedMillis = 299L,
      ),
    )
  }

  @Test
  fun `movement exactly at touch slop stays pending before hold duration`() {
    assertEquals(
      EditorScrollbarDragDecision.Pending,
      resolveEditorScrollbarDragDecision(
        horizontal = false,
        directDragEnabled = true,
        displacement = Offset(x = 0f, y = 8f),
        touchSlop = 8f,
        elapsedMillis = 299L,
      ),
    )
  }

  @Test
  fun `stationary vertical press claims at hold duration when direct drag is enabled`() {
    assertEquals(
      EditorScrollbarDragDecision.Claim,
      resolveEditorScrollbarDragDecision(
        horizontal = false,
        directDragEnabled = true,
        displacement = Offset.Zero,
        touchSlop = 8f,
        elapsedMillis = 300L,
      ),
    )
  }

  @Test
  fun `horizontal movement within touch slop claims at hold duration`() {
    assertEquals(
      EditorScrollbarDragDecision.Claim,
      resolveEditorScrollbarDragDecision(
        horizontal = true,
        directDragEnabled = true,
        displacement = Offset(x = 4f, y = 2f),
        touchSlop = 8f,
        elapsedMillis = 300L,
      ),
    )
  }

  @Test
  fun `vertical axis drag claims after slop before hold duration`() {
    assertEquals(
      EditorScrollbarDragDecision.Claim,
      resolveEditorScrollbarDragDecision(
        horizontal = false,
        directDragEnabled = true,
        displacement = Offset(x = 2f, y = 12f),
        touchSlop = 8f,
        elapsedMillis = 0L,
      ),
    )
  }

  @Test
  fun `horizontal cross axis drag yields after slop before hold duration`() {
    assertEquals(
      EditorScrollbarDragDecision.Yield,
      resolveEditorScrollbarDragDecision(
        horizontal = true,
        directDragEnabled = true,
        displacement = Offset(x = 2f, y = 12f),
        touchSlop = 8f,
        elapsedMillis = 0L,
      ),
    )
  }

  @Test
  fun `cross axis drag yields after slop at hold duration`() {
    assertEquals(
      EditorScrollbarDragDecision.Yield,
      resolveEditorScrollbarDragDecision(
        horizontal = true,
        directDragEnabled = true,
        displacement = Offset(x = 2f, y = 12f),
        touchSlop = 8f,
        elapsedMillis = 300L,
      ),
    )
  }

  @Test
  fun `equal axis and cross axis movement past Euclidean slop yields`() {
    assertEquals(
      EditorScrollbarDragDecision.Yield,
      resolveEditorScrollbarDragDecision(
        horizontal = false,
        directDragEnabled = true,
        displacement = Offset(x = 8f, y = 8f),
        touchSlop = 8f,
        elapsedMillis = 0L,
      ),
    )
  }

  @Test
  fun `stationary horizontal press yields at hold duration when direct drag is disabled`() {
    assertEquals(
      EditorScrollbarDragDecision.Yield,
      resolveEditorScrollbarDragDecision(
        horizontal = true,
        directDragEnabled = false,
        displacement = Offset.Zero,
        touchSlop = 8f,
        elapsedMillis = 300L,
      ),
    )
  }
}
