package co.typie.editor.viewport

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EditorViewportStateTest {
  @Test
  fun `touch pan updates viewport scroll and records user intent`() {
    val state = EditorViewportState()
    state.updateMeasuredBounds(
      viewportSize = Size(width = 100f, height = 100f),
      contentSize = Size(width = 300f, height = 300f),
    )

    val consumed =
      consumeEditorViewportTouchPan(
        viewportState = state,
        deltaPx = Offset(x = -120f, y = -80f),
        density = 2f,
      )

    assertEquals(Offset(x = -120f, y = -80f), consumed)
    assertEquals(Offset(x = 60f, y = 40f), state.scrollOffset)
    assertEquals(1, state.lastScrollRevision)
    assertFalse(state.lastScrollWasAuto)
  }

  @Test
  fun `auto scroll updates viewport scroll and records auto intent`() {
    val state = EditorViewportState()
    state.updateMeasuredBounds(
      viewportSize = Size(width = 100f, height = 100f),
      contentSize = Size(width = 100f, height = 280f),
    )

    val consumedDelta = state.dispatchDeltaY(deltaY = 120f, isAutoScroll = true)

    assertEquals(120f, consumedDelta)
    assertEquals(Offset(x = 0f, y = 120f), state.scrollOffset)
    assertEquals(1, state.lastScrollRevision)
    assertTrue(state.lastScrollWasAuto)
  }

  @Test
  fun `transform session blocks user pan until pinch ends`() {
    val state = EditorViewportState()
    state.updateMeasuredBounds(
      viewportSize = Size(width = 100f, height = 100f),
      contentSize = Size(width = 300f, height = 300f),
    )
    state.beginTransform()

    val blocked =
      consumeEditorViewportTouchPan(
        viewportState = state,
        deltaPx = Offset(x = -100f, y = -100f),
        density = 2f,
      )

    assertEquals(Offset.Zero, blocked)
    assertEquals(Offset.Zero, state.scrollOffset)
    assertEquals(0, state.lastScrollRevision)

    state.endTransform()

    val consumed =
      consumeEditorViewportTouchPan(
        viewportState = state,
        deltaPx = Offset(x = -100f, y = -100f),
        density = 2f,
      )

    assertEquals(Offset(x = -100f, y = -100f), consumed)
    assertEquals(Offset(x = 50f, y = 50f), state.scrollOffset)
    assertEquals(1, state.lastScrollRevision)
    assertFalse(state.lastScrollWasAuto)
  }

  @Test
  fun `restored scroll offset reclamps when viewport and content resolve`() {
    val state = EditorViewportState(initialScrollOffset = Offset(180f, 120f))

    state.updateMeasuredBounds(
      viewportSize = Size(width = 100f, height = 100f),
      contentSize = Size(width = 210f, height = 160f),
    )

    assertEquals(Offset(110f, 60f), state.scrollOffset)
    assertEquals(0, state.lastScrollRevision)
  }

  @Test
  fun `direct manipulation stays active while scrollbar drag is in progress`() {
    val state = EditorViewportState()

    state.updateScrollableInteractionInProgress(true)
    assertTrue(state.isDirectManipulationInProgress)

    state.updateScrollbarDragInProgress(true)
    state.updateScrollableInteractionInProgress(false)
    assertTrue(state.isDirectManipulationInProgress)

    state.updateScrollbarDragInProgress(false)
    assertFalse(state.isDirectManipulationInProgress)
  }
}
