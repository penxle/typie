package co.typie.ui.component.reorder

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.foundation.gestures.ScrollableState
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.boundsInWindow
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import co.typie.ext.edgeAutoScroll
import co.typie.ext.rememberEdgeAutoScrollState

data class ReorderCommit<K : Any>(
  val movedKey: K,
  val fromIndex: Int,
  val toIndex: Int,
  val orderedKeys: List<K>,
)

private data class ActiveReorderDrag<K : Any>(
  val key: K,
  val startIndex: Int,
  val startOrderedKeys: List<K>,
  val referenceSlotBoundsByIndex: Map<Int, Rect>,
  val referenceItemBoundsByKey: Map<K, Rect>,
  val pointerOffsetWithinItemY: Float,
  val currentPointerWindowPosition: Offset,
  val currentComparisonWindowY: Float,
  val movementDirection: Int,
)

private data class SettlingReorderDrag<K : Any>(val key: K, val initialTranslationY: Float)

@Stable
class ReorderableListState<K : Any>
internal constructor(internal val edgeAutoScrollState: co.typie.ext.EdgeAutoScrollState) {
  private val itemBounds = mutableStateMapOf<K, Rect>()

  private var pendingCommittedKeys by mutableStateOf<List<K>?>(null)
  private var activeDrag by mutableStateOf<ActiveReorderDrag<K>?>(null)
  private var settlingDrag by mutableStateOf<SettlingReorderDrag<K>?>(null)

  var displayedKeys by mutableStateOf<List<K>>(emptyList())
    private set

  val draggingKey: K?
    get() = activeDrag?.key

  val isDragging: Boolean
    get() = activeDrag != null

  fun isDragging(key: K): Boolean = draggingKey == key

  fun syncKeys(serverKeys: List<K>) {
    itemBounds.keys.retainAll(serverKeys.toSet())
    settlingDrag = settlingDrag?.takeIf { it.key in serverKeys }

    val activeDrag = activeDrag
    if (activeDrag != null) {
      if (displayedKeys.toSet() != serverKeys.toSet()) {
        cancelDrag()
        displayedKeys = serverKeys
        pendingCommittedKeys = null
      }
      return
    }

    val pendingCommittedKeys = pendingCommittedKeys
    when {
      pendingCommittedKeys == null -> {
        displayedKeys = serverKeys
      }

      serverKeys == pendingCommittedKeys -> {
        displayedKeys = serverKeys
        this.pendingCommittedKeys = null
      }

      serverKeys.toSet() == pendingCommittedKeys.toSet() -> {
        displayedKeys = pendingCommittedKeys
      }

      else -> {
        displayedKeys = serverKeys
        this.pendingCommittedKeys = null
      }
    }
  }

  fun resetToServerKeys(serverKeys: List<K>) {
    activeDrag = null
    settlingDrag = null
    pendingCommittedKeys = null
    displayedKeys = serverKeys
    edgeAutoScrollState.stop()
  }

  fun registerItemBounds(key: K, bounds: Rect?) {
    if (bounds == null) {
      itemBounds.remove(key)
    } else {
      itemBounds[key] = bounds
    }
  }

  fun beginDrag(key: K, pointerWindowPosition: Offset): Boolean {
    val bounds = itemBounds[key] ?: return false
    val startIndex = displayedKeys.indexOf(key)
    if (startIndex == -1) return false
    val referenceSlotBoundsByIndex =
      displayedKeys
        .mapIndexedNotNull { index, displayedKey ->
          itemBounds[displayedKey]?.takeIf { it.isValidReorderTargetBounds }?.let { index to it }
        }
        .toMap()
    val referenceItemBoundsByKey = displayedKeys.associateWithNotNull { displayedKey ->
      itemBounds[displayedKey]?.takeIf { it.isValidReorderTargetBounds }
    }
    val initialComparisonWindowY =
      calculateDragComparisonWindowY(
        pointerWindowY = pointerWindowPosition.y,
        pointerOffsetWithinItemY = pointerWindowPosition.y - bounds.top,
        itemHeight = bounds.height,
      )

    settlingDrag = null
    activeDrag =
      ActiveReorderDrag(
        key = key,
        startIndex = startIndex,
        startOrderedKeys = displayedKeys,
        referenceSlotBoundsByIndex = referenceSlotBoundsByIndex,
        referenceItemBoundsByKey = referenceItemBoundsByKey,
        pointerOffsetWithinItemY = pointerWindowPosition.y - bounds.top,
        currentPointerWindowPosition = pointerWindowPosition,
        currentComparisonWindowY = initialComparisonWindowY,
        movementDirection = 0,
      )
    return true
  }

  fun updateDrag(pointerWindowPosition: Offset): Boolean {
    val activeDrag = activeDrag ?: return false
    val draggedItemBounds =
      calculateDraggedItemWindowBounds(
        itemBounds = itemBounds[activeDrag.key],
        pointerWindowY = pointerWindowPosition.y,
        pointerOffsetWithinItemY = activeDrag.pointerOffsetWithinItemY,
      )
    val comparisonWindowY =
      calculateDragComparisonWindowY(
        pointerWindowY = pointerWindowPosition.y,
        pointerOffsetWithinItemY = activeDrag.pointerOffsetWithinItemY,
        itemHeight = draggedItemBounds?.height,
      )
    val movementDirection =
      when {
        comparisonWindowY > activeDrag.currentComparisonWindowY + ReorderDirectionEpsilonPx -> 1
        comparisonWindowY < activeDrag.currentComparisonWindowY - ReorderDirectionEpsilonPx -> -1
        else -> activeDrag.movementDirection
      }
    this.activeDrag =
      activeDrag.copy(
        currentPointerWindowPosition = pointerWindowPosition,
        currentComparisonWindowY = comparisonWindowY,
        movementDirection = movementDirection,
      )

    val reorderedKeys =
      calculateReorderedKeys(
        orderedKeys = displayedKeys,
        draggedKey = activeDrag.key,
        comparisonWindowY = comparisonWindowY,
        movementDirection = movementDirection,
        itemBounds = draggedItemBounds?.let { itemBounds + (activeDrag.key to it) } ?: itemBounds,
        referenceSlotBoundsByIndex = activeDrag.referenceSlotBoundsByIndex,
        referenceItemBoundsByKey = activeDrag.referenceItemBoundsByKey,
      ) ?: return false

    if (reorderedKeys == displayedKeys) {
      return false
    }

    displayedKeys = reorderedKeys
    return true
  }

  fun endDrag(): ReorderCommit<K>? {
    edgeAutoScrollState.stop()

    val activeDrag = activeDrag ?: return null
    val settlingTranslationY = draggedItemTranslationY(activeDrag.key)
    this.activeDrag = null
    settlingDrag =
      settlingTranslationY
        .takeUnless { it == 0f }
        ?.let { SettlingReorderDrag(key = activeDrag.key, initialTranslationY = it) }

    val orderedKeys = displayedKeys
    if (orderedKeys == activeDrag.startOrderedKeys) {
      return null
    }

    pendingCommittedKeys = orderedKeys
    return ReorderCommit(
      movedKey = activeDrag.key,
      fromIndex = activeDrag.startIndex,
      toIndex = orderedKeys.indexOf(activeDrag.key),
      orderedKeys = orderedKeys,
    )
  }

  fun cancelDrag() {
    activeDrag = null
    settlingDrag = null
    edgeAutoScrollState.stop()
  }

  fun draggedItemTranslationY(key: K): Float {
    val activeDrag = activeDrag ?: return 0f
    if (activeDrag.key != key) return 0f

    val bounds = itemBounds[key] ?: return 0f
    return activeDrag.currentPointerWindowPosition.y -
      activeDrag.pointerOffsetWithinItemY -
      bounds.top
  }

  internal fun settlingTranslationY(key: K): Float? {
    return settlingDrag?.takeIf { it.key == key }?.initialTranslationY
  }

  internal fun clearSettlingTranslation(key: K) {
    val settlingDrag = settlingDrag ?: return
    if (settlingDrag.key == key) {
      this.settlingDrag = null
    }
  }
}

