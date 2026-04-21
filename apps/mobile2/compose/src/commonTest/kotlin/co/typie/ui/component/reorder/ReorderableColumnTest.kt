package co.typie.ui.component.reorder

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.coroutines.test.runTest

class ReorderableColumnTest {
  @Test
  fun `draggedCenterY returns midpoint of dragged item`() {
    assertEquals(
      110f,
      draggedCenterY(pointerY = 130f, pointerOffsetInItemY = 45f, itemHeight = 50f),
    )
  }

  @Test
  fun `reorderedKeysForDrag waits until dragged item covers half of next slot`() {
    val result =
      reorderedKeysForDrag(
        keys = listOf("dragged", "next", "tail"),
        draggedKey = "dragged",
        direction = 1,
        slotBounds =
          mapOf(
            "dragged" to Rect(0f, 20f, 100f, 80f),
            "next" to Rect(0f, 60f, 100f, 160f),
            "tail" to Rect(0f, 160f, 100f, 200f),
          ),
      )
    assertEquals(listOf("dragged", "next", "tail"), result)
  }

  @Test
  fun `reorderedKeysForDrag swaps once dragged item covers half of next slot`() {
    val result =
      reorderedKeysForDrag(
        keys = listOf("dragged", "next", "tail"),
        draggedKey = "dragged",
        direction = 1,
        slotBounds =
          mapOf(
            "dragged" to Rect(0f, 50f, 100f, 110f),
            "next" to Rect(0f, 60f, 100f, 160f),
            "tail" to Rect(0f, 160f, 100f, 200f),
          ),
      )
    assertEquals(listOf("next", "dragged", "tail"), result)
  }

  @Test
  fun `reorderedKeysForDrag waits until dragged item covers half of previous slot`() {
    val result =
      reorderedKeysForDrag(
        keys = listOf("head", "dragged", "tail"),
        draggedKey = "dragged",
        direction = -1,
        slotBounds =
          mapOf(
            "head" to Rect(0f, 0f, 100f, 100f),
            "dragged" to Rect(0f, 70f, 100f, 130f),
            "tail" to Rect(0f, 160f, 100f, 200f),
          ),
      )
    assertEquals(listOf("head", "dragged", "tail"), result)
  }

  @Test
  fun `reorderedKeysForDrag swaps once dragged item covers half of previous slot`() {
    val result =
      reorderedKeysForDrag(
        keys = listOf("head", "dragged", "tail"),
        draggedKey = "dragged",
        direction = -1,
        slotBounds =
          mapOf(
            "head" to Rect(0f, 0f, 100f, 100f),
            "dragged" to Rect(0f, 50f, 100f, 110f),
            "tail" to Rect(0f, 160f, 100f, 200f),
          ),
      )
    assertEquals(listOf("dragged", "head", "tail"), result)
  }

  @Test
  fun `reorderedKeysForDrag uses smaller item height as threshold when items differ in size`() {
    val result =
      reorderedKeysForDrag(
        keys = listOf("dragged", "large", "tail"),
        draggedKey = "dragged",
        direction = 1,
        slotBounds =
          mapOf(
            "dragged" to Rect(0f, 85f, 100f, 145f),
            "large" to Rect(0f, 100f, 100f, 300f),
            "tail" to Rect(0f, 300f, 100f, 360f),
          ),
      )
    assertEquals(listOf("large", "dragged", "tail"), result)
  }

  @Test
  fun `reorderedKeysForDrag does not oscillate when drag pauses after swap`() {
    val result =
      reorderedKeysForDrag(
        keys = listOf("a", "c", "b", "d"),
        draggedKey = "b",
        direction = 0,
        slotBounds =
          mapOf(
            "a" to Rect(0f, 0f, 100f, 50f),
            "c" to Rect(0f, 100f, 100f, 150f),
            "b" to Rect(0f, 50f, 100f, 100f),
            "d" to Rect(0f, 150f, 100f, 200f),
          ),
      )
    assertEquals(listOf("a", "c", "b", "d"), result)
  }

  @Test
  fun `reorderedKeysForDrag returns null when dragged bounds missing`() {
    val result =
      reorderedKeysForDrag(
        keys = listOf("a", "b"),
        draggedKey = "a",
        direction = 0,
        slotBounds = mapOf("b" to Rect(0f, 50f, 100f, 100f)),
      )
    assertEquals(null, result)
  }
}

