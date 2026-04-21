package co.typie.ui.component.sheet

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.unit.Velocity
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.test.runTest

class SheetTopHysteresisOverscrollEffectTest {

  @Test
  fun downwardDragStartsOnlyAfterReturningToExpandedStop() {
    val effect = SheetTopHysteresisOverscrollEffect()
    var offset = 100f

    fun drag(deltaY: Float) {
      effect.applyToScroll(Offset(0f, deltaY), NestedScrollSource.UserInput) { delta ->
        val previousOffset = offset
        offset = (offset + delta.y).coerceAtLeast(0f)
        Offset(0f, offset - previousOffset)
      }
    }

    drag(-150f)
    assertEquals(0f, offset)

    drag(40f)
    assertEquals(0f, offset)

    drag(10f)
    assertEquals(0f, offset)

    drag(1f)
    assertEquals(1f, offset)
  }

  @Test
  fun releaseClearsPendingOverdragForNextGesture() = runTest {
    val effect = SheetTopHysteresisOverscrollEffect()
    var offset = 100f

    fun drag(deltaY: Float) {
      effect.applyToScroll(Offset(0f, deltaY), NestedScrollSource.UserInput) { delta ->
        val previousOffset = offset
        offset = (offset + delta.y).coerceAtLeast(0f)
        Offset(0f, offset - previousOffset)
      }
    }

    drag(-150f)
    assertEquals(0f, offset)

    effect.applyToFling(Velocity.Zero) { Velocity.Zero }

    drag(1f)
    assertEquals(1f, offset)
  }
}
