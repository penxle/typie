package co.typie.ui.component.reorder

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.composed
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.boundsInWindow
import androidx.compose.ui.layout.onGloballyPositioned
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

@Stable
class ReorderableLazyColumnState<K : Any> internal constructor(
  internal val edgeAutoScrollState: co.typie.ext.EdgeAutoScrollState,
) {
  private val itemBounds = mutableStateMapOf<K, Rect>()

  private var pendingCommittedKeys by mutableStateOf<List<K>?>(null)
  private var activeDrag by mutableStateOf<ActiveReorderDrag<K>?>(null)

  var displayedKeys by mutableStateOf<List<K>>(emptyList())
    private set

  val draggingKey: K?
    get() = activeDrag?.key

  val isDragging: Boolean
    get() = activeDrag != null

  fun isDragging(key: K): Boolean = draggingKey == key

  fun syncKeys(serverKeys: List<K>) {
    itemBounds.keys.retainAll(serverKeys.toSet())

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

    activeDrag = ActiveReorderDrag(
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

    val reorderedKeys = calculateReorderedKeys(
      orderedKeys = displayedKeys,
      draggedKey = activeDrag.key,
      pointerWindowY = pointerWindowPosition.y,
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
    this.activeDrag = null

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
    edgeAutoScrollState.stop()
  }

  fun draggedItemTranslationY(key: K): Float {
    val activeDrag = activeDrag ?: return 0f
    if (activeDrag.key != key) return 0f

    val bounds = itemBounds[key] ?: return 0f
    return activeDrag.currentPointerWindowPosition.y - activeDrag.pointerOffsetWithinItemY - bounds.top
  }
}

@Composable
fun <K : Any> rememberReorderableLazyColumnState(
  keys: List<K>,
  lazyListState: LazyListState,
): ReorderableLazyColumnState<K> {
  val edgeAutoScrollState = rememberEdgeAutoScrollState(verticalScrollableState = lazyListState)
  val state = remember(edgeAutoScrollState) {
    ReorderableLazyColumnState<K>(edgeAutoScrollState = edgeAutoScrollState)
  }

  SideEffect {
    state.syncKeys(keys)
  }

  return state
}

fun Modifier.reorderableLazyColumnContainer(
  state: ReorderableLazyColumnState<*>,
): Modifier {
  return edgeAutoScroll(
    state = state.edgeAutoScrollState,
    enabled = state.isDragging,
  )
}

fun <K : Any> Modifier.reorderableItem(
  state: ReorderableLazyColumnState<K>,
  key: K,
): Modifier = composed {
  DisposableEffect(state, key) {
    onDispose {
      state.registerItemBounds(key, bounds = null)
    }
  }

  onGloballyPositioned { coordinates ->
    state.registerItemBounds(key, coordinates.boundsInWindow())
  }
    .zIndex(if (state.isDragging(key)) 2f else 0f)
    .graphicsLayer {
      translationY = state.draggedItemTranslationY(key)
    }
}

fun <K : Any> Modifier.reorderableDragHandle(
  state: ReorderableLazyColumnState<K>,
  key: K,
  enabled: Boolean = true,
  onDragStarted: () -> Unit = {},
  onDragMoved: () -> Unit = {},
  onDragStopped: (ReorderCommit<K>?) -> Unit = {},
): Modifier = composed {
  var handleWindowTopLeft by remember { mutableStateOf(Offset.Zero) }

  onGloballyPositioned { coordinates ->
    handleWindowTopLeft = coordinates.boundsInWindow().topLeft
  }
    .pointerInput(state, key, enabled) {
      if (!enabled) return@pointerInput

      awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false)
        val pointerId = down.id
        val originWindowPosition = down.position + handleWindowTopLeft
        var currentWindowPosition = originWindowPosition
        val startedDrag = state.beginDrag(key, originWindowPosition)

        if (startedDrag) {
          down.consume()
          onDragStarted()
        }

        while (true) {
          val event = awaitPointerEvent()
          val change = event.changes.find { it.id == pointerId } ?: break
          currentWindowPosition = change.position + handleWindowTopLeft

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

private fun <K : Any> calculateReorderedKeys(
  orderedKeys: List<K>,
  draggedKey: K,
  pointerWindowY: Float,
  itemBounds: Map<K, Rect>,
): List<K>? {
  if (orderedKeys.size < 2) return null

  val orderedWithoutDragged = orderedKeys.filterNot { it == draggedKey }
  val visibleTargets = orderedWithoutDragged.mapIndexedNotNull { index, key ->
    val bounds = itemBounds[key] ?: return@mapIndexedNotNull null
    IndexedItemBounds(
      index = index,
      bounds = bounds,
    )
  }
  if (visibleTargets.isEmpty()) return null

  val firstVisible = visibleTargets.first()
  val lastVisible = visibleTargets.last()

  val insertionIndex = when {
    pointerWindowY < firstVisible.bounds.top -> firstVisible.index
    pointerWindowY > lastVisible.bounds.bottom -> lastVisible.index + 1
    else -> {
      visibleTargets.firstOrNull { pointerWindowY < it.bounds.centerY }?.index ?: orderedWithoutDragged.size
    }
  }

  val reordered = orderedWithoutDragged.toMutableList()
  reordered.add(insertionIndex.coerceIn(0, reordered.size), draggedKey)
  return reordered
}

private data class IndexedItemBounds(
  val index: Int,
  val bounds: Rect,
)

private val Rect.centerY: Float
  get() = (top + bottom) / 2f