class ReorderableColumnStateTest {
  @Test
  fun `keys reorder while dragging based on pointer position alone`() = runTest {
    val state = createState<String>()
    state.inputKeys = listOf("a", "b", "c")
    state.registerSlotBounds("a", Rect(0f, 0f, 100f, 50f))
    state.registerSlotBounds("b", Rect(0f, 50f, 100f, 100f))
    state.registerSlotBounds("c", Rect(0f, 100f, 100f, 150f))

    state.beginDrag("a", Offset(0f, 25f))
    state.updateDrag(Offset(0f, 140f))

    assertEquals(listOf("b", "c", "a"), state.keys)
  }

  @Test
  fun `direction flip near boundary does not oscillate rendered order`() = runTest {
    val state = createState<String>()
    state.inputKeys = listOf("a", "b", "c")
    state.registerSlotBounds("a", Rect(0f, 0f, 100f, 50f))
    state.registerSlotBounds("b", Rect(0f, 50f, 100f, 100f))
    state.registerSlotBounds("c", Rect(0f, 100f, 100f, 150f))

    state.beginDrag("a", Offset(0f, 25f))
    state.updateDrag(Offset(0f, 140f))

    state.registerSlotBounds("a", Rect(0f, 100f, 100f, 150f))
    state.registerSlotBounds("b", Rect(0f, 0f, 100f, 50f))
    state.registerSlotBounds("c", Rect(0f, 50f, 100f, 100f))

    assertEquals(listOf("b", "c", "a"), state.keys)

    state.updateDrag(Offset(0f, 139f))
    assertEquals(listOf("b", "c", "a"), state.keys)

    state.updateDrag(Offset(0f, 140f))
    assertEquals(listOf("b", "c", "a"), state.keys)
  }

  @Test
  fun `slot bounds update can reorder without additional pointer movement`() = runTest {
    val state = createState<String>()
    state.inputKeys = listOf("a", "b", "c")
    state.registerSlotBounds("a", Rect(0f, 0f, 100f, 50f))
    state.registerSlotBounds("b", Rect(0f, 50f, 100f, 100f))
    state.registerSlotBounds("c", Rect(0f, 100f, 100f, 150f))

    state.beginDrag("a", Offset(0f, 25f))
    state.updateDrag(Offset(0f, 40f))

    assertEquals(listOf("a", "b", "c"), state.keys)

    state.registerSlotBounds("a", Rect(0f, -20f, 100f, 30f))
    state.registerSlotBounds("b", Rect(0f, 30f, 100f, 80f))
    state.registerSlotBounds("c", Rect(0f, 80f, 100f, 130f))

    assertEquals(listOf("b", "a", "c"), state.keys)
  }

  @Test
  fun `endDrag exposes release translation for dragged item handoff`() = runTest {
    val state = createState<String>()
    state.inputKeys = listOf("a", "b", "c")
    state.registerSlotBounds("a", Rect(0f, 0f, 100f, 50f))
    state.registerSlotBounds("b", Rect(0f, 50f, 100f, 100f))
    state.registerSlotBounds("c", Rect(0f, 100f, 100f, 150f))

    state.beginDrag("a", Offset(0f, 25f))
    state.updateDrag(Offset(0f, 140f))

    state.registerSlotBounds("a", Rect(0f, 100f, 100f, 150f))

    val drop = state.endDrag()

    assertEquals(15f, state.settlingOffsetY("a"))
    assertEquals(2, drop?.toIndex)
  }

  @Test
  fun `endDrag keeps release translation when item returns to same slot`() = runTest {
    val state = createState<String>()
    state.inputKeys = listOf("a", "b")
    state.registerSlotBounds("a", Rect(0f, 0f, 100f, 50f))
    state.registerSlotBounds("b", Rect(0f, 50f, 100f, 100f))

    state.beginDrag("a", Offset(0f, 25f))
    state.updateDrag(Offset(0f, 40f))

    val drop = state.endDrag()

    assertNull(drop)
    assertEquals(15f, state.settlingOffsetY("a"))
  }

  @Test
  fun `cancelDrag clears active drag and autoscroll pointer`() = runTest {
    val state = createState<String>()
    state.inputKeys = listOf("a", "b")
    state.registerSlotBounds("a", Rect(0f, 0f, 100f, 50f))
    state.registerSlotBounds("b", Rect(0f, 50f, 100f, 100f))

    state.beginDrag("a", Offset(0f, 25f))
    state.autoScrollController.pointer = Offset(0f, 25f)
    state.cancelDrag()

    assertNull(state.draggingKey)
    assertNull(state.autoScrollController.pointer)
  }
}

private fun <K : Any> createState(): ReorderableColumnState<K> =
  ReorderableColumnState(
    autoScrollController =
      co.typie.ext.AutoScrollController(
        verticalScrollableState = null,
        horizontalScrollableState = null,
      )
  )