@Composable
fun <K : Any> rememberReorderableListState(
  keys: List<K>,
  verticalScrollableState: ScrollableState? = null,
  horizontalScrollableState: ScrollableState? = null,
): ReorderableListState<K> {
  val edgeAutoScrollState =
    rememberEdgeAutoScrollState(
      verticalScrollableState = verticalScrollableState,
      horizontalScrollableState = horizontalScrollableState,
    )
  val state =
    remember(edgeAutoScrollState) {
      ReorderableListState<K>(edgeAutoScrollState = edgeAutoScrollState)
    }

  SideEffect { state.syncKeys(keys) }

  return state
}

fun Modifier.reorderableListContainer(
  state: ReorderableListState<*>,
  viewportTopInset: Dp = 0.dp,
  viewportBottomInset: Dp = 0.dp,
): Modifier {
  return edgeAutoScroll(
    state = state.edgeAutoScrollState,
    enabled = state.isDragging,
    viewportTopInset = viewportTopInset,
    viewportBottomInset = viewportBottomInset,
  )
}

fun <K : Any> Modifier.reorderableItem(state: ReorderableListState<K>, key: K): Modifier =
  composed {
    val settlingTranslationY = state.settlingTranslationY(key)
    val isSettling = settlingTranslationY != null
    val settlingTranslation = remember(key, isSettling) { Animatable(settlingTranslationY ?: 0f) }

    LaunchedEffect(key, isSettling) {
      if (!isSettling) {
        settlingTranslation.snapTo(0f)
        return@LaunchedEffect
      }

      settlingTranslation.animateTo(
        targetValue = 0f,
        animationSpec = spring(dampingRatio = 0.9f, stiffness = Spring.StiffnessMedium),
      )
      state.clearSettlingTranslation(key)
    }

    DisposableEffect(state, key) { onDispose { state.registerItemBounds(key, bounds = null) } }

    onGloballyPositioned { coordinates ->
        state.registerItemBounds(key, coordinates.boundsInWindow())
      }
      .zIndex(if (state.isDragging(key)) 2f else 0f)
      .graphicsLayer {
        translationY = settlingTranslation.value + state.draggedItemTranslationY(key)
      }
  }

