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
import androidx.compose.ui.test.SemanticsNodeInteraction
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
          items(state.layoutKeys, key = { it }) { itemKey ->
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
        modifier =
          Modifier.size(width = 100.dp, height = 120.dp)
            .background(Color.White)
            .testTag("edge-autoscroll-list")
            .reorderableViewport(state = state),
      ) {
        items(state.layoutKeys, key = { it }) { itemKey ->
          Box(
            reorderableAnimatedItem(
                state = state,
                key = itemKey,
                modifier = Modifier.fillMaxWidth().height(40.dp),
              )
              .reorderableItem(state = state, key = itemKey)
              .background(if (itemKey == "0") Color.Red else Color.Blue)
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
    handle.performTouchInput { moveBy(Offset(x = 0f, y = 75f)) }
    val targetIndices = buildList {
      repeat(4) {
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

    val list = onNodeWithTag("edge-autoscroll-list")
    val redRowsBeforeRelease = list.capturedColorRows(Color.Red)
    assertTrue(redRowsBeforeRelease.isNotEmpty())
    handle.performTouchInput { up() }
    var redRowsAfterFirstReleaseFrame = emptyList<Int>()
    repeat(4) {
      mainClock.advanceTimeByFrame()
      if (!state.isDragging && redRowsAfterFirstReleaseFrame.isEmpty()) {
        redRowsAfterFirstReleaseFrame = list.capturedColorRows(Color.Red)
      }
    }
    repeat(90) { mainClock.advanceTimeByFrame() }
    val redRowsAfterSettling = list.capturedColorRows(Color.Red)
    val expectedFirstRowRange =
      minOf(redRowsBeforeRelease.first(), redRowsAfterSettling.first()) - 1..maxOf(
          redRowsBeforeRelease.first(),
          redRowsAfterSettling.first(),
        ) + 1
    assertTrue(
      redRowsAfterFirstReleaseFrame.firstOrNull()?.let { it in expectedFirstRowRange } == true,
      "Dragged pixels jumped on far release: before=$redRowsBeforeRelease, first=$redRowsAfterFirstReleaseFrame, settled=$redRowsAfterSettling",
    )
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
        items(state.layoutKeys, key = { it }) { itemKey ->
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
        items(state.layoutKeys, key = { it }) { itemKey ->
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
        items(state.layoutKeys, key = { it }) { itemKey ->
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
        items(state.layoutKeys, key = { it }) { itemKey ->
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
              .testTag(itemKey)
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
  fun upwardReorderInsideContentPaddingKeepsTheContentCoordinate() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>
    val itemBounds = mutableMapOf<String, Rect>()
    val keys = (0 until 10).map(Int::toString)
    val draggedKey = "6"

    setContent {
      val lazyListState =
        rememberLazyListState(
          initialFirstVisibleItemIndex = 5,
          initialFirstVisibleItemScrollOffset = 50,
        )
      state = rememberReorderableLazyColumnState(keys = keys, lazyListState = lazyListState)

      ReorderableLazyColumn(
        state = state,
        modifier = Modifier.size(width = 100.dp, height = 240.dp),
        contentPadding = PaddingValues(top = 40.dp),
      ) {
        items(state.layoutKeys, key = { it }) { itemKey ->
          Box(
            reorderableAnimatedItem(
                state = state,
                key = itemKey,
                modifier =
                  Modifier.fillMaxWidth().height(if (itemKey == draggedKey) 120.dp else 40.dp),
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
              .testTag("padding-$itemKey")
          )
        }
      }
    }
    waitForIdle()

    val layoutBefore = state.lazyListState.layoutInfo
    assertEquals(6, state.lazyListState.firstVisibleItemIndex)
    val paddingItem = assertNotNull(layoutBefore.visibleItemsInfo.firstOrNull { it.key == "5" })
    assertTrue(paddingItem.offset < 0)
    assertTrue(paddingItem.offset + paddingItem.size <= 0)
    val unaffectedOffsetBefore =
      assertNotNull(layoutBefore.visibleItemsInfo.firstOrNull { it.key == "7" }).offset
    val crossedSiblingTopBefore = onNodeWithTag("padding-5").fetchSemanticsNode().boundsInRoot.top
    val draggedBounds = assertNotNull(itemBounds[draggedKey])
    val viewport = assertNotNull(state.viewport)
    val pointer = Offset(x = draggedBounds.center.x, y = draggedBounds.top + 60f)
    val paddingPointer = Offset(x = pointer.x, y = viewport.top + 29f)
    mainClock.autoAdvance = false

    runOnUiThread {
      assertTrue(
        state.beginDrag(
          key = draggedKey,
          pointer = pointer,
          onTargetChanged = {},
          onTargetHaptic = {},
          onDragStopped = {},
        )
      )
      state.updatePointer(paddingPointer)
      assertEquals(listOf("0", "1", "2", "3", "4", "6", "5", "7", "8", "9"), state.keys)
      assertEquals(keys, state.layoutKeys)
    }
    mainClock.advanceTimeByFrame()

    val layoutAfter = state.lazyListState.layoutInfo
    assertEquals("5", layoutAfter.visibleItemsInfo.firstOrNull { it.index == 5 }?.key)
    assertEquals("6", layoutAfter.visibleItemsInfo.firstOrNull { it.index == 6 }?.key)
    val unaffectedOffsetAfter =
      assertNotNull(layoutAfter.visibleItemsInfo.firstOrNull { it.key == "7" }).offset
    assertEquals(unaffectedOffsetBefore, unaffectedOffsetAfter)
    assertTrue(
      onNodeWithTag("padding-5").fetchSemanticsNode().boundsInRoot.top <
        crossedSiblingTopBefore + 100f,
      "Crossed sibling snapped to the 120px destination on the first frame",
    )
    val draggedVisualTopBeforeRelease = assertNotNull(itemBounds[draggedKey]).top
    runOnUiThread { state.release() }
    val initialReleaseOffset = assertNotNull(state.settlingOffsetY(draggedKey))
    assertEquals(
      draggedVisualTopBeforeRelease,
      viewport.top + layoutBefore.beforeContentPadding + paddingItem.offset + initialReleaseOffset,
      1f,
    )
    mainClock.advanceTimeByFrame()

    assertEquals(listOf("0", "1", "2", "3", "4", "6", "5", "7", "8", "9"), state.layoutKeys)
    val draggedOffsetAfterRelease =
      assertNotNull(
          state.lazyListState.layoutInfo.visibleItemsInfo.firstOrNull { it.key == draggedKey }
        )
        .offset
    val unaffectedOffsetAfterRelease =
      assertNotNull(state.lazyListState.layoutInfo.visibleItemsInfo.firstOrNull { it.key == "7" })
        .offset
    assertEquals(paddingItem.offset, draggedOffsetAfterRelease)
    assertEquals(unaffectedOffsetBefore, unaffectedOffsetAfterRelease)
    repeat(90) { mainClock.advanceTimeByFrame() }
    mainClock.autoAdvance = true
  }

  @Test
  fun movedReleaseKeepsTallDraggedItemContinuousThenSettlesIntoItsSlot() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>

    setContent {
      state = rememberReorderableLazyColumnState(keys = listOf("a", "b", "c"))
      ReorderableLazyColumn(
        state = state,
        modifier = Modifier.size(width = 100.dp, height = 200.dp),
      ) {
        items(state.layoutKeys, key = { it }) { itemKey ->
          Box(
            reorderableAnimatedItem(
                state = state,
                key = itemKey,
                modifier = Modifier.fillMaxWidth().height(if (itemKey == "a") 80.dp else 40.dp),
              )
              .reorderableItem(state = state, key = itemKey)
              .testTag(itemKey)
          )
        }
      }
    }
    waitForIdle()
    val initialTop = onNodeWithTag("a").fetchSemanticsNode().boundsInRoot.top
    val pointer = Offset(x = 50f, y = initialTop + 40f)
    mainClock.autoAdvance = false

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
      state.updatePointer(pointer + Offset(x = 0f, y = 45f))
      assertEquals(listOf("b", "a", "c"), state.keys)
      assertEquals(listOf("a", "b", "c"), state.layoutKeys)
    }
    mainClock.advanceTimeByFrame()
    val topBeforeRelease = onNodeWithTag("a").fetchSemanticsNode().boundsInRoot.top
    val siblingTopBeforeRelease = onNodeWithTag("b").fetchSemanticsNode().boundsInRoot.top

    runOnUiThread { state.release() }
    mainClock.advanceTimeByFrame()

    assertEquals(listOf("b", "a", "c"), state.layoutKeys)
    assertEquals("b", state.lazyListState.layoutInfo.visibleItemsInfo.first { it.index == 0 }.key)
    assertEquals(topBeforeRelease, onNodeWithTag("a").fetchSemanticsNode().boundsInRoot.top, 1f)
    assertEquals(
      siblingTopBeforeRelease,
      onNodeWithTag("b").fetchSemanticsNode().boundsInRoot.top,
      1f,
    )

    repeat(90) { mainClock.advanceTimeByFrame() }
    assertEquals(initialTop + 40f, onNodeWithTag("a").fetchSemanticsNode().boundsInRoot.top, 1f)
    mainClock.autoAdvance = true
  }

  @Test
  fun movedReleaseCapturesTheFinalTargetBeforeTheNextCompositionFrame() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>

    setContent {
      state = rememberReorderableLazyColumnState(keys = listOf("a", "b", "c"))
      ReorderableLazyColumn(
        state = state,
        modifier = Modifier.size(width = 100.dp, height = 200.dp),
      ) {
        items(state.layoutKeys, key = { it }) { itemKey ->
          Box(
            reorderableAnimatedItem(
                state = state,
                key = itemKey,
                modifier = Modifier.fillMaxWidth().height(if (itemKey == "a") 80.dp else 40.dp),
              )
              .reorderableItem(state = state, key = itemKey)
              .testTag("immediate-$itemKey")
          )
        }
      }
    }
    waitForIdle()
    val draggedInitialTop = onNodeWithTag("immediate-a").fetchSemanticsNode().boundsInRoot.top
    val siblingInitialTop = onNodeWithTag("immediate-b").fetchSemanticsNode().boundsInRoot.top
    val pointer = Offset(x = 50f, y = draggedInitialTop + 40f)
    mainClock.autoAdvance = false

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
      state.updatePointer(pointer + Offset(x = 0f, y = 45f))
      assertEquals(listOf("b", "a", "c"), state.keys)
      state.release()
    }
    mainClock.advanceTimeByFrame()

    assertEquals(listOf("b", "a", "c"), state.layoutKeys)
    assertEquals(
      draggedInitialTop + 45f,
      onNodeWithTag("immediate-a").fetchSemanticsNode().boundsInRoot.top,
      1f,
    )
    assertEquals(
      siblingInitialTop,
      onNodeWithTag("immediate-b").fetchSemanticsNode().boundsInRoot.top,
      1f,
    )

    repeat(90) { mainClock.advanceTimeByFrame() }
    assertEquals(
      draggedInitialTop + 40f,
      onNodeWithTag("immediate-a").fetchSemanticsNode().boundsInRoot.top,
      1f,
    )
    mainClock.autoAdvance = true
  }

  @Test
  fun movedReleaseAnchorsTheProjectedSlotWithVariableHeights() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>
    val itemBounds = mutableMapOf<String, Rect>()
    val keys = listOf("a", "b", "c", "d", "e")
    val draggedKey = "a"

    setContent {
      val lazyListState = rememberLazyListState()
      state = rememberReorderableLazyColumnState(keys = keys, lazyListState = lazyListState)
      ReorderableLazyColumn(
        state = state,
        modifier =
          Modifier.size(width = 100.dp, height = 160.dp)
            .background(Color.White)
            .testTag("projected-anchor-list"),
      ) {
        items(state.layoutKeys, key = { it }) { itemKey ->
          val itemHeight =
            when (itemKey) {
              draggedKey -> 80.dp
              "c" -> 100.dp
              else -> 40.dp
            }
          Box(
            reorderableAnimatedItem(
                state = state,
                key = itemKey,
                modifier = Modifier.fillMaxWidth().height(itemHeight),
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
              .background(if (itemKey == draggedKey) Color.Red else Color.Blue)
          )
        }
      }
    }
    waitForIdle()
    val draggedBounds = assertNotNull(itemBounds[draggedKey])
    val viewport = assertNotNull(state.viewport)
    val pointer = Offset(x = draggedBounds.center.x, y = draggedBounds.top + 40f)
    mainClock.autoAdvance = false

    runOnUiThread {
      assertTrue(
        state.beginDrag(
          key = draggedKey,
          pointer = pointer,
          onTargetChanged = {},
          onTargetHaptic = {},
          onDragStopped = {},
        )
      )
      state.lazyListState.requestScrollToItem(index = 1, scrollOffset = 20)
    }
    repeat(2) { mainClock.advanceTimeByFrame() }
    runOnUiThread { state.updatePointer(Offset(x = pointer.x, y = viewport.top + 110f)) }
    assertEquals(listOf("b", "c", "d", "a", "e"), state.keys)
    mainClock.advanceTimeByFrame()
    val sourceCOffset =
      assertNotNull(state.lazyListState.layoutInfo.visibleItemsInfo.firstOrNull { it.key == "c" })
        .offset
    val expectedProjectedCOffset = sourceCOffset - 80
    val redRowsBeforeRelease = onNodeWithTag("projected-anchor-list").capturedColorRows(Color.Red)

    runOnUiThread { state.release() }
    assertEquals(-10f, assertNotNull(state.settlingOffsetY(draggedKey)), 1f)
    mainClock.advanceTimeByFrame()
    val redRowsAfterFirstReleaseFrame =
      onNodeWithTag("projected-anchor-list").capturedColorRows(Color.Red)
    val committedCOffset =
      assertNotNull(state.lazyListState.layoutInfo.visibleItemsInfo.firstOrNull { it.key == "c" })
        .offset
    val committedDraggedOffsets =
      state.lazyListState.layoutInfo.visibleItemsInfo
        .filter { it.key == draggedKey }
        .map { it.offset }

    assertEquals(expectedProjectedCOffset, committedCOffset)
    assertEquals(listOf(80), committedDraggedOffsets)
    repeat(90) { mainClock.advanceTimeByFrame() }
    val redRowsAfterSettling = onNodeWithTag("projected-anchor-list").capturedColorRows(Color.Red)
    val expectedFirstRowRange =
      minOf(redRowsBeforeRelease.first(), redRowsAfterSettling.first()) - 1..maxOf(
          redRowsBeforeRelease.first(),
          redRowsAfterSettling.first(),
        ) + 1
    assertTrue(
      redRowsAfterFirstReleaseFrame.firstOrNull()?.let { it in expectedFirstRowRange } == true,
      "Dragged pixels moved away from the slot: before=$redRowsBeforeRelease, first=$redRowsAfterFirstReleaseFrame, settled=$redRowsAfterSettling",
    )
    mainClock.autoAdvance = true
  }

  @Test
  fun upwardReorderTargetsAnOverlappedItemInsideTheEdgeInset() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>
    val itemBounds = mutableMapOf<String, Rect>()
    val keys = (0 until 10).map(Int::toString)
    val draggedKey = "6"

    setContent {
      val lazyListState = rememberLazyListState(initialFirstVisibleItemIndex = 5)
      state = rememberReorderableLazyColumnState(keys = keys, lazyListState = lazyListState)

      ReorderableLazyColumn(
        state = state,
        modifier =
          Modifier.size(width = 100.dp, height = 240.dp)
            .reorderableViewport(state = state, viewportTopInset = 40.dp),
      ) {
        items(state.layoutKeys, key = { it }) { itemKey ->
          Box(
            reorderableAnimatedItem(
                state = state,
                key = itemKey,
                modifier =
                  Modifier.fillMaxWidth().height(if (itemKey == draggedKey) 120.dp else 40.dp),
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

    val draggedBounds = assertNotNull(itemBounds[draggedKey])
    val viewport = assertNotNull(state.viewport)
    val pointer = Offset(x = draggedBounds.center.x, y = draggedBounds.top + 60f)
    val edgePointer = Offset(x = pointer.x, y = viewport.top + 69f)

    runOnUiThread {
      assertTrue(
        state.beginDrag(
          key = draggedKey,
          pointer = pointer,
          onTargetChanged = {},
          onTargetHaptic = {},
          onDragStopped = {},
        )
      )
      state.updatePointer(edgePointer)
      assertEquals(listOf("0", "1", "2", "3", "4", "6", "5", "7", "8", "9"), state.keys)
      state.cancel()
    }
  }

  @Test
  fun upwardEdgeReorderKeepsThePhysicalOrderStable() = runComposeUiTest {
    lateinit var state: ReorderableLazyColumnState<String>
    val itemBounds = mutableMapOf<String, Rect>()
    val keys = (0 until 10).map(Int::toString)

    setContent {
      val lazyListState =
        rememberLazyListState(
          initialFirstVisibleItemIndex = 4,
          initialFirstVisibleItemScrollOffset = 10,
        )
      state = rememberReorderableLazyColumnState(keys = keys, lazyListState = lazyListState)

      ReorderableLazyColumn(
        state = state,
        modifier = Modifier.size(width = 100.dp, height = 160.dp).reorderableViewport(state = state),
      ) {
        items(state.layoutKeys, key = { it }) { itemKey ->
          val itemHeight =
            when (itemKey) {
              "5" -> 80.dp
              "6" -> 60.dp
              else -> 40.dp
            }
          Box(
            reorderableAnimatedItem(
                state = state,
                key = itemKey,
                modifier = Modifier.fillMaxWidth().height(itemHeight),
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

    assertEquals("4", state.lazyListState.layoutInfo.visibleItemsInfo.first().key)
    val draggedBounds = assertNotNull(itemBounds["6"])
    val viewport = assertNotNull(state.viewport)
    val pointer = Offset(x = draggedBounds.center.x, y = draggedBounds.top + 10f)
    val edgePointer = Offset(x = pointer.x, y = viewport.top + 15f)
    mainClock.autoAdvance = false

    runOnUiThread {
      assertTrue(
        state.beginDrag(
          key = "6",
          pointer = pointer,
          onTargetChanged = {},
          onTargetHaptic = {},
          onDragStopped = {},
        )
      )
      state.updatePointer(edgePointer)
      assertEquals(listOf("0", "1", "2", "3", "6", "4", "5", "7", "8", "9"), state.keys)
    }
    mainClock.advanceTimeByFrame()

    assertEquals("4", state.lazyListState.layoutInfo.visibleItemsInfo.first().key)
    assertEquals(keys, state.layoutKeys)
    runOnUiThread { state.cancel() }
    mainClock.advanceTimeByFrame()
    mainClock.autoAdvance = true
  }
}

private fun SemanticsNodeInteraction.capturedColorRows(color: Color): List<Int> {
  val pixels = captureToImage().toPixelMap()
  val x = pixels.width / 2
  return (0 until pixels.height).filter { y -> pixels[x, y] == color }
}
