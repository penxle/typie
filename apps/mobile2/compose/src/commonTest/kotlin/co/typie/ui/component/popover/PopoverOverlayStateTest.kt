package co.typie.ui.component.popover

import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class PopoverOverlayStateTest {

  @Test
  fun clearIgnoresRequestsFromPreviousOwner() {
    val state = PopoverOverlayState()
    val firstOwner = Any()
    val secondOwner = Any()
    val firstEntry = createEntry(firstOwner)
    val secondEntry = createEntry(secondOwner)
    val secondBounds = IntRect(left = 20, top = 30, right = 100, bottom = 110)

    state.show(firstOwner, firstEntry, IntRect.Zero)
    state.show(secondOwner, secondEntry, secondBounds)
    state.clear(firstOwner)

    assertEquals(secondEntry, state.entry)
    assertEquals(secondBounds, state.anchorBounds)
  }

  @Test
  fun updateIgnoresRequestsFromPreviousOwner() {
    val state = PopoverOverlayState()
    val firstOwner = Any()
    val secondOwner = Any()
    val secondEntry = createEntry(secondOwner)
    val secondBounds = IntRect(left = 40, top = 50, right = 140, bottom = 170)

    state.show(firstOwner, createEntry(firstOwner), IntRect.Zero)
    state.show(secondOwner, secondEntry, secondBounds)
    state.update(
      owner = firstOwner,
      anchorBounds = IntRect(left = 1, top = 2, right = 3, bottom = 4),
      progress = 0.25f,
      interactive = false,
    )

    assertEquals(secondEntry, state.entry)
    assertEquals(secondBounds, state.anchorBounds)
    assertEquals(0f, state.progress)
    assertEquals(0f, state.easedProgress)
    assertEquals(true, state.interactive)
  }

  @Test
  fun clearResetsVisibleOverlayForCurrentOwner() {
    val state = PopoverOverlayState()
    val owner = Any()

    state.show(owner, createEntry(owner), IntRect(left = 10, top = 12, right = 34, bottom = 56))
    state.update(
      owner = owner,
      anchorBounds = IntRect(left = 10, top = 12, right = 34, bottom = 56),
      progress = 1f,
      interactive = false,
    )
    state.clear(owner)

    assertNull(state.entry)
    assertEquals(IntRect.Zero, state.anchorBounds)
    assertEquals(0f, state.progress)
    assertEquals(0f, state.easedProgress)
    assertEquals(true, state.interactive)
    assertNull(state.paneBoundsInWindow)
  }

  @Test
  fun detachKeepsOverlayVisibleForClosingAnimation() {
    val state = PopoverOverlayState()
    val owner = Any()
    val entry = createEntry(owner)
    val bounds = IntRect(left = 10, top = 12, right = 34, bottom = 56)
    val progress = 0.8f

    state.show(owner, entry, bounds)
    state.update(owner = owner, anchorBounds = bounds, progress = progress, interactive = true)
    state.detach(owner)

    assertEquals(entry, state.entry)
    assertEquals(bounds, state.anchorBounds)
    assertEquals(progress, state.progress)
    assertEquals(expectedProgress(progress), state.easedProgress)
    assertEquals(false, state.interactive)
    assertEquals(true, state.isDetached)
    assertEquals(false, state.isOwnedBy(owner))
  }

  @Test
  fun easedProgressUsesSameEasingForProgress() {
    val state = PopoverOverlayState()
    val owner = Any()
    val progress = 0.4f
    val bounds = IntRect(left = 10, top = 12, right = 34, bottom = 56)

    state.show(owner, createEntry(owner), bounds)
    state.update(owner = owner, anchorBounds = bounds, progress = progress, interactive = true)

    assertEquals(progress, state.progress)
    assertEquals(expectedProgress(progress), state.easedProgress)
  }
}

private fun expectedProgress(progress: Float): Float {
  return PopoverDefaults.PopoverEasing.transform(progress).coerceIn(0f, 1f)
}

private fun createEntry(owner: Any): PopoverOverlayEntry {
  return PopoverOverlayEntry(
    owner = owner,
    placement = PopoverPlacement.BelowEnd,
    screenPadding = PopoverScreenPadding(left = 0, top = 0, right = 0, bottom = 0),
    collapsedCornerRadius = 0.dp,
    maxWidth = null,
    minWidth = 0.dp,
    expandToMaxWidth = false,
    pane = {},
    anchor = {},
  )
}
