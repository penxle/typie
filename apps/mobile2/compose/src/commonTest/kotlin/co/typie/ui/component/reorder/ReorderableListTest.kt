package co.typie.ui.component.reorder

import androidx.compose.ui.geometry.Rect
import kotlin.test.Test
import kotlin.test.assertEquals

class ReorderableListTest {
  @Test
  fun `drag comparison window y follows dragged item center instead of touch hotspot`() {
    assertEquals(
      110f,
      calculateDragComparisonWindowY(
        pointerWindowY = 130f,
        pointerOffsetWithinItemY = 45f,
        itemHeight = 50f,
      ),
    )
  }

  @Test
  fun `calculateReorderedKeys keeps logical order when animated bounds temporarily lag`() {
    val reordered =
      calculateReorderedKeys(
        orderedKeys = listOf("a", "c", "b", "d"),
        draggedKey = "b",
        comparisonWindowY = 160f,
        itemBounds =
          mapOf(
            "a" to Rect(left = 0f, top = 0f, right = 100f, bottom = 50f),
            "c" to Rect(left = 0f, top = 200f, right = 100f, bottom = 250f),
            "d" to Rect(left = 0f, top = 100f, right = 100f, bottom = 150f),
          ),
      )

    assertEquals(listOf("a", "c", "b", "d"), reordered)
  }

  @Test
  fun `calculateReorderedKeys ignores zero sized bounds from offscreen items`() {
    val reordered =
      calculateReorderedKeys(
        orderedKeys = listOf("dragged", "a", "b", "c", "d", "e"),
        draggedKey = "dragged",
        comparisonWindowY = 522.5f,
        itemBounds =
          mapOf(
            "a" to Rect(left = 0f, top = 587f, right = 100f, bottom = 714f),
            "b" to Rect(left = 0f, top = 714f, right = 100f, bottom = 841f),
            "c" to Rect(left = 0f, top = 0f, right = 0f, bottom = 0f),
            "d" to Rect(left = 0f, top = 0f, right = 0f, bottom = 0f),
            "e" to Rect(left = 0f, top = 0f, right = 0f, bottom = 0f),
          ),
      )

    assertEquals(listOf("dragged", "a", "b", "c", "d", "e"), reordered)
  }
}
