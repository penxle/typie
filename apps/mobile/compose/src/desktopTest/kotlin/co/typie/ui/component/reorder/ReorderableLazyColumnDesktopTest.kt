package co.typie.ui.component.reorder

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class ReorderableLazyColumnDesktopTest {
  @Test
  fun dragHandleKeepsThePointerInWindowCoordinatesWhileTheItemMoves() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>

    setContent {
      state = rememberReorderableLazyColumnState(keys = listOf("a", "b"))
      Box(Modifier.padding(top = 80.dp)) {
        ReorderableLazyColumn(
          state = state,
          modifier = Modifier.size(width = 100.dp, height = 120.dp),
          contentPadding = PaddingValues(top = 20.dp),
        ) {
          item(key = "header") { Box(Modifier.fillMaxWidth().height(40.dp)) }
          items(state.keys, key = { it }) { itemKey ->
            Box(
              reorderableAnimatedItem(
                  state = state,
                  key = itemKey,
                  modifier = Modifier.fillMaxWidth().height(40.dp),
                )
                .reorderableItem(state = state, key = itemKey)
                .testTag(itemKey)
            ) {
              Box(
                Modifier.fillMaxSize()
                  .reorderableDragHandle(state = state, key = itemKey)
                  .testTag("handle-$itemKey")
              )
            }
          }
        }
      }
    }
    waitForIdle()

    val handle = onNodeWithTag("handle-a")
    val initialItemTop = onNodeWithTag("a").fetchSemanticsNode().boundsInRoot.top
    handle.performTouchInput { down(center) }
    waitForIdle()
    assertEquals(0f, state.draggedOffsetY("a"), absoluteTolerance = 0.5f)

    handle.performTouchInput { moveBy(Offset(x = 0f, y = 10f)) }
    waitForIdle()
    assertEquals(10f, state.draggedOffsetY("a"), absoluteTolerance = 0.5f)

    handle.performTouchInput { moveBy(Offset(x = 0f, y = 40f)) }
    waitForIdle()
    assertEquals(listOf("b", "a"), state.keys)
    assertEquals(
      initialItemTop + 50f,
      onNodeWithTag("a").fetchSemanticsNode().boundsInRoot.top,
      absoluteTolerance = 0.5f,
    )

    handle.performTouchInput { up() }
    waitForIdle()
  }

  @Test
  fun edgeAutoScrollKeepsAdvancingTheDraggedSlotWithoutMorePointerEvents() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>
    val keys = (0 until 100).map(Int::toString)

    setContent {
      state = rememberReorderableLazyColumnState(keys = keys)
      ReorderableLazyColumn(
        state = state,
        modifier = Modifier.size(width = 100.dp, height = 120.dp).reorderableViewport(state = state),
      ) {
        items(state.keys, key = { it }) { itemKey ->
          Box(
            reorderableAnimatedItem(
                state = state,
                key = itemKey,
                modifier = Modifier.fillMaxWidth().height(40.dp),
              )
              .reorderableItem(state = state, key = itemKey)
              .testTag(itemKey)
          ) {
            Box(
              Modifier.fillMaxSize()
                .reorderableDragHandle(state = state, key = itemKey)
                .testTag("handle-$itemKey")
            )
          }
        }
      }
    }
    waitForIdle()
    mainClock.autoAdvance = false

    val handle = onNodeWithTag("handle-0")
    handle.performTouchInput { down(center) }
    mainClock.advanceTimeByFrame()
    handle.performTouchInput { moveBy(Offset(x = 0f, y = 200f)) }
    val targetIndices = buildList {
      repeat(6) {
        repeat(12) { mainClock.advanceTimeByFrame() }
        add(state.keys.indexOf("0"))
      }
    }

    assertTrue(
      state.lazyListState.firstVisibleItemIndex > 0,
      "Edge auto-scroll did not move the lazy viewport",
    )
    assertTrue(
      targetIndices.zipWithNext().all { (previous, next) -> next > previous },
      "Dragged slot stalled during edge auto-scroll: $targetIndices",
    )

    handle.performTouchInput { up() }
    repeat(3) { mainClock.advanceTimeByFrame() }
    mainClock.autoAdvance = true
  }

  @Test
  fun adjacentItemNeverMovesAwayFromItsDestinationWhenTheSlotChanges() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>

    setContent {
      state = rememberReorderableLazyColumnState(keys = listOf("a", "b", "c"))
      ReorderableLazyColumn(
        state = state,
        modifier = Modifier.size(width = 100.dp, height = 120.dp),
      ) {
        items(state.keys, key = { it }) { itemKey ->
          Box(
            reorderableAnimatedItem(
                state = state,
                key = itemKey,
                modifier = Modifier.fillMaxWidth().height(40.dp),
              )
              .reorderableItem(state = state, key = itemKey)
              .testTag(itemKey)
          ) {
            Box(
              Modifier.fillMaxSize()
                .reorderableDragHandle(state = state, key = itemKey)
                .testTag("handle-$itemKey")
            )
          }
        }
      }
    }
    waitForIdle()
    val initialSiblingTop = onNodeWithTag("b").fetchSemanticsNode().boundsInRoot.top
    mainClock.autoAdvance = false

    val handle = onNodeWithTag("handle-a")
    handle.performTouchInput { down(center) }
    mainClock.advanceTimeByFrame()
    handle.performTouchInput { moveBy(Offset(x = 0f, y = 25f)) }
    assertEquals(listOf("b", "a", "c"), state.keys)

    val siblingTops = buildList {
      repeat(20) {
        mainClock.advanceTimeByFrame()
        add(onNodeWithTag("b").fetchSemanticsNode().boundsInRoot.top)
      }
    }
    val destinationTop = initialSiblingTop - 40f
    assertTrue(
      siblingTops.all { it in (destinationTop - 0.5f)..(initialSiblingTop + 0.5f) },
      "Sibling left the path between its start and destination: $siblingTops",
    )
    handle.performTouchInput { up() }
    repeat(3) { mainClock.advanceTimeByFrame() }
    mainClock.autoAdvance = true
  }

  @Test
  fun draggedItemStaysUnderThePointerOnTheFirstFrameAfterItsSlotChanges() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>

    setContent {
      state = rememberReorderableLazyColumnState(keys = listOf("a", "b"))
      ReorderableLazyColumn(
        state = state,
        modifier = Modifier.size(width = 100.dp, height = 120.dp),
      ) {
        items(state.keys, key = { it }) { itemKey ->
          Box(
            reorderableAnimatedItem(
                state = state,
                key = itemKey,
                modifier = Modifier.fillMaxWidth().height(40.dp),
              )
              .reorderableItem(state = state, key = itemKey)
              .testTag(itemKey)
          ) {
            Box(
              Modifier.fillMaxSize()
                .reorderableDragHandle(state = state, key = itemKey)
                .testTag("handle-$itemKey")
            )
          }
        }
      }
    }
    waitForIdle()
    val initialTop = onNodeWithTag("a").fetchSemanticsNode().boundsInRoot.top
    mainClock.autoAdvance = false

    val handle = onNodeWithTag("handle-a")
    handle.performTouchInput { down(center) }
    mainClock.advanceTimeByFrame()
    handle.performTouchInput { moveBy(Offset(x = 0f, y = 25f)) }
    assertEquals(listOf("b", "a"), state.keys)

    mainClock.advanceTimeByFrame()
    assertEquals(
      initialTop + 25f,
      onNodeWithTag("a").fetchSemanticsNode().boundsInRoot.top,
      absoluteTolerance = 0.5f,
    )

    handle.performTouchInput { up() }
    repeat(3) { mainClock.advanceTimeByFrame() }
    mainClock.autoAdvance = true
  }

  @Test
  fun draggedItemTracksThePointerBeforeCrossingAnotherSlot() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>
    val itemBounds = mutableMapOf<String, Rect>()

    setContent {
      state = rememberReorderableLazyColumnState(keys = listOf("a", "b"))
      ReorderableLazyColumn(
        state = state,
        modifier =
          Modifier.size(width = 100.dp, height = 120.dp).background(Color.White).testTag("list"),
      ) {
        items(state.keys, key = { it }) { itemKey ->
          Box(
            modifier =
              Modifier.fillMaxWidth()
                .height(40.dp)
                .reorderableItem(state = state, key = itemKey)
                .onGloballyPositioned { coordinates ->
                  val origin = coordinates.positionInWindow()
                  itemBounds[itemKey] =
                    Rect(
                      left = origin.x,
                      top = origin.y,
                      right = origin.x + coordinates.size.width,
                      bottom = origin.y + coordinates.size.height,
                    )
                }
                .testTag(itemKey)
          ) {
            Box(Modifier.fillMaxSize().background(if (itemKey == "a") Color.Red else Color.Blue))
          }
        }
      }
    }
    waitForIdle()

    val initialBounds = assertNotNull(itemBounds["a"])
    val pointer = initialBounds.center

    runOnUiThread {
      assertTrue(
        state.beginDrag(
          key = "a",
          pointer = pointer,
          onTargetChanged = {},
          onTargetHaptic = {},
          onDragStopped = {},
        )
      )
      state.updatePointer(pointer + Offset(x = 0f, y = 10f))
    }
    waitForIdle()

    assertEquals(10f, state.draggedOffsetY("a"), absoluteTolerance = 0.5f)
    val list = onNodeWithTag("list")
    val listBounds = list.fetchSemanticsNode().boundsInRoot
    val pixels = list.captureToImage().toPixelMap()
    val sampleX = (initialBounds.center.x - listBounds.left).roundToInt()
    val oldTopSampleY = (initialBounds.top - listBounds.top + 2f).roundToInt()
    assertEquals(Color.White, pixels[sampleX, oldTopSampleY])
    assertEquals(Color.Red, pixels[sampleX, oldTopSampleY + 10])
    runOnUiThread { state.cancel() }
  }

  @Test
  fun movingTheFirstVisibleKeyKeepsTheRequestedViewportPosition() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>
    val itemBounds = mutableMapOf<String, Rect>()

    setContent {
      val lazyListState =
        rememberLazyListState(
          initialFirstVisibleItemIndex = 2,
          initialFirstVisibleItemScrollOffset = 10,
        )
      state =
        rememberReorderableLazyColumnState(
          keys = listOf("a", "b", "c", "d", "e", "f"),
          lazyListState = lazyListState,
        )

      ReorderableLazyColumn(
        state = state,
        modifier = Modifier.size(width = 100.dp, height = 120.dp),
      ) {
        items(state.keys, key = { it }) { itemKey ->
          Box(
            reorderableAnimatedItem(
                state = state,
                key = itemKey,
                modifier = Modifier.fillMaxWidth().height(40.dp),
              )
              .reorderableItem(state = state, key = itemKey)
              .onGloballyPositioned { coordinates ->
                val origin = coordinates.positionInWindow()
                itemBounds[itemKey] =
                  Rect(
                    left = origin.x,
                    top = origin.y,
                    right = origin.x + coordinates.size.width,
                    bottom = origin.y + coordinates.size.height,
                  )
              }
          )
        }
      }
    }
    waitForIdle()

    assertEquals(2, state.lazyListState.firstVisibleItemIndex)
    assertEquals(10, state.lazyListState.firstVisibleItemScrollOffset)
    val draggedBounds = assertNotNull(itemBounds["c"])
    val pointer = draggedBounds.center

    runOnUiThread {
      assertTrue(
        state.beginDrag(
          key = "c",
          pointer = pointer,
          onTargetChanged = {},
          onTargetHaptic = {},
          onDragStopped = {},
        )
      )
      state.updatePointer(pointer + Offset(x = 0f, y = 30f))
      assertEquals(listOf("a", "b", "d", "c", "e", "f"), state.keys)
      state.release()
    }
    waitForIdle()

    assertEquals(2, state.lazyListState.firstVisibleItemIndex)
    assertEquals(10, state.lazyListState.firstVisibleItemScrollOffset)
  }

  @Test
  fun movingAnItemAboveTheFirstVisibleKeyKeepsTheVisibleSlotAnchored() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>
    val itemBounds = mutableMapOf<String, Rect>()

    setContent {
      val lazyListState =
        rememberLazyListState(
          initialFirstVisibleItemIndex = 2,
          initialFirstVisibleItemScrollOffset = 10,
        )
      state =
        rememberReorderableLazyColumnState(
          keys = listOf("a", "b", "c", "d", "e", "f"),
          lazyListState = lazyListState,
        )

      ReorderableLazyColumn(
        state = state,
        modifier = Modifier.size(width = 100.dp, height = 120.dp),
      ) {
        items(state.keys, key = { it }) { itemKey ->
          Box(
            reorderableAnimatedItem(
                state = state,
                key = itemKey,
                modifier = Modifier.fillMaxWidth().height(40.dp),
              )
              .reorderableItem(state = state, key = itemKey)
              .onGloballyPositioned { coordinates ->
                val origin = coordinates.positionInWindow()
                itemBounds[itemKey] =
                  Rect(
                    left = origin.x,
                    top = origin.y,
                    right = origin.x + coordinates.size.width,
                    bottom = origin.y + coordinates.size.height,
                  )
              }
          )
        }
      }
    }
    waitForIdle()

    val draggedBounds = assertNotNull(itemBounds["e"])
    val pointer = draggedBounds.center
    runOnUiThread {
      assertTrue(
        state.beginDrag(
          key = "e",
          pointer = pointer,
          onTargetChanged = {},
          onTargetHaptic = {},
          onDragStopped = {},
        )
      )
      state.updatePointer(pointer + Offset(x = 0f, y = -90f))
      assertEquals(listOf("a", "b", "e", "c", "d", "f"), state.keys)
    }
    waitForIdle()

    assertEquals(2, state.lazyListState.firstVisibleItemIndex)
    assertEquals(10, state.lazyListState.firstVisibleItemScrollOffset)
    assertEquals("e", state.lazyListState.layoutInfo.visibleItemsInfo.first().key)
    runOnUiThread { state.cancel() }
  }
}
