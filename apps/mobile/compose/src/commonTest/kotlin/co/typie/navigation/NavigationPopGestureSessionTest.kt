package co.typie.navigation

import androidx.compose.ui.geometry.Offset
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class NavigationPopGestureSessionTest {
  @Test
  fun zeroDeltaKeepsSessionPossibleForNextMovement() {
    val session = NavigationPopGestureSession()

    assertFalse(session.tryClaim(initialDrag = Offset.Zero, childConsumed = false))

    assertTrue(session.tryClaim(initialDrag = Offset(x = 10f, y = 0f), childConsumed = false))
  }

  @Test
  fun multiTouchRejectsClaimsUntilEveryPointerIsUp() {
    val session = NavigationPopGestureSession()

    session.updatePressedPointerCount(1)
    session.updatePressedPointerCount(2)
    assertFalse(session.tryClaim(initialDrag = Offset(x = 10f, y = 0f), childConsumed = false))

    session.reset()
    session.updatePressedPointerCount(1)
    assertFalse(session.tryClaim(initialDrag = Offset(x = 10f, y = 0f), childConsumed = false))

    session.updatePressedPointerCount(0)
    assertFalse(session.tryClaim(initialDrag = Offset(x = 10f, y = 0f), childConsumed = false))

    session.updatePressedPointerCount(1)
    assertTrue(session.tryClaim(initialDrag = Offset(x = 10f, y = 0f), childConsumed = false))
  }

  @Test
  fun multiTouchRejectsAnAlreadyClaimedSession() {
    val session = NavigationPopGestureSession()

    session.updatePressedPointerCount(1)
    assertTrue(session.tryClaim(initialDrag = Offset(x = 10f, y = 0f), childConsumed = false))

    assertTrue(session.updatePressedPointerCount(2))
    assertFalse(session.isClaimed)
  }

  @Test
  fun downInSystemBackZoneRejectsClaimsUntilNextGesture() {
    val session = NavigationPopGestureSession()

    session.updatePressedPointerCount(1, downInSystemBackZone = true)
    assertTrue(session.isSystemBackZoneRejected)
    assertFalse(session.tryClaim(initialDrag = Offset(x = 10f, y = 0f), childConsumed = false))

    session.reset()
    assertFalse(session.tryClaim(initialDrag = Offset(x = 10f, y = 0f), childConsumed = false))

    session.updatePressedPointerCount(0)
    session.updatePressedPointerCount(1)
    assertFalse(session.isSystemBackZoneRejected)
    assertTrue(session.tryClaim(initialDrag = Offset(x = 10f, y = 0f), childConsumed = false))
  }

  @Test
  fun claimedSessionSurvivesAllUpUntilNestedScrollTerminal() {
    val session = NavigationPopGestureSession()

    session.updatePressedPointerCount(1)
    assertTrue(session.tryClaim(initialDrag = Offset(x = 10f, y = 0f), childConsumed = false))

    session.updatePressedPointerCount(0)
    assertTrue(session.isClaimed)

    session.reset()
    assertFalse(session.isClaimed)
  }
}
