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
  fun `stable source is projected downward with variable heights and spacing`() {
    val source =
      layoutSnapshot(
        item("a", lazyIndex = 1, top = 0f, bottom = 40f),
        item("b", lazyIndex = 2, top = 50f, bottom = 130f),
        item("c", lazyIndex = 3, top = 140f, bottom = 170f),
      )

    assertEquals(
      layoutSnapshot(
        item("a", lazyIndex = 3, top = 130f, bottom = 170f),
        item("b", lazyIndex = 1, top = 0f, bottom = 80f),
        item("c", lazyIndex = 2, top = 90f, bottom = 120f),
      ),
      projectReorderLayoutFromStableSource(
          layoutKeys = listOf("a", "b", "c"),
          projectedKeys = listOf("b", "c", "a"),
          draggedKey = "a",
          draggedHeight = 40f,
          itemSpacing = 10f,
          blockOffset = 1,
          snapshot = source,
        )
        .snapshot,
    )
  }

  @Test
  fun `stable source is projected upward with variable heights and spacing`() {
    val source =
      layoutSnapshot(
        item("a", lazyIndex = 0, top = 0f, bottom = 30f),
        item("b", lazyIndex = 1, top = 40f, bottom = 100f),
        item("c", lazyIndex = 2, top = 110f, bottom = 150f),
        item("d", lazyIndex = 3, top = 160f, bottom = 230f),
      )

    assertEquals(
      layoutSnapshot(
        item("a", lazyIndex = 0, top = 0f, bottom = 30f),
        item("b", lazyIndex = 2, top = 120f, bottom = 180f),
        item("c", lazyIndex = 3, top = 190f, bottom = 230f),
        item("d", lazyIndex = 1, top = 40f, bottom = 110f),
      ),
      projectReorderLayoutFromStableSource(
          layoutKeys = listOf("a", "b", "c", "d"),
          projectedKeys = listOf("a", "d", "b", "c"),
          draggedKey = "d",
          draggedHeight = 70f,
          itemSpacing = 10f,
          blockOffset = 0,
          snapshot = source,
        )
        .snapshot,
    )
  }

  @Test
  fun `reversing to the source index clears sibling displacement`() {
    assertEquals(
      0f,
      reorderItemDisplacement(
        layoutKeys = listOf("a", "b", "c"),
        projectedKeys = listOf("a", "b", "c"),
        draggedKey = "c",
        itemKey = "b",
        draggedHeight = 80f,
        itemSpacing = 12f,
      ),
    )
  }

  @Test
  fun `dragged destination top follows the actual target slot height`() {
    val source =
      layoutSnapshot(
        item("a", lazyIndex = 0, top = 0f, bottom = 40f),
        item("b", lazyIndex = 1, top = 50f, bottom = 150f),
      )

    val projected =
      projectReorderLayoutFromStableSource(
          layoutKeys = listOf("a", "b"),
          projectedKeys = listOf("b", "a"),
          draggedKey = "a",
          draggedHeight = 40f,
          itemSpacing = 10f,
          blockOffset = 0,
          snapshot = source,
        )
        .snapshot

    assertEquals(110f, projected.items.single { it.key == "a" }.top)
  }

  @Test
  fun `upward destination just below the published range is extrapolated from its boundary`() {
    val source =
      ReorderLayoutSnapshot(
        viewportTop = 0f,
        viewportBottom = 160f,
        itemSpacing = 10f,
        items =
          listOf(
            item("a", lazyIndex = 0, top = 0f, bottom = 40f),
            item("b", lazyIndex = 1, top = 50f, bottom = 90f),
            item("c", lazyIndex = 2, top = 100f, bottom = 150f),
            item("e", lazyIndex = 4, top = 300f, bottom = 350f),
          ),
      )

    val projected =
      projectReorderLayoutFromStableSource(
          layoutKeys = listOf("a", "b", "c", "d", "e"),
          projectedKeys = listOf("a", "b", "c", "e", "d"),
          draggedKey = "e",
          draggedHeight = 50f,
          itemSpacing = 10f,
          blockOffset = 0,
          snapshot = source,
        )
        .snapshot

    assertEquals(160f, projected.items.single { it.key == "e" }.top)
  }

  @Test
  fun `downward destination just above the published range is extrapolated from its boundary`() {
    val source =
      ReorderLayoutSnapshot(
        viewportTop = 100f,
        viewportBottom = 260f,
        itemSpacing = 10f,
        items =
          listOf(
            item("a", lazyIndex = 0, top = -100f, bottom = -50f),
            item("c", lazyIndex = 2, top = 100f, bottom = 140f),
            item("d", lazyIndex = 3, top = 150f, bottom = 190f),
            item("e", lazyIndex = 4, top = 200f, bottom = 240f),
          ),
      )

    val projected =
      projectReorderLayoutFromStableSource(
          layoutKeys = listOf("a", "b", "c", "d", "e"),
          projectedKeys = listOf("b", "a", "c", "d", "e"),
          draggedKey = "a",
          draggedHeight = 50f,
          itemSpacing = 10f,
          blockOffset = 0,
          snapshot = source,
        )
        .snapshot

    assertEquals(40f, projected.items.single { it.key == "a" }.top)
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
