package co.typie.ui.component.popover

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class PopoverListTest {

  @Test
  fun hitTest_returnsIndex_whenInsideBounds() {
    val bounds = mapOf(
      0 to Rect(0f, 0f, 100f, 44f),
      1 to Rect(0f, 44f, 100f, 88f),
      2 to Rect(0f, 88f, 100f, 132f),
    )
    assertEquals(0, hitTestItems(Offset(50f, 22f), bounds))
    assertEquals(1, hitTestItems(Offset(50f, 66f), bounds))
    assertEquals(2, hitTestItems(Offset(50f, 110f), bounds))
  }

  @Test
  fun hitTest_returnsNull_whenOutsideAllBounds() {
    val bounds = mapOf(
      0 to Rect(0f, 0f, 100f, 44f),
      1 to Rect(0f, 44f, 100f, 88f),
    )
    assertNull(hitTestItems(Offset(50f, 200f), bounds))
    assertNull(hitTestItems(Offset(150f, 22f), bounds))
  }

  @Test
  fun hitTest_returnsNull_whenEmpty() {
    assertNull(hitTestItems(Offset(50f, 50f), emptyMap()))
  }

  @Test
  fun hitTest_edgeBounds_topLeftInclusive() {
    val bounds = mapOf(0 to Rect(10f, 10f, 100f, 50f))
    assertEquals(0, hitTestItems(Offset(10f, 10f), bounds))
  }

  @Test
  fun hitTest_edgeBounds_bottomRightExclusive() {
    val bounds = mapOf(0 to Rect(10f, 10f, 100f, 50f))
    assertNull(hitTestItems(Offset(100f, 50f), bounds))
  }
}
