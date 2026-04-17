package co.typie.ui.component.reorder

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.foundation.gestures.ScrollableState
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.LookaheadScope
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import co.typie.ext.AutoScrollController
import co.typie.ext.autoScroll
import co.typie.ext.rememberAutoScrollController
import kotlinx.coroutines.flow.drop

data class ReorderDrop<K : Any>(
  val movedKey: K,
  val fromIndex: Int,
  val toIndex: Int,
  val orderedKeys: List<K>,
)

private data class ActiveDrag<K : Any>(
  val key: K,
  val startIndex: Int,
  val startKeys: List<K>,
  val pointer: Offset,
  val pointerOffsetInItemY: Float,
  val comparisonY: Float,
  val direction: Int,
)

private data class Settling<K : Any>(val key: K, val initialOffsetY: Float)

@Stable
class ReorderableColumnState<K : Any>
internal constructor(internal val autoScrollController: AutoScrollController) {
  private val slotBounds = mutableStateMapOf<K, Rect>()
  private var activeDrag by mutableStateOf<ActiveDrag<K>?>(null)
  private var settling by mutableStateOf<Settling<K>?>(null)

  private val _inputKeys = mutableStateOf<List<K>>(emptyList())
  private var _keys by mutableStateOf<List<K>>(emptyList())

  internal var inputKeys: List<K>
    get() = _inputKeys.value
    set(value) {
      if (_inputKeys.value == value) return
      _inputKeys.value = value
      if (activeDrag == null) {
        _keys = value
      }
    }

  val keys: List<K>
    get() = _keys

  val draggingKey: K?
    get() = activeDrag?.key

  val isDragging: Boolean
    get() = activeDrag != null

  fun isDragging(key: K): Boolean = draggingKey == key

  internal val activeDragPointer: Offset?
    get() = activeDrag?.pointer

  internal fun registerSlotBounds(key: K, bounds: Rect?) {
    if (bounds == null) slotBounds.remove(key) else slotBounds[key] = bounds
  }

  internal fun pruneSlotBoundsTo(validKeys: Set<K>) {
    slotBounds.keys.retainAll(validKeys)
  }

  fun draggedOffsetY(key: K): Float {
    val drag = activeDrag ?: return 0f
    if (drag.key != key) return 0f
    val bounds = slotBounds[key] ?: return 0f
    return drag.pointer.y - drag.pointerOffsetInItemY - bounds.top
  }

  internal fun settlingOffsetY(key: K): Float? = settling?.takeIf { it.key == key }?.initialOffsetY

  internal fun clearSettling(key: K) {
    val s = settling ?: return
    if (s.key == key) settling = null
  }

  fun beginDrag(key: K, pointer: Offset): Boolean {
    val bounds = slotBounds[key] ?: return false
    val startIndex = _keys.indexOf(key)
    if (startIndex == -1) return false

    val pointerOffsetInItemY = pointer.y - bounds.top
    val comparisonY = draggedCenterY(pointer.y, pointerOffsetInItemY, bounds.height)

    settling = null
    activeDrag =
      ActiveDrag(
        key = key,
        startIndex = startIndex,
        startKeys = _keys,
        pointer = pointer,
        pointerOffsetInItemY = pointerOffsetInItemY,
        comparisonY = comparisonY,
        direction = 0,
      )
    return true
  }

  fun updateDrag(pointer: Offset): Boolean {
    val drag = activeDrag ?: return false
    val bounds = slotBounds[drag.key]
    val height = bounds?.height ?: 0f
    val newComparisonY = draggedCenterY(pointer.y, drag.pointerOffsetInItemY, height)

    val newDirection =
      when {
        newComparisonY > drag.comparisonY + DragDirectionEpsilonPx -> 1
        newComparisonY < drag.comparisonY - DragDirectionEpsilonPx -> -1
        else -> drag.direction
      }

    val previousKeys = _keys
    if (bounds != null) {
      val top = pointer.y - drag.pointerOffsetInItemY
      val draggedBounds =
        Rect(left = bounds.left, top = top, right = bounds.right, bottom = top + bounds.height)
      _keys =
        reorderedKeysForDrag(
          keys = _keys,
          draggedKey = drag.key,
          direction = newDirection,
          slotBounds = slotBounds + (drag.key to draggedBounds),
        ) ?: _keys
    }

    activeDrag =
      drag.copy(pointer = pointer, comparisonY = newComparisonY, direction = newDirection)
    return _keys != previousKeys
  }

  fun endDrag(): ReorderDrop<K>? {
    autoScrollController.pointer = null
    val drag = activeDrag ?: return null

    val releaseOffset = draggedOffsetY(drag.key)
    val finalKeys = _keys
    activeDrag = null

    settling =
      releaseOffset.takeUnless { it == 0f }?.let { Settling(key = drag.key, initialOffsetY = it) }

    if (finalKeys == drag.startKeys) return null

    return ReorderDrop(
      movedKey = drag.key,
      fromIndex = drag.startIndex,
      toIndex = finalKeys.indexOf(drag.key),
      orderedKeys = finalKeys,
    )
  }

  fun cancelDrag() {
    activeDrag = null
    settling = null
    _keys = inputKeys
    autoScrollController.pointer = null
  }
}

