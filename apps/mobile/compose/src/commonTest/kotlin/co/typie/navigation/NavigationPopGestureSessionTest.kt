package co.typie.navigation

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class NavigationPopGestureSessionTest {
  @Test
  fun multiTouchRejectsClaimsUntilEveryPointerIsUp() {
    val session = NavigationPopGestureSession()

    session.updatePressedDragPointerCount(1)
    session.updatePressedDragPointerCount(2)
    assertFalse(session.tryClaim())

    session.reset()
    session.updatePressedDragPointerCount(1)
    assertFalse(session.tryClaim())

    session.updatePressedDragPointerCount(0)
    assertFalse(session.tryClaim())

    session.updatePressedDragPointerCount(1)
    assertTrue(session.tryClaim())
  }

  @Test
  fun multiTouchRejectsAnAlreadyClaimedSession() {
    val session = NavigationPopGestureSession()

    session.updatePressedDragPointerCount(1)
    assertTrue(session.tryClaim())

    session.updatePressedDragPointerCount(2)
    assertFalse(session.isClaimed)
  }

  @Test
  fun downInSystemBackZoneRejectsClaimsUntilNextGesture() {
    val session = NavigationPopGestureSession()

    session.updatePressedDragPointerCount(1, downInSystemBackZone = true)
    assertTrue(session.isCurrentSequenceRejected)
    assertFalse(session.tryClaim())

    session.reset()
    assertFalse(session.tryClaim())

    session.updatePressedDragPointerCount(0)
    session.updatePressedDragPointerCount(1)
    assertFalse(session.isCurrentSequenceRejected)
    assertTrue(session.tryClaim())
  }

  @Test
  fun claimedSessionSurvivesAllUpUntilNestedScrollTerminal() {
    val session = NavigationPopGestureSession()

    session.updatePressedDragPointerCount(1)
    assertTrue(session.tryClaim())

    session.updatePressedDragPointerCount(0)
    assertTrue(session.isClaimed)

    session.reset()
    assertFalse(session.isClaimed)
  }
}
