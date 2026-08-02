package co.typie.ui.component.reorder

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ReorderLayoutTest {
  @Test
  fun `block offset is derived from absolute lazy indices`() {
    val snapshot =
      layoutSnapshot(
        item("b", lazyIndex = 3, top = 0f, bottom = 50f),
        item("c", lazyIndex = 4, top = 50f, bottom = 100f),
      )

    assertEquals(
      2,
      discoverReorderBlockOffset(displayedKeys = listOf("a", "b", "c"), snapshot = snapshot),
    )
  }

  @Test
  fun `block offset is rejected when visible items imply different offsets`() {
    val snapshot =
      layoutSnapshot(
        item("a", lazyIndex = 1, top = 0f, bottom = 50f),
        item("b", lazyIndex = 3, top = 50f, bottom = 100f),
      )

    assertNull(discoverReorderBlockOffset(listOf("a", "b"), snapshot))
  }

  @Test
  fun `single visible stale item is incompatible with displayed order`() {
    val stale = layoutSnapshot(item("b", lazyIndex = 2, top = 0f, bottom = 50f))

    assertFalse(
      isCompatibleReorderSnapshot(
        displayedKeys = listOf("b", "c", "a"),
        blockOffset = 1,
        snapshot = stale,
      )
    )
    assertTrue(
      isCompatibleReorderSnapshot(
        displayedKeys = listOf("a", "b", "c"),
        blockOffset = 1,
        snapshot = stale,
      )
    )
  }

  @Test
  fun `stale item keys are projected onto the current lazy slots`() {
    val stale =
      layoutSnapshot(
        item("a", lazyIndex = 1, top = -40f, bottom = 10f),
        item("b", lazyIndex = 2, top = 10f, bottom = 60f),
        item("c", lazyIndex = 3, top = 60f, bottom = 110f),
      )

    assertEquals(
      layoutSnapshot(
        item("b", lazyIndex = 1, top = -40f, bottom = 10f),
        item("c", lazyIndex = 2, top = 10f, bottom = 60f),
        item("a", lazyIndex = 3, top = 60f, bottom = 110f),
      ),
      projectReorderSnapshotOntoDisplayedSlots(
        displayedKeys = listOf("b", "c", "a", "d"),
        blockOffset = 1,
        snapshot = stale,
      ),
    )
  }

  @Test
  fun `exact half overlap does not cross downward`() {
    val snapshot = layoutSnapshot(item("a", 0, 0f, 50f), item("b", 1, 50f, 150f))

    assertEquals(
      0,
      targetIndexForDrag(
        displayedKeys = listOf("a", "b"),
        draggedKey = "a",
        direction = 1,
        draggedTop = 25f,
        draggedBottom = 75f,
        blockOffset = 0,
        snapshot = snapshot,
      ),
    )
  }

  @Test
  fun `more than half overlap crosses downward using smaller item height`() {
    val snapshot = layoutSnapshot(item("a", 0, 0f, 50f), item("b", 1, 50f, 150f))

    assertEquals(
      1,
      targetIndexForDrag(
        displayedKeys = listOf("a", "b"),
        draggedKey = "a",
        direction = 1,
        draggedTop = 26f,
        draggedBottom = 76f,
        blockOffset = 0,
        snapshot = snapshot,
      ),
    )
  }

  @Test
  fun `one snapshot can cross several items downward`() {
    val snapshot =
      layoutSnapshot(
        item("a", 0, 0f, 50f),
        item("b", 1, 50f, 100f),
        item("c", 2, 100f, 150f),
        item("d", 3, 150f, 200f),
      )

    assertEquals(
      3,
      targetIndexForDrag(
        displayedKeys = listOf("a", "b", "c", "d"),
        draggedKey = "a",
        direction = 1,
        draggedTop = 160f,
        draggedBottom = 210f,
        blockOffset = 0,
        snapshot = snapshot,
      ),
    )
  }

  @Test
  fun `one snapshot can cross several items upward`() {
    val snapshot =
      layoutSnapshot(
        item("a", 0, 0f, 50f),
        item("b", 1, 50f, 100f),
        item("c", 2, 100f, 150f),
        item("d", 3, 150f, 200f),
      )

    assertEquals(
      0,
      targetIndexForDrag(
        displayedKeys = listOf("a", "b", "c", "d"),
        draggedKey = "d",
        direction = -1,
        draggedTop = -10f,
        draggedBottom = 40f,
        blockOffset = 0,
        snapshot = snapshot,
      ),
    )
  }

  @Test
  fun `target stops before an unpublished adjacent slot`() {
    val snapshot =
      layoutSnapshot(item("a", 0, 0f, 50f), item("b", 1, 50f, 100f), item("d", 3, 150f, 200f))

    assertEquals(
      1,
      targetIndexForDrag(
        displayedKeys = listOf("a", "b", "c", "d"),
        draggedKey = "a",
        direction = 1,
        draggedTop = 220f,
        draggedBottom = 270f,
        blockOffset = 0,
        snapshot = snapshot,
      ),
    )
  }

  @Test
  fun `downward target crosses slots that already scrolled above the viewport`() {
    val snapshot =
      layoutSnapshot(item("c", 2, -40f, 10f), item("d", 3, 10f, 60f), item("e", 4, 60f, 110f))

    assertEquals(
      4,
      targetIndexForDrag(
        displayedKeys = listOf("a", "b", "c", "d", "e"),
        draggedKey = "a",
        direction = 1,
        draggedTop = 70f,
        draggedBottom = 120f,
        blockOffset = 0,
        snapshot = snapshot,
      ),
    )
  }

  @Test
  fun `upward target crosses slots that already scrolled below the viewport`() {
    val snapshot =
      layoutSnapshot(item("a", 0, -10f, 40f), item("b", 1, 40f, 90f), item("c", 2, 90f, 140f))

    assertEquals(
      0,
      targetIndexForDrag(
        displayedKeys = listOf("a", "b", "c", "d", "e"),
        draggedKey = "e",
        direction = -1,
        draggedTop = -20f,
        draggedBottom = 30f,
        blockOffset = 0,
        snapshot = snapshot,
      ),
    )
  }

  private fun item(key: String, lazyIndex: Int, top: Float, bottom: Float) =
    ReorderLayoutItem(key = key, lazyIndex = lazyIndex, top = top, bottom = bottom)

  private fun layoutSnapshot(vararg items: ReorderLayoutItem<String>) =
    ReorderLayoutSnapshot(viewportTop = 0f, viewportBottom = 200f, items = items.toList())
}
