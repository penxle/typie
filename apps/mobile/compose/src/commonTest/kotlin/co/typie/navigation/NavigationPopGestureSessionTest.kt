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
}
