package co.typie.ui.component.reorder

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.coroutines.test.runTest

class ReorderableListTest {
  @Test
  fun `drag comparison window y follows dragged item center`() {
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
        comparisonWindowY = 125f,
        itemBounds =
          mapOf(
            "a" to Rect(left = 0f, top = 0f, right = 100f, bottom = 50f),
            "c" to Rect(left = 0f, top = 200f, right = 100f, bottom = 250f),
            "d" to Rect(left = 0f, top = 100f, right = 100f, bottom = 150f),
          ),
        referenceSlotBoundsByIndex =
          mapOf(
            0 to Rect(left = 0f, top = 0f, right = 100f, bottom = 50f),
            1 to Rect(left = 0f, top = 50f, right = 100f, bottom = 100f),
            2 to Rect(left = 0f, top = 100f, right = 100f, bottom = 150f),
            3 to Rect(left = 0f, top = 150f, right = 100f, bottom = 200f),
          ),
      )

    assertEquals(listOf("a", "c", "b", "d"), reordered)
  }

  @Test
  fun `calculateReorderedKeys does not oscillate when drag pauses after swap`() {
    val reordered =
      calculateReorderedKeys(
        orderedKeys = listOf("a", "c", "b", "d"),
        draggedKey = "b",
        comparisonWindowY = 100f,
        itemBounds =
          mapOf(
            "a" to Rect(left = 0f, top = 0f, right = 100f, bottom = 50f),
            "c" to Rect(left = 0f, top = 100f, right = 100f, bottom = 150f),
            "d" to Rect(left = 0f, top = 150f, right = 100f, bottom = 200f),
          ),
        referenceSlotBoundsByIndex =
          mapOf(
            0 to Rect(left = 0f, top = 0f, right = 100f, bottom = 50f),
            1 to Rect(left = 0f, top = 50f, right = 100f, bottom = 100f),
            2 to Rect(left = 0f, top = 100f, right = 100f, bottom = 150f),
            3 to Rect(left = 0f, top = 150f, right = 100f, bottom = 200f),
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
        referenceSlotBoundsByIndex =
          mapOf(
            0 to Rect(left = 0f, top = 460f, right = 100f, bottom = 587f),
            1 to Rect(left = 0f, top = 587f, right = 100f, bottom = 714f),
            2 to Rect(left = 0f, top = 714f, right = 100f, bottom = 841f),
          ),
      )

    assertEquals(listOf("dragged", "a", "b", "c", "d", "e"), reordered)
  }

  @Test
  fun `calculateReorderedKeys waits until dragged item covers half of next slot`() {
    val reordered =
      calculateReorderedKeys(
        orderedKeys = listOf("dragged", "next", "tail"),
        draggedKey = "dragged",
        comparisonWindowY = 70f,
        itemBounds =
          mapOf(
            "dragged" to Rect(left = 0f, top = 40f, right = 100f, bottom = 100f),
            "next" to Rect(left = 0f, top = 60f, right = 100f, bottom = 160f),
            "tail" to Rect(left = 0f, top = 160f, right = 100f, bottom = 200f),
          ),
        referenceSlotBoundsByIndex =
          mapOf(
            0 to Rect(left = 0f, top = 0f, right = 100f, bottom = 60f),
            1 to Rect(left = 0f, top = 60f, right = 100f, bottom = 160f),
            2 to Rect(left = 0f, top = 160f, right = 100f, bottom = 200f),
          ),
      )

    assertEquals(listOf("dragged", "next", "tail"), reordered)
  }

  @Test
  fun `calculateReorderedKeys swaps once dragged item covers half of next slot`() {
    val reordered =
      calculateReorderedKeys(
        orderedKeys = listOf("dragged", "next", "tail"),
        draggedKey = "dragged",
        comparisonWindowY = 80f,
        movementDirection = 1,
        itemBounds =
          mapOf(
            "dragged" to Rect(left = 0f, top = 50f, right = 100f, bottom = 110f),
            "next" to Rect(left = 0f, top = 60f, right = 100f, bottom = 160f),
            "tail" to Rect(left = 0f, top = 160f, right = 100f, bottom = 200f),
          ),
        referenceSlotBoundsByIndex =
          mapOf(
            0 to Rect(left = 0f, top = 0f, right = 100f, bottom = 60f),
            1 to Rect(left = 0f, top = 60f, right = 100f, bottom = 160f),
            2 to Rect(left = 0f, top = 160f, right = 100f, bottom = 200f),
          ),
      )

    assertEquals(listOf("next", "dragged", "tail"), reordered)
  }

  @Test
  fun `calculateReorderedKeys waits until dragged item covers half of previous slot`() {
    val reordered =
      calculateReorderedKeys(
        orderedKeys = listOf("head", "dragged", "tail"),
        draggedKey = "dragged",
        comparisonWindowY = 100f,
        itemBounds =
          mapOf(
            "head" to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            "dragged" to Rect(left = 0f, top = 70f, right = 100f, bottom = 130f),
            "tail" to Rect(left = 0f, top = 160f, right = 100f, bottom = 200f),
          ),
        referenceSlotBoundsByIndex =
          mapOf(
            0 to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            1 to Rect(left = 0f, top = 100f, right = 100f, bottom = 160f),
            2 to Rect(left = 0f, top = 160f, right = 100f, bottom = 200f),
          ),
      )

    assertEquals(listOf("head", "dragged", "tail"), reordered)
  }

  @Test
  fun `calculateReorderedKeys swaps once dragged item covers half of previous slot`() {
    val reordered =
      calculateReorderedKeys(
        orderedKeys = listOf("head", "dragged", "tail"),
        draggedKey = "dragged",
        comparisonWindowY = 80f,
        movementDirection = -1,
        itemBounds =
          mapOf(
            "head" to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            "dragged" to Rect(left = 0f, top = 50f, right = 100f, bottom = 110f),
            "tail" to Rect(left = 0f, top = 160f, right = 100f, bottom = 200f),
          ),
        referenceSlotBoundsByIndex =
          mapOf(
            0 to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            1 to Rect(left = 0f, top = 100f, right = 100f, bottom = 160f),
            2 to Rect(left = 0f, top = 160f, right = 100f, bottom = 200f),
          ),
      )

    assertEquals(listOf("dragged", "head", "tail"), reordered)
  }

  @Test
  fun `calculateReorderedKeys swaps when small dragged item covers half of larger next item relative to smaller height`() {
    val reordered =
      calculateReorderedKeys(
        orderedKeys = listOf("dragged", "large", "tail"),
        draggedKey = "dragged",
        comparisonWindowY = 100f,
        movementDirection = 1,
        itemBounds =
          mapOf(
            "dragged" to Rect(left = 0f, top = 70f, right = 100f, bottom = 130f),
            "large" to Rect(left = 0f, top = 100f, right = 100f, bottom = 300f),
            "tail" to Rect(left = 0f, top = 300f, right = 100f, bottom = 360f),
          ),
        referenceSlotBoundsByIndex =
          mapOf(
            0 to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            1 to Rect(left = 0f, top = 100f, right = 100f, bottom = 300f),
            2 to Rect(left = 0f, top = 300f, right = 100f, bottom = 360f),
          ),
        referenceItemBoundsByKey =
          mapOf(
            "dragged" to Rect(left = 0f, top = 0f, right = 100f, bottom = 60f),
            "large" to Rect(left = 0f, top = 100f, right = 100f, bottom = 300f),
            "tail" to Rect(left = 0f, top = 300f, right = 100f, bottom = 360f),
          ),
      )

    assertEquals(listOf("large", "dragged", "tail"), reordered)
  }

  @Test
  fun `calculateReorderedKeys swaps when small dragged item covers half of larger previous item relative to smaller height`() {
    val reordered =
      calculateReorderedKeys(
        orderedKeys = listOf("head", "dragged", "tail"),
        draggedKey = "dragged",
        comparisonWindowY = 200f,
        movementDirection = -1,
        itemBounds =
          mapOf(
            "head" to Rect(left = 0f, top = 0f, right = 100f, bottom = 200f),
            "dragged" to Rect(left = 0f, top = 170f, right = 100f, bottom = 230f),
            "tail" to Rect(left = 0f, top = 260f, right = 100f, bottom = 320f),
          ),
        referenceSlotBoundsByIndex =
          mapOf(
            0 to Rect(left = 0f, top = 0f, right = 100f, bottom = 200f),
            1 to Rect(left = 0f, top = 200f, right = 100f, bottom = 260f),
            2 to Rect(left = 0f, top = 260f, right = 100f, bottom = 320f),
          ),
        referenceItemBoundsByKey =
          mapOf(
            "head" to Rect(left = 0f, top = 0f, right = 100f, bottom = 200f),
            "dragged" to Rect(left = 0f, top = 200f, right = 100f, bottom = 260f),
            "tail" to Rect(left = 0f, top = 260f, right = 100f, bottom = 320f),
          ),
      )

    assertEquals(listOf("dragged", "head", "tail"), reordered)
  }

  @Test
  fun `calculateReorderedKeys uses adjacent item height when returning over larger original slot`() {
    val reordered =
      calculateReorderedKeys(
        orderedKeys = listOf("dragged", "small", "tail"),
        draggedKey = "dragged",
        comparisonWindowY = 0f,
        movementDirection = 1,
        itemBounds =
          mapOf(
            "dragged" to Rect(left = 0f, top = -25f, right = 100f, bottom = 175f),
            "small" to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            "tail" to Rect(left = 0f, top = 300f, right = 100f, bottom = 400f),
          ),
        referenceSlotBoundsByIndex =
          mapOf(
            0 to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            1 to Rect(left = 0f, top = 100f, right = 100f, bottom = 300f),
            2 to Rect(left = 0f, top = 300f, right = 100f, bottom = 400f),
          ),
        referenceItemBoundsByKey =
          mapOf(
            "dragged" to Rect(left = 0f, top = 100f, right = 100f, bottom = 300f),
            "small" to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            "tail" to Rect(left = 0f, top = 300f, right = 100f, bottom = 400f),
          ),
      )

    assertEquals(listOf("small", "dragged", "tail"), reordered)
  }

  @Test
  fun `calculateReorderedKeys does not reverse upward swap while pointer pauses`() {
    val reordered =
      calculateReorderedKeys(
        orderedKeys = listOf("dragged", "small", "tail"),
        draggedKey = "dragged",
        comparisonWindowY = 100f,
        itemBounds =
          mapOf(
            "dragged" to Rect(left = 0f, top = -50f, right = 100f, bottom = 250f),
            "small" to Rect(left = 0f, top = 100f, right = 100f, bottom = 200f),
            "tail" to Rect(left = 0f, top = 300f, right = 100f, bottom = 400f),
          ),
        referenceSlotBoundsByIndex =
          mapOf(
            0 to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            1 to Rect(left = 0f, top = 100f, right = 100f, bottom = 400f),
            2 to Rect(left = 0f, top = 400f, right = 100f, bottom = 500f),
          ),
        referenceItemBoundsByKey =
          mapOf(
            "dragged" to Rect(left = 0f, top = 100f, right = 100f, bottom = 400f),
            "small" to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            "tail" to Rect(left = 0f, top = 400f, right = 100f, bottom = 500f),
          ),
      )

    assertEquals(listOf("dragged", "small", "tail"), reordered)
  }

  @Test
  fun `calculateReorderedKeys does not reverse downward swap while pointer pauses`() {
    val reordered =
      calculateReorderedKeys(
        orderedKeys = listOf("head", "small", "dragged"),
        draggedKey = "dragged",
        comparisonWindowY = 400f,
        itemBounds =
          mapOf(
            "head" to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            "small" to Rect(left = 0f, top = 300f, right = 100f, bottom = 400f),
            "dragged" to Rect(left = 0f, top = 250f, right = 100f, bottom = 550f),
          ),
        referenceSlotBoundsByIndex =
          mapOf(
            0 to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            1 to Rect(left = 0f, top = 100f, right = 100f, bottom = 200f),
            2 to Rect(left = 0f, top = 200f, right = 100f, bottom = 500f),
          ),
        referenceItemBoundsByKey =
          mapOf(
            "head" to Rect(left = 0f, top = 0f, right = 100f, bottom = 100f),
            "small" to Rect(left = 0f, top = 100f, right = 100f, bottom = 200f),
            "dragged" to Rect(left = 0f, top = 200f, right = 100f, bottom = 500f),
          ),
      )

    assertEquals(listOf("head", "small", "dragged"), reordered)
  }

  @Test
  fun `endDrag exposes release translation for dragged item handoff`() = runTest {
    val state = createReorderableListState<String>()
    state.syncKeys(listOf("a", "b", "c"))
    state.registerItemBounds("a", Rect(left = 0f, top = 0f, right = 100f, bottom = 50f))
    state.registerItemBounds("b", Rect(left = 0f, top = 50f, right = 100f, bottom = 100f))
    state.registerItemBounds("c", Rect(left = 0f, top = 100f, right = 100f, bottom = 150f))

    state.beginDrag(key = "a", pointerWindowPosition = Offset(x = 0f, y = 25f))
    state.updateDrag(pointerWindowPosition = Offset(x = 0f, y = 140f))

    state.registerItemBounds("a", Rect(left = 0f, top = 100f, right = 100f, bottom = 150f))

    val commit = state.endDrag()

    assertEquals(15f, state.settlingTranslationY("a"))
    assertEquals(2, commit?.toIndex)
  }

  @Test
  fun `endDrag keeps release translation when item returns to same slot`() = runTest {
    val state = createReorderableListState<String>()
    state.syncKeys(listOf("a", "b"))
    state.registerItemBounds("a", Rect(left = 0f, top = 0f, right = 100f, bottom = 50f))
    state.registerItemBounds("b", Rect(left = 0f, top = 50f, right = 100f, bottom = 100f))

    state.beginDrag(key = "a", pointerWindowPosition = Offset(x = 0f, y = 25f))
    state.updateDrag(pointerWindowPosition = Offset(x = 0f, y = 40f))

    val commit = state.endDrag()

    assertNull(commit)
    assertEquals(15f, state.settlingTranslationY("a"))
  }

  @Test
  fun `noop drag does not discard pending committed order before server catches up`() = runTest {
    val state = createReorderableListState<String>()
    val serverKeys = listOf("a", "b", "c")
    val reorderedKeys = listOf("b", "c", "a")

    state.syncKeys(serverKeys)
    state.registerItemBounds("a", Rect(left = 0f, top = 0f, right = 100f, bottom = 50f))
    state.registerItemBounds("b", Rect(left = 0f, top = 50f, right = 100f, bottom = 100f))
    state.registerItemBounds("c", Rect(left = 0f, top = 100f, right = 100f, bottom = 150f))

    state.beginDrag(key = "a", pointerWindowPosition = Offset(x = 0f, y = 25f))
    state.updateDrag(pointerWindowPosition = Offset(x = 0f, y = 140f))
    state.registerItemBounds("a", Rect(left = 0f, top = 100f, right = 100f, bottom = 150f))
    assertEquals(reorderedKeys, state.endDrag()?.orderedKeys)

    state.syncKeys(serverKeys)
    assertEquals(reorderedKeys, state.displayedKeys)

    state.registerItemBounds("b", Rect(left = 0f, top = 0f, right = 100f, bottom = 50f))
    state.registerItemBounds("c", Rect(left = 0f, top = 50f, right = 100f, bottom = 100f))
    state.registerItemBounds("a", Rect(left = 0f, top = 100f, right = 100f, bottom = 150f))
    state.beginDrag(key = "a", pointerWindowPosition = Offset(x = 0f, y = 125f))
    assertNull(state.endDrag())

    state.syncKeys(serverKeys)
    assertEquals(reorderedKeys, state.displayedKeys)
  }
}

private fun <K : Any> createReorderableListState(): ReorderableListState<K> {
  return ReorderableListState(autoScrollController = createAutoScrollController())
}

private fun createAutoScrollController(): co.typie.ext.AutoScrollController {
  return co.typie.ext.AutoScrollController(
    verticalScrollableState = null,
    horizontalScrollableState = null,
  )
}