fun <K : Any> Modifier.reorderableDragHandle(
  state: ReorderableListState<K>,
  key: K,
  enabled: Boolean = true,
  onDragStarted: () -> Unit = {},
  onDragMoved: () -> Unit = {},
  onDragStopped: (ReorderCommit<K>?) -> Unit = {},
): Modifier = composed {
  var handleCoordinates by remember { mutableStateOf<LayoutCoordinates?>(null) }

  onGloballyPositioned { coordinates -> handleCoordinates = coordinates }
    .pointerInput(state, key, enabled) {
      if (!enabled) return@pointerInput

      awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false)
        val pointerId = down.id
        val originWindowPosition =
          handleCoordinates?.localToWindow(down.position) ?: return@awaitEachGesture
        var currentWindowPosition = originWindowPosition
        val startedDrag = state.beginDrag(key, originWindowPosition)

        if (startedDrag) {
          down.consume()
          onDragStarted()
        }

        while (true) {
          val event = awaitPointerEvent()
          val change = event.changes.find { it.id == pointerId } ?: break
          currentWindowPosition =
            handleCoordinates?.localToWindow(change.position) ?: currentWindowPosition

          if (!change.pressed) {
            if (startedDrag) {
              onDragStopped(state.endDrag())
            } else {
              state.cancelDrag()
            }
            break
          }

          if (startedDrag) {
            change.consume()
            if (state.updateDrag(currentWindowPosition)) {
              onDragMoved()
            }
            state.edgeAutoScrollState.update(
              pointerPosition = currentWindowPosition,
              onAutoScroll = {
                if (state.updateDrag(currentWindowPosition)) {
                  onDragMoved()
                }
              },
            )
          }
        }

        if (startedDrag && state.isDragging(key)) {
          state.cancelDrag()
          onDragStopped(null)
        }
      }
    }
}

internal fun <K : Any> calculateReorderedKeys(
  orderedKeys: List<K>,
  draggedKey: K,
  comparisonWindowY: Float,
  movementDirection: Int = 0,
  itemBounds: Map<K, Rect>,
  referenceSlotBoundsByIndex: Map<Int, Rect> = emptyMap(),
  referenceItemBoundsByKey: Map<K, Rect> = emptyMap(),
): List<K>? {
  if (orderedKeys.size < 2) return null

  val currentIndex = orderedKeys.indexOf(draggedKey)
  if (currentIndex == -1) return null

  val draggedBounds = itemBounds[draggedKey]?.takeIf { it.isValidReorderTargetBounds }
  var insertionIndex = currentIndex

  if (movementDirection < 0) {
    while (insertionIndex > 0) {
      val previousIndex = insertionIndex - 1
      val previousKey = orderedKeys[previousIndex]
      val previousBounds =
        adjacentBoundsFor(
          key = previousKey,
          index = previousIndex,
          itemBounds = itemBounds,
          referenceItemBoundsByKey = referenceItemBoundsByKey,
          referenceSlotBoundsByIndex = referenceSlotBoundsByIndex,
        ) ?: break
      val previousThresholdHeight =
        adjacentThresholdHeight(
          key = previousKey,
          adjacentBounds = previousBounds,
          itemBounds = itemBounds,
          referenceItemBoundsByKey = referenceItemBoundsByKey,
        ) ?: break

      if (
        shouldSwapTowardsPrevious(
          draggedBounds = draggedBounds,
          comparisonWindowY = comparisonWindowY,
          previousBounds = previousBounds,
          previousThresholdHeight = previousThresholdHeight,
        )
      ) {
        insertionIndex -= 1
      } else {
        break
      }
    }
  } else if (movementDirection > 0) {
    while (insertionIndex < orderedKeys.lastIndex) {
      val nextIndex = insertionIndex + 1
      val nextKey = orderedKeys[nextIndex]
      val nextBounds =
        adjacentBoundsFor(
          key = nextKey,
          index = nextIndex,
          itemBounds = itemBounds,
          referenceItemBoundsByKey = referenceItemBoundsByKey,
          referenceSlotBoundsByIndex = referenceSlotBoundsByIndex,
        ) ?: break
      val nextThresholdHeight =
        adjacentThresholdHeight(
          key = nextKey,
          adjacentBounds = nextBounds,
          itemBounds = itemBounds,
          referenceItemBoundsByKey = referenceItemBoundsByKey,
        ) ?: break

      if (
        shouldSwapTowardsNext(
          draggedBounds = draggedBounds,
          comparisonWindowY = comparisonWindowY,
          nextBounds = nextBounds,
          nextThresholdHeight = nextThresholdHeight,
        )
      ) {
        insertionIndex += 1
      } else {
        break
      }
    }
  }

  val reordered = orderedKeys.toMutableList()
  reordered.remove(draggedKey)
  reordered.add(insertionIndex.coerceIn(0, reordered.size), draggedKey)
  return reordered
}

