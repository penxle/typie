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
  val pointerOffsetWithinItemY: Float,
  val currentPointerWindowPosition: Offset,
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

    settlingDrag = null
    activeDrag =
      ActiveReorderDrag(
        key = key,
        startIndex = startIndex,
        startOrderedKeys = displayedKeys,
        pointerOffsetWithinItemY = pointerWindowPosition.y - bounds.top,
        currentPointerWindowPosition = pointerWindowPosition,
      )
    return true
  }

  fun updateDrag(pointerWindowPosition: Offset): Boolean {
    val activeDrag = activeDrag ?: return false
    this.activeDrag = activeDrag.copy(currentPointerWindowPosition = pointerWindowPosition)
    val draggedItemHeight = itemBounds[activeDrag.key]?.height
    val comparisonWindowY =
      calculateDragComparisonWindowY(
        pointerWindowY = pointerWindowPosition.y,
        pointerOffsetWithinItemY = activeDrag.pointerOffsetWithinItemY,
        itemHeight = draggedItemHeight,
      )

    val reorderedKeys =
      calculateReorderedKeys(
        orderedKeys = displayedKeys,
        draggedKey = activeDrag.key,
        comparisonWindowY = comparisonWindowY,
        itemBounds = itemBounds,
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
      pendingCommittedKeys = null
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
    val settlingTranslation = remember(key) { Animatable(0f) }
    val settlingTranslationY = state.settlingTranslationY(key)

    LaunchedEffect(key, settlingTranslationY) {
      if (settlingTranslationY == null) {
        settlingTranslation.snapTo(0f)
        return@LaunchedEffect
      }

      settlingTranslation.snapTo(settlingTranslationY)
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

          if (!change.pressed) {
            if (startedDrag) {
              onDragStopped(state.endDrag())
            } else {
              state.cancelDrag()
            }
            break
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
  itemBounds: Map<K, Rect>,
): List<K>? {
  if (orderedKeys.size < 2) return null

  val orderedWithoutDragged = orderedKeys.filterNot { it == draggedKey }
  val visibleTargetsInLogicalOrder = orderedWithoutDragged.mapIndexedNotNull { index, key ->
    val bounds = itemBounds[key] ?: return@mapIndexedNotNull null
    if (!bounds.isValidReorderTargetBounds) {
      return@mapIndexedNotNull null
    }
    IndexedItemBounds(index = index, bounds = bounds)
  }
  if (visibleTargetsInLogicalOrder.isEmpty()) return null

  // animateBounds can temporarily leave a key's visual position behind its new logical order.
  // Reorder thresholds should follow the visible slots top-to-bottom, not the lagging key
  // positions,
  // otherwise the list can flip-flop while rows are animating.
  val visibleSlotBounds = visibleTargetsInLogicalOrder.map { it.bounds }.sortedBy { it.top }

  val visibleTargets = visibleTargetsInLogicalOrder.mapIndexed { visibleIndex, target ->
    target.copy(bounds = visibleSlotBounds[visibleIndex])
  }

  val firstVisible = visibleTargets.first()
  val lastVisible = visibleTargets.last()

  val insertionIndex =
    when {
      comparisonWindowY < firstVisible.bounds.top -> firstVisible.index
      comparisonWindowY > lastVisible.bounds.bottom -> lastVisible.index + 1
      else -> {
        visibleTargets.firstOrNull { comparisonWindowY < it.bounds.centerY }?.index
          ?: orderedWithoutDragged.size
      }
    }

  val reordered = orderedWithoutDragged.toMutableList()
  reordered.add(insertionIndex.coerceIn(0, reordered.size), draggedKey)
  return reordered
}

private data class IndexedItemBounds(val index: Int, val bounds: Rect)

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

private val Rect.centerY: Float
  get() = (top + bottom) / 2f

private val Rect.isValidReorderTargetBounds: Boolean
  get() = width > 0f && height > 0f
