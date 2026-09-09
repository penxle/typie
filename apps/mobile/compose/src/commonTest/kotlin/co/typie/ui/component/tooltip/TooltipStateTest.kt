package co.typie.ui.component.tooltip

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.unit.IntSize
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class TooltipStateTest {
  @Test
  fun firstHoverWaitsAndLeavingCancelsThePendingTooltip() = runTest {
    val state = TooltipState(backgroundScope)
    val first = TooltipAnchor()
    val second = TooltipAnchor()
    state.enter(first)
    advanceTimeBy(400)
    assertNull(state.active)
    state.leave(first)
    state.enter(second)
    advanceTimeBy(499)
    runCurrent()
    assertNull(state.active)
    advanceTimeBy(1)
    runCurrent()
    assertSame(second, state.active)
  }

  @Test
  fun nearbyControlsShareTheDelayEvenAcrossTheirGap() = runTest {
    val state = TooltipState(backgroundScope)
    val first = TooltipAnchor()
    val second = TooltipAnchor()
    state.enter(first)
    advanceTimeBy(500)
    runCurrent()
    state.leave(first)
    advanceTimeBy(50)
    assertSame(first, state.active)
    state.enter(second)
    assertSame(second, state.active)
    // A late exit from the old anchor must not hide the new one.
    state.leave(first)
    advanceTimeBy(500)
    assertSame(second, state.active)
    state.leave(second)
    advanceTimeBy(81)
    assertNull(state.active)
    state.onHidden()
    advanceTimeBy(301)
    state.enter(first)
    runCurrent()
    assertNull(state.active)
  }

  @Test
  fun navigationDismissalCancelsBothVisibleAndPendingTooltips() = runTest {
    val state = TooltipState(backgroundScope)
    val anchor = TooltipAnchor()
    state.enter(anchor)
    advanceTimeBy(250)
    state.dismiss()
    advanceTimeBy(1000)
    assertNull(state.active)
    state.enter(anchor)
    advanceTimeBy(500)
    runCurrent()
    assertSame(anchor, state.active)
    state.dismiss()
    state.enter(anchor)
    runCurrent()
    assertNull(state.active)
  }

  @Test
  fun clickingPreservesWarmHoverUntilAfterActualOutroCompletion() = runTest {
    val state = TooltipState(backgroundScope)
    val first = TooltipAnchor()
    val second = TooltipAnchor()
    state.enter(first)
    advanceTimeBy(500)
    runCurrent()
    state.close(first)
    // A delayed renderer must not spend the warm window while the old bubble is still visible.
    advanceTimeBy(600)
    state.enter(second)
    assertSame(second, state.active)
    state.close(second)
    state.onHidden()
    advanceTimeBy(299)
    state.enter(first)
    assertSame(first, state.active)
    state.onHidden() // Stale completion must not cool a visible tooltip.
    advanceTimeBy(500)
    state.enter(second)
    assertSame(second, state.active)
    state.close(second)
    state.onHidden()
    advanceTimeBy(301)
    state.enter(first)
    assertNull(state.active)
  }

  @Test
  fun clickingAnotherAnchorDoesNotCancelTheHoveredAnchor() = runTest {
    val state = TooltipState(backgroundScope)
    val hovered = TooltipAnchor()
    val clicked = TooltipAnchor()
    state.enter(hovered)
    state.close(clicked)
    advanceTimeBy(500)
    runCurrent()
    assertSame(hovered, state.active)
    state.close(clicked)
    assertSame(hovered, state.active)
    state.close(hovered)
    assertNull(state.active)
  }

  @Test
  fun removingOrDisablingAnAnchorCancelsItsPendingOrVisibleTooltip() = runTest {
    val state = TooltipState(backgroundScope)
    val anchor = TooltipAnchor()
    state.enter(anchor)
    state.remove(anchor)
    advanceTimeBy(1000)
    assertNull(state.active)
    state.enter(anchor)
    advanceTimeBy(500)
    runCurrent()
    state.remove(TooltipAnchor())
    assertSame(anchor, state.active)
    state.remove(anchor)
    assertNull(state.active)
  }

  @Test
  fun tooltipFlipsBelowTopBarAndStaysInsideTheWindow() {
    val viewport = IntSize(320, 600)
    val bubble = IntSize(120, 26)
    val top = tooltipPosition(Rect(0f, 5f, 40f, 45f), viewport, bubble, 8f)
    assertFalse(top.above)
    assertEquals(8, top.offset.x)
    assertEquals(53, top.offset.y)
    val bottom = tooltipPosition(Rect(290f, 540f, 320f, 590f), viewport, bubble, 8f)
    assertTrue(bottom.above)
    assertEquals(192, bottom.offset.x)
    assertEquals(506, bottom.offset.y)
  }
}