internal fun calculateDragComparisonWindowY(
  pointerWindowY: Float,
  pointerOffsetWithinItemY: Float,
  itemHeight: Float?,
): Float {
  if (itemHeight == null) {
    return pointerWindowY
  }

  return pointerWindowY - pointerOffsetWithinItemY + itemHeight / 2f
}

private fun calculateDraggedItemWindowBounds(
  itemBounds: Rect?,
  pointerWindowY: Float,
  pointerOffsetWithinItemY: Float,
): Rect? {
  val bounds = itemBounds?.takeIf { it.isValidReorderTargetBounds } ?: return null
  val top = pointerWindowY - pointerOffsetWithinItemY
  return Rect(left = bounds.left, top = top, right = bounds.right, bottom = top + bounds.height)
}

private fun Rect.verticalOverlap(other: Rect): Float {
  return (minOf(bottom, other.bottom) - maxOf(top, other.top)).coerceAtLeast(0f)
}

private fun <K : Any> adjacentBoundsFor(
  key: K,
  index: Int,
  itemBounds: Map<K, Rect>,
  referenceItemBoundsByKey: Map<K, Rect>,
  referenceSlotBoundsByIndex: Map<Int, Rect>,
): Rect? {
  return itemBounds[key]?.takeIf { it.isValidReorderTargetBounds }
    ?: referenceItemBoundsByKey[key]
    ?: referenceSlotBoundsByIndex[index]
}

private fun <K : Any> adjacentThresholdHeight(
  key: K,
  adjacentBounds: Rect?,
  itemBounds: Map<K, Rect>,
  referenceItemBoundsByKey: Map<K, Rect>,
): Float? {
  return referenceItemBoundsByKey[key]?.height
    ?: itemBounds[key]?.takeIf { it.isValidReorderTargetBounds }?.height
    ?: adjacentBounds?.height
}

private fun shouldSwapTowardsPrevious(
  draggedBounds: Rect?,
  comparisonWindowY: Float,
  previousBounds: Rect,
  previousThresholdHeight: Float,
): Boolean {
  if (draggedBounds == null) {
    return comparisonWindowY <= previousBounds.bottom
  }

  val requiredOverlap = minOf(draggedBounds.height, previousThresholdHeight) / 2f
  return draggedBounds.verticalOverlap(previousBounds) >= requiredOverlap ||
    draggedBounds.bottom <= previousBounds.top
}

private fun shouldSwapTowardsNext(
  draggedBounds: Rect?,
  comparisonWindowY: Float,
  nextBounds: Rect,
  nextThresholdHeight: Float,
): Boolean {
  if (draggedBounds == null) {
    return comparisonWindowY >= nextBounds.top
  }

  val requiredOverlap = minOf(draggedBounds.height, nextThresholdHeight) / 2f
  return draggedBounds.verticalOverlap(nextBounds) >= requiredOverlap ||
    draggedBounds.top >= nextBounds.bottom
}

private inline fun <K, V> Iterable<K>.associateWithNotNull(valueTransform: (K) -> V?): Map<K, V> {
  return buildMap {
    for (key in this@associateWithNotNull) {
      val value = valueTransform(key) ?: continue
      put(key, value)
    }
  }
}

private const val ReorderDirectionEpsilonPx = 0.5f

private val Rect.isValidReorderTargetBounds: Boolean
  get() = width > 0f && height > 0f
