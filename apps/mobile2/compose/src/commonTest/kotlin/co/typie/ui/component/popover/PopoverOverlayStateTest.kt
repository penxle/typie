package co.typie.ui.component.popover

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertSame

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
      entry = createEntry(firstOwner),
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
  fun updateReplacesEntryForCurrentOwner() {
    val state = PopoverOverlayState()
    val owner = Any()
    val initialEntry = createEntry(owner)
    val updatedEntry = createEntry(owner)
    val bounds = IntRect(left = 40, top = 50, right = 140, bottom = 170)

    state.show(owner, initialEntry, IntRect.Zero)
    state.update(
      owner = owner,
      entry = updatedEntry,
      anchorBounds = bounds,
      progress = 0.25f,
      interactive = false,
    )

    assertSame(updatedEntry, state.entry)
    assertEquals(bounds, state.anchorBounds)
    assertEquals(0.25f, state.progress)
    assertEquals(false, state.interactive)
  }

  @Test
  fun clearResetsVisibleOverlayForCurrentOwner() {
    val state = PopoverOverlayState()
    val owner = Any()
    val entry = createEntry(owner)

    state.show(owner, entry, IntRect(left = 10, top = 12, right = 34, bottom = 56))
    state.update(
      owner = owner,
      entry = entry,
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
    state.update(
      owner = owner,
      entry = entry,
      anchorBounds = bounds,
      progress = progress,
      interactive = true,
    )
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
    val entry = createEntry(owner)

    state.show(owner, entry, bounds)
    state.update(
      owner = owner,
      entry = entry,
      anchorBounds = bounds,
      progress = progress,
      interactive = true,
    )

    assertEquals(progress, state.progress)
    assertEquals(expectedProgress(progress), state.easedProgress)
  }

  @Test
  fun outsideDismissUsesCurrentOwnerCallback() {
    val state = PopoverOverlayState()
    val firstOwner = Any()
    val secondOwner = Any()
    var firstDismissCount = 0
    var secondDismissCount = 0

    state.show(firstOwner, createEntry(firstOwner), IntRect.Zero)
    state.updateOutsideDismiss(firstOwner) { firstDismissCount += 1 }
    state.show(secondOwner, createEntry(secondOwner), IntRect.Zero)

    state.dismissFromOutsideGesture()
    state.updateOutsideDismiss(secondOwner) { secondDismissCount += 1 }
    state.dismissFromOutsideGesture()

    assertEquals(0, firstDismissCount)
    assertEquals(1, secondDismissCount)
  }

  @Test
  fun outsideDismissPaneBoundsAreExposedOnlyWhileDismissIsArmed() {
    val state = PopoverOverlayState()
    val owner = Any()
    val paneBounds = Rect(left = 10f, top = 20f, right = 110f, bottom = 220f)

    state.show(owner, createEntry(owner), IntRect.Zero)
    state.updatePaneBounds(owner, paneBounds)

    assertNull(state.outsideDismissPaneBoundsInWindow)

    state.updateOutsideDismiss(owner) {}

    assertEquals(paneBounds, state.outsideDismissPaneBoundsInWindow)

    state.clearOutsideDismiss(owner)

    assertNull(state.outsideDismissPaneBoundsInWindow)
  }

  @Test
  fun endingOlderOutsideDismissGestureDoesNotCancelNewerGesture() {
    val state = PopoverOverlayState()

    val firstGestureId = state.beginOutsideDismissGesture()
    val secondGestureId = state.beginOutsideDismissGesture()

    state.endOutsideDismissGesture(firstGestureId)

    assertEquals(true, state.isOutsideDismissGestureActive)

    state.endOutsideDismissGesture(secondGestureId)

    assertEquals(false, state.isOutsideDismissGestureActive)
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