@Composable
fun <K : Any> rememberReorderableColumnState(
  keys: List<K>,
  verticalScrollableState: ScrollableState? = null,
): ReorderableColumnState<K> {
  val controller = rememberAutoScrollController(verticalScrollableState = verticalScrollableState)
  val state = remember(controller) { ReorderableColumnState<K>(autoScrollController = controller) }

  SideEffect { state.inputKeys = keys }

  LaunchedEffect(state) {
    snapshotFlow { state.draggingKey to state.inputKeys }
      .collect { (draggingKey, inputKeys) ->
        if (draggingKey != null && draggingKey !in inputKeys) {
          state.cancelDrag()
        }
        state.pruneSlotBoundsTo(inputKeys.toSet())
      }
  }

  return state
}

interface ReorderableColumnScope : ColumnScope, LookaheadScope

internal val LocalReorderableLookaheadScope = staticCompositionLocalOf<LookaheadScope?> { null }

@Composable
fun ReorderableColumn(
  state: ReorderableColumnState<*>,
  modifier: Modifier = Modifier,
  verticalArrangement: Arrangement.Vertical = Arrangement.Top,
  horizontalAlignment: Alignment.Horizontal = Alignment.Start,
  content: @Composable ReorderableColumnScope.() -> Unit,
) {
  LookaheadScope {
    val outerLookaheadScope = this@LookaheadScope
    CompositionLocalProvider(LocalReorderableLookaheadScope provides outerLookaheadScope) {
      Column(
        modifier = modifier,
        verticalArrangement = verticalArrangement,
        horizontalAlignment = horizontalAlignment,
      ) {
        val columnScope = this@Column
        val reorderScope =
          remember(columnScope, outerLookaheadScope) {
            object :
              ReorderableColumnScope,
              ColumnScope by columnScope,
              LookaheadScope by outerLookaheadScope {}
          }
        reorderScope.content()
      }
    }
  }
}

@Composable
fun <K : Any> Modifier.reorderableItem(state: ReorderableColumnState<K>, key: K): Modifier {
  val lookaheadScope =
    LocalReorderableLookaheadScope.current
      ?: error("Modifier.reorderableItem must be called inside ReorderableColumn { ... }")

  val settlingOffsetY = state.settlingOffsetY(key)
  val isSettling = settlingOffsetY != null
  val settlingAnim = remember(key, isSettling) { Animatable(settlingOffsetY ?: 0f) }

  LaunchedEffect(key, isSettling) {
    if (!isSettling) {
      settlingAnim.snapTo(0f)
      return@LaunchedEffect
    }
    settlingAnim.animateTo(
      targetValue = 0f,
      animationSpec = spring(dampingRatio = 0.9f, stiffness = Spring.StiffnessMedium),
    )
    state.clearSettling(key)
  }

  return this.onPlaced { coords ->
      with(lookaheadScope) {
        val lookaheadCoords = coords.toLookaheadCoordinates()
        val origin = lookaheadCoords.positionInWindow()
        val size = lookaheadCoords.size
        state.registerSlotBounds(
          key,
          Rect(
            left = origin.x,
            top = origin.y,
            right = origin.x + size.width,
            bottom = origin.y + size.height,
          ),
        )
      }
    }
    .zIndex(if (state.isDragging(key)) 2f else 0f)
    .graphicsLayer { translationY = settlingAnim.value + state.draggedOffsetY(key) }
}

@Composable
fun Modifier.reorderableViewport(
  state: ReorderableColumnState<*>,
  viewportTopInset: Dp = 0.dp,
  viewportBottomInset: Dp = 0.dp,
): Modifier =
  autoScroll(
    controller = state.autoScrollController,
    enabled = state.isDragging,
    viewportTopInset = viewportTopInset,
    viewportBottomInset = viewportBottomInset,
  )

