package co.typie.ui.component.reorder

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.test.runTest

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
}

private fun <K : Any> createReorderableListState(): ReorderableListState<K> {
  return ReorderableListState(edgeAutoScrollState = createEdgeAutoScrollState())
}

private fun createEdgeAutoScrollState(): co.typie.ext.EdgeAutoScrollState {
  return co.typie.ext.EdgeAutoScrollState(
    scope = CoroutineScope(SupervisorJob() + Dispatchers.Default),
    edgeThresholdPx = 0f,
    minScrollSpeedPx = 0f,
    maxScrollSpeedPx = 0f,
    frameDurationMs = 16L,
  )
}
