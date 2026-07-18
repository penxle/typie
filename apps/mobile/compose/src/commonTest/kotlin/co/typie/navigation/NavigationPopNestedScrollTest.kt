package co.typie.navigation

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.unit.Velocity
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.test.runTest

class NavigationPopNestedScrollTest {
  @Test
  fun nestedActivationUsesRootDisplacementAndForwardsOnlyOvershoot() {
    val fixture = Fixture()
    fixture.pointerDown()
    fixture.pointerMoveTo(x = 14f)

    assertEquals(Offset.Zero, fixture.postScroll(availableX = 4f))

    fixture.pointerMoveTo(x = 20f)

    assertEquals(Offset(x = 6f, y = 0f), fixture.postScroll(availableX = 6f))
    assertEquals(1, fixture.startCount)
    assertEquals(listOf(5f), fixture.dragAmounts)
  }

  @Test
  fun nestedReleaseForwardsTheActualPreFlingVelocity() = runTest {
    val fixture = Fixture()
    fixture.claimNestedGesture()

    fixture.connection.onPreFling(Velocity(x = 640f, y = 12f))

    assertEquals(640f, fixture.releasedVelocityX)
  }

  private class Fixture {
    val dragAmounts = mutableListOf<Float>()
    var startCount = 0
    var releasedVelocityX: Float? = null
    val connection =
      NavigationPopNestedScroll().apply {
        update(
          activationDistance = 15f,
          canStart = { true },
          onStart = { startCount += 1 },
          onDrag = dragAmounts::add,
          onRelease = { releasedVelocityX = it },
          onCancel = {},
        )
      }

    fun pointerDown() {
      updatePointer(count = 1, pointerId = 1L)
    }

    fun pointerMoveTo(x: Float) {
      updatePointer(count = 1, pointerId = 1L, x = x)
    }

    fun postScroll(availableX: Float): Offset =
      connection.onPostScroll(
        consumed = Offset.Zero,
        available = Offset(x = availableX, y = 0f),
        source = NestedScrollSource.UserInput,
      )

    fun claimNestedGesture() {
      pointerDown()
      pointerMoveTo(x = 20f)
      postScroll(availableX = 20f)
    }

    private fun updatePointer(count: Int, pointerId: Long, x: Float = 0f) {
      connection.updatePressedDragPointerCount(
        count = count,
        pointerId = pointerId,
        position = Offset(x = x, y = 0f),
      )
    }
  }
}