@Composable
fun <K : Any> Modifier.reorderableDragHandle(
  state: ReorderableColumnState<K>,
  key: K,
  enabled: Boolean = true,
  onDragStarted: () -> Unit = {},
  onDragMoved: () -> Unit = {},
  onDragStopped: (drop: ReorderDrop<K>?) -> Unit = {},
): Modifier {
  val haptic = LocalHapticFeedback.current
  val onDragMovedUpdated by rememberUpdatedState(onDragMoved)
  var handleCoordinates by remember { mutableStateOf<LayoutCoordinates?>(null) }

  LaunchedEffect(state, key) {
    snapshotFlow { if (state.isDragging(key)) state.keys else null }
      .drop(1)
      .collect { keys ->
        if (keys == null) return@collect
        haptic.performHapticFeedback(HapticFeedbackType.SegmentFrequentTick)
        onDragMovedUpdated()
      }
  }

  return this.onGloballyPositioned { coords -> handleCoordinates = coords }
    .pointerInput(state, key, enabled) {
      if (!enabled) return@pointerInput

      awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false)
        val pointerId = down.id
        val originWindow =
          handleCoordinates?.localToWindow(down.position) ?: return@awaitEachGesture
        var currentWindow = originWindow
        val started = state.beginDrag(key, originWindow)

        if (started) {
          down.consume()
          haptic.performHapticFeedback(HapticFeedbackType.GestureThresholdActivate)
          onDragStarted()
        }

        while (true) {
          val event = awaitPointerEvent()
          val change = event.changes.find { it.id == pointerId } ?: break
          currentWindow = handleCoordinates?.localToWindow(change.position) ?: currentWindow

          if (!change.pressed) {
            if (started) {
              val drop = state.endDrag()
              haptic.performHapticFeedback(HapticFeedbackType.GestureEnd)
              onDragStopped(drop)
            } else {
              state.cancelDrag()
            }
            break
          }

          if (started) {
            change.consume()
            state.updateDrag(currentWindow)
            state.autoScrollController.pointer = currentWindow
          }
        }

        if (started && state.isDragging(key)) {
          state.cancelDrag()
          onDragStopped(null)
        }
      }
    }
}

private const val DragDirectionEpsilonPx = 0.5f

internal fun draggedCenterY(
  pointerY: Float,
  pointerOffsetInItemY: Float,
  itemHeight: Float,
): Float {
  return pointerY - pointerOffsetInItemY + itemHeight / 2f
}

internal fun <K : Any> reorderedKeysForDrag(
  keys: List<K>,
  draggedKey: K,
  direction: Int,
  slotBounds: Map<K, Rect>,
): List<K>? {
  if (keys.size < 2) return null
  val currentIndex = keys.indexOf(draggedKey)
  if (currentIndex == -1) return null
  val draggedBounds = slotBounds[draggedKey] ?: return null

  var insertionIndex = currentIndex

  if (direction < 0) {
    while (insertionIndex > 0) {
      val prev = keys[insertionIndex - 1]
      val prevBounds = slotBounds[prev] ?: break
      if (shouldSwapTowardsPrevious(draggedBounds, prevBounds)) {
        insertionIndex -= 1
      } else break
    }
  } else if (direction > 0) {
    while (insertionIndex < keys.lastIndex) {
      val next = keys[insertionIndex + 1]
      val nextBounds = slotBounds[next] ?: break
      if (shouldSwapTowardsNext(draggedBounds, nextBounds)) {
        insertionIndex += 1
      } else break
    }
  }

  val reordered = keys.toMutableList()
  reordered.remove(draggedKey)
  reordered.add(insertionIndex.coerceIn(0, reordered.size), draggedKey)
  return reordered
}

private fun shouldSwapTowardsPrevious(draggedBounds: Rect, prevBounds: Rect): Boolean {
  val requiredOverlap = minOf(draggedBounds.height, prevBounds.height) / 2f
  return draggedBounds.verticalOverlap(prevBounds) > requiredOverlap ||
    draggedBounds.bottom <= prevBounds.top
}

private fun shouldSwapTowardsNext(draggedBounds: Rect, nextBounds: Rect): Boolean {
  val requiredOverlap = minOf(draggedBounds.height, nextBounds.height) / 2f
  return draggedBounds.verticalOverlap(nextBounds) > requiredOverlap ||
    draggedBounds.top >= nextBounds.bottom
}

private fun Rect.verticalOverlap(other: Rect): Float =
  (minOf(bottom, other.bottom) - maxOf(top, other.top)).coerceAtLeast(0f)
