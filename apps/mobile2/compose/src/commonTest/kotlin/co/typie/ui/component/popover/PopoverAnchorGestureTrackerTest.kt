package co.typie.ui.component.popover

import androidx.compose.ui.geometry.Offset
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class PopoverAnchorGestureTrackerTest {

  @Test
  fun consumesGesture_fromPressUntilRelease() {
    val tracker = PopoverAnchorGestureTracker(origin = Offset(24f, 32f), armDistancePx = 9f)

    val start = tracker.start()
    assertTrue(start.consumeChange)
    assertFalse(start.pointerState.isSelectionArmed)
    assertFalse(start.pointerState.isUp)

    val moved =
      tracker.update(currentPosition = Offset(26f, 34f), elapsedMillis = 50L, isPressed = true)
    assertTrue(moved.consumeChange)
    assertFalse(moved.pointerState.isUp)

    val released =
      tracker.update(currentPosition = Offset(26f, 34f), elapsedMillis = 60L, isPressed = false)
    assertTrue(released.consumeChange)
    assertTrue(released.pointerState.isUp)
  }

  @Test
  fun armsSelection_afterDelayAndDistance() {
    val tracker = PopoverAnchorGestureTracker(origin = Offset(10f, 10f), armDistancePx = 9f)

    tracker.start()
    val update =
      tracker.update(
        currentPosition = Offset(25f, 10f),
        elapsedMillis = PopoverDefaults.ArmDelayMs,
        isPressed = true,
      )

    assertTrue(update.pointerState.isSelectionArmed)
  }

  @Test
  fun doesNotArm_beforeDelayOrDistance() {
    val tracker = PopoverAnchorGestureTracker(origin = Offset(10f, 10f), armDistancePx = 9f)

    tracker.start()

    val beforeDelay =
      tracker.update(
        currentPosition = Offset(30f, 10f),
        elapsedMillis = PopoverDefaults.ArmDelayMs - 1,
        isPressed = true,
      )
    assertFalse(beforeDelay.pointerState.isSelectionArmed)

    val belowDistanceTracker =
      PopoverAnchorGestureTracker(origin = Offset(10f, 10f), armDistancePx = 9f)
    belowDistanceTracker.start()
    val belowDistance =
      belowDistanceTracker.update(
        currentPosition = Offset(18f, 10f),
        elapsedMillis = PopoverDefaults.ArmDelayMs,
        isPressed = true,
      )
    assertFalse(belowDistance.pointerState.isSelectionArmed)
  }
}
