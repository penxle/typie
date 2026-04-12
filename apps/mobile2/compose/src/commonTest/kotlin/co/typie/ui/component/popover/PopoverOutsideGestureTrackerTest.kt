package co.typie.ui.component.popover

import androidx.compose.ui.geometry.Offset
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class PopoverOutsideGestureTrackerTest {

  @Test
  fun start_tracksGesture_withoutImmediateDismiss() {
    val tracker = PopoverOutsideGestureTracker(origin = Offset(10f, 20f), touchSlop = 8f)

    val start = tracker.start()

    assertTrue(start.dismiss)
    assertFalse(start.consumeChange)
    assertTrue(start.keepTracking)
  }

  @Test
  fun tapOutside_dismissesAndConsumesOnlyRelease() {
    val tracker = PopoverOutsideGestureTracker(origin = Offset(10f, 20f), touchSlop = 8f)
    tracker.start()

    val move = tracker.update(currentPosition = Offset(12f, 22f), isPressed = true)
    val release = tracker.update(currentPosition = Offset(12f, 22f), isPressed = false)

    assertFalse(move.dismiss)
    assertFalse(move.consumeChange)
    assertTrue(move.keepTracking)
    assertFalse(release.dismiss)
    assertTrue(release.consumeChange)
    assertFalse(release.keepTracking)
  }

  @Test
  fun panOutside_keepsTrackingUntilRelease_withoutConsumingGesture() {
    val tracker = PopoverOutsideGestureTracker(origin = Offset(10f, 20f), touchSlop = 8f)
    tracker.start()

    val drag = tracker.update(currentPosition = Offset(24f, 20f), isPressed = true)
    val release = tracker.update(currentPosition = Offset(24f, 20f), isPressed = false)

    assertFalse(drag.dismiss)
    assertFalse(drag.consumeChange)
    assertTrue(drag.keepTracking)
    assertFalse(release.dismiss)
    assertFalse(release.consumeChange)
    assertFalse(release.keepTracking)
  }
}
