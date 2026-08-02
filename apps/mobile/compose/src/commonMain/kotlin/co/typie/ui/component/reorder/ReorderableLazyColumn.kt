package co.typie.ui.component.reorder

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.VisibilityThreshold
import androidx.compose.animation.core.spring
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyItemScope
import androidx.compose.foundation.lazy.LazyListLayoutInfo
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChangeIgnoreConsumed
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.LocalPinnableContainer
import androidx.compose.ui.layout.layout
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import co.typie.ext.EdgeAutoScrollController
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.ScrollGestureLockHandle
import co.typie.ext.edgeAutoScroll
import co.typie.ext.rememberEdgeAutoScrollController
import kotlin.time.TimeSource
import kotlinx.coroutines.launch

private data class LazyLayoutObservation(
  val scrollEpoch: Long,
  val verticalScrollDirection: Int,
  val viewport: Rect?,
  val layoutInfo: LazyListLayoutInfo,
)

@Stable
class ReorderableLazyColumnState<K : Any>
internal constructor(
  val lazyListState: LazyListState,
  internal val orderState: ReorderState<K>,
  internal val edgeAutoScrollController: EdgeAutoScrollController,
  private val nowMillis: () -> Long,
) {
  private val interaction = ReorderInteraction(orderState)
  private val hapticPolicy = ReorderHapticPolicy()
  private var orderEpoch by mutableLongStateOf(0L)
  private var latestIdleLayout: ReorderLayoutSnapshot<K>? = null
  private var onTargetChanged: (() -> Unit)? = null
  private var onTargetHaptic: (() -> Unit)? = null
  private var onDragStopped: ((ReorderDrop<K>?) -> Unit)? = null
  private var observedDragging = orderState.isDragging
  private var endingDrag = false
  private var draggedVisualOffsetY by mutableFloatStateOf(0f)
  private val placedItemTops = mutableMapOf<K, Float>()
  private var draggedPlacedTopY: Float? = null
  private var draggedVisualOriginY: Float? = null
  private var draggedPlacementOrderRevision: Long? = null
  internal var viewport: Rect? by mutableStateOf(null)

  private val orderChangedListener: () -> Unit = {
    val wasDragging = observedDragging
    observedDragging = orderState.isDragging
    orderEpoch += 1
    if (wasDragging && !observedDragging && !endingDrag) {
      interaction.acknowledgeExternalCancellation()
      finishDrag(drop = null)
    }
  }

  init {
    orderState.onChanged = orderChangedListener
  }

  val keys: List<K>
    get() {
      orderEpoch
      return orderState.keys
    }

  val draggingKey: K?
    get() {
      orderEpoch
      return orderState.draggingKey
    }

  val isDragging: Boolean
    get() = draggingKey != null

  internal fun isDragging(key: K): Boolean {
    orderEpoch
    return orderState.isDragging(key)
  }

  internal fun isSettling(key: K): Boolean {
    orderEpoch
    return orderState.isSettling(key)
  }

  internal fun settlingOffsetY(key: K): Float? {
    orderEpoch
    return orderState.settlingOffsetY(key)
  }

  internal fun clearSettling(key: K) {
    orderState.clearSettling(key)
  }

  internal fun draggedOffsetY(key: K): Float =
    if (orderState.isDragging(key)) draggedVisualOffsetY else 0f

  internal fun updateInputKeys(keys: List<K>) {
    if (orderState.inputKeys != keys) placedItemTops.keys.retainAll(keys.toSet())
    val wasDragging = orderState.isDragging
    endingDrag = true
    try {
      interaction.updateInputKeys(keys)
    } finally {
      endingDrag = false
    }
    if (wasDragging && !orderState.isDragging) finishDrag(drop = null)
  }

  internal fun publishLayout(layoutInfo: LazyListLayoutInfo, viewport: Rect, scrollDirection: Int) {
    val displayedKeys = orderState.keys
    val displayedKeyByValue = displayedKeys.associateBy { it as Any }
    val snapshot =
      ReorderLayoutSnapshot(
        viewportTop = viewport.top + layoutInfo.viewportStartOffset,
        viewportBottom = viewport.top + layoutInfo.viewportEndOffset,
        items =
          layoutInfo.visibleItemsInfo.mapNotNull { item ->
            val key = displayedKeyByValue[item.key] ?: return@mapNotNull null
            ReorderLayoutItem(
              key = key,
              lazyIndex = item.index,
              top = viewport.top + item.offset,
              bottom = viewport.top + item.offset + item.size,
            )
          },
      )
    if (!orderState.isDragging) {
      latestIdleLayout = snapshot
      interaction.publishLayout(snapshot)
      return
    }

    val layoutProposal = interaction.publishLayout(snapshot, scrollDirection)
    val placementProposal =
      interaction.draggedItemInPublishedLayout()?.let { draggedItem ->
        val placementOrderRevision = orderState.orderRevision
        interaction
          .updateDraggedSize(height = draggedItem.height, scrollDirection = scrollDirection)
          .also { draggedPlacementOrderRevision = placementOrderRevision }
      }
    val proposal = placementProposal ?: layoutProposal
    commitProposal(proposal)
    refreshDraggedVisualOffset()
  }

  internal fun beginDrag(
    key: K,
    pointer: Offset,
    onTargetChanged: () -> Unit,
    onTargetHaptic: () -> Unit,
    onDragStopped: (ReorderDrop<K>?) -> Unit,
  ): Boolean {
    val item = latestIdleLayout?.items?.firstOrNull { it.key == key } ?: return false
    if (
      !interaction.beginDrag(
        key = key,
        pointerY = pointer.y,
        pointerOffsetInItemY = pointer.y - item.top,
      )
    ) {
      return false
    }
    draggedPlacementOrderRevision = orderState.orderRevision
    interaction.updateDraggedSize(height = item.height)
    draggedPlacedTopY = placedItemTops[key]
    draggedVisualOriginY = draggedPlacedTopY?.let { placedTop ->
      interaction.draggedTopY(key)?.let { draggedTop -> placedTop - draggedTop }
    }
    refreshDraggedVisualOffset()

    this.onTargetChanged = onTargetChanged
    this.onTargetHaptic = onTargetHaptic
    this.onDragStopped = onDragStopped
    hapticPolicy.beginDrag()
    return true
  }

  internal fun updatePointer(pointer: Offset) {
    if (!orderState.isDragging) return
    edgeAutoScrollController.pointer = pointer
    commitProposal(interaction.updatePointer(pointer.y))
    refreshDraggedVisualOffset()
  }

  internal fun updateItemPlacement(key: K, placedTop: Float) {
    placedItemTops[key] = placedTop
    if (!orderState.isDragging(key)) return

    draggedPlacedTopY = placedTop
    if (draggedVisualOriginY == null) {
      draggedVisualOriginY =
        interaction.draggedTopY(key)?.let { draggedTop ->
          placedTop + draggedVisualOffsetY - draggedTop
        }
    }
    refreshDraggedVisualOffset()
  }

  internal suspend fun awaitCurrentDraggedPlacement() {
    repeat(2) {
      if (!orderState.isDragging) return
      if (draggedPlacementOrderRevision == orderState.orderRevision) return
      withFrameNanos {}
    }
  }

  internal fun release() {
    if (!orderState.isDragging) return
    val drop =
      try {
        endingDrag = true
        interaction.release(releaseOffsetY = draggedVisualOffsetY)
      } finally {
        endingDrag = false
      }
    finishDrag(drop)
  }

  internal fun cancel() {
    if (!orderState.isDragging) return
    try {
      endingDrag = true
      interaction.cancel()
    } finally {
      endingDrag = false
    }
    finishDrag(drop = null)
  }

  internal fun dispose() {
    cancel()
    if (orderState.onChanged === orderChangedListener) orderState.onChanged = null
  }

  private fun commitProposal(proposal: ReorderTargetProposal<K>?) {
    proposal ?: return
    if (orderState.draggingKey != proposal.draggedKey) return
    if (orderState.orderRevision != proposal.sourceOrderRevision) return

    val firstVisibleItem =
      lazyListState.layoutInfo.visibleItemsInfo.firstOrNull {
        it.index == lazyListState.firstVisibleItemIndex
      }
    if (firstVisibleItem != null) {
      lazyListState.requestScrollToItem(
        index = lazyListState.firstVisibleItemIndex,
        scrollOffset = lazyListState.firstVisibleItemScrollOffset,
      )
    }

    if (!interaction.commitTarget(proposal)) return
    refreshDraggedVisualOffset()
    onTargetChanged?.invoke()
    if (hapticPolicy.shouldEmit(proposal.targetIndex, nowMillis())) onTargetHaptic?.invoke()
  }

  private fun finishDrag(drop: ReorderDrop<K>?) {
    edgeAutoScrollController.pointer = null
    latestIdleLayout = null
    draggedVisualOffsetY = 0f
    draggedPlacedTopY = null
    draggedVisualOriginY = null
    draggedPlacementOrderRevision = null
    hapticPolicy.endDrag()

    val stopped = onDragStopped
    onTargetChanged = null
    onTargetHaptic = null
    onDragStopped = null
    stopped?.invoke(drop)
  }

  private fun refreshDraggedVisualOffset() {
    val key = orderState.draggingKey ?: return
    val placedTop = draggedPlacedTopY ?: return
    val visualOrigin = draggedVisualOriginY ?: return
    val draggedTop = interaction.draggedTopY(key) ?: return
    draggedVisualOffsetY = draggedTop + visualOrigin - placedTop
  }
}

@Composable
fun <K : Any> rememberReorderableLazyColumnState(
  keys: List<K>,
  lazyListState: LazyListState = rememberLazyListState(),
): ReorderableLazyColumnState<K> {
  val edgeAutoScrollController =
    rememberEdgeAutoScrollController(verticalScrollableState = lazyListState)
  val monotonicOrigin = remember { TimeSource.Monotonic.markNow() }
  val state =
    remember(lazyListState, edgeAutoScrollController) {
      ReorderableLazyColumnState(
        lazyListState = lazyListState,
        orderState = ReorderState(keys),
        edgeAutoScrollController = edgeAutoScrollController,
        nowMillis = { monotonicOrigin.elapsedNow().inWholeMilliseconds },
      )
    }

  SideEffect { state.updateInputKeys(keys) }
  DisposableEffect(state) { onDispose { state.dispose() } }
  return state
}

@Composable
fun <K : Any> ReorderableLazyColumn(
  state: ReorderableLazyColumnState<K>,
  modifier: Modifier = Modifier,
  contentPadding: PaddingValues = PaddingValues(0.dp),
  verticalArrangement: Arrangement.Vertical = Arrangement.Top,
  horizontalAlignment: Alignment.Horizontal = Alignment.Start,
  content: LazyListScope.() -> Unit,
) {
  LaunchedEffect(state) {
    snapshotFlow {
        LazyLayoutObservation(
          scrollEpoch = state.edgeAutoScrollController.scrollEpoch,
          verticalScrollDirection = state.edgeAutoScrollController.verticalScrollDirection,
          viewport = state.viewport,
          layoutInfo = state.lazyListState.layoutInfo,
        )
      }
      .collect { observation ->
        val viewport = observation.viewport ?: return@collect
        state.publishLayout(
          layoutInfo = observation.layoutInfo,
          viewport = viewport,
          scrollDirection = observation.verticalScrollDirection,
        )
      }
  }

  LazyColumn(
    state = state.lazyListState,
    modifier =
      modifier.onGloballyPositioned { coordinates ->
        val position = coordinates.positionInWindow()
        val size = coordinates.size
        state.viewport =
          Rect(
            left = position.x,
            top = position.y,
            right = position.x + size.width,
            bottom = position.y + size.height,
          )
      },
    contentPadding = contentPadding,
    verticalArrangement = verticalArrangement,
    horizontalAlignment = horizontalAlignment,
    content = content,
  )
}

@Composable
fun <K : Any> Modifier.reorderableItem(state: ReorderableLazyColumnState<K>, key: K): Modifier {
  val isDragging = state.isDragging(key)
  val settlingOffsetY = state.settlingOffsetY(key)
  val isSettling = settlingOffsetY != null
  val settlingAnim = remember(key, isSettling) { Animatable(settlingOffsetY ?: 0f) }
  val pinnableContainer = LocalPinnableContainer.current

  DisposableEffect(pinnableContainer, isDragging || isSettling) {
    val pinnedHandle = if (isDragging || isSettling) pinnableContainer?.pin() else null
    onDispose { pinnedHandle?.release() }
  }

  LaunchedEffect(key, isSettling) {
    if (!isSettling) {
      settlingAnim.snapTo(0f)
      return@LaunchedEffect
    }
    settlingAnim.animateTo(targetValue = 0f, animationSpec = ReorderReleaseSpring)
    state.clearSettling(key)
  }

  return this.zIndex(if (isDragging) 2f else 0f)
    .onPlaced { coordinates ->
      state.updateItemPlacement(key = key, placedTop = coordinates.positionInWindow().y)
    }
    .layout { measurable, constraints ->
      val placeable = measurable.measure(constraints)
      layout(placeable.width, placeable.height) {
        placeable.placeWithLayer(0, 0) {
          translationY = settlingAnim.value + state.draggedOffsetY(key)
        }
      }
    }
}

@Composable
fun <K : Any> LazyItemScope.reorderableAnimatedItem(
  state: ReorderableLazyColumnState<K>,
  key: K,
  modifier: Modifier = Modifier,
): Modifier {
  val placementSpec =
    if (state.isDragging(key) || state.isSettling(key)) null else ReorderPlacementSpring
  return modifier.animateItem(fadeInSpec = null, placementSpec = placementSpec, fadeOutSpec = null)
}

@Composable
fun Modifier.reorderableViewport(
  state: ReorderableLazyColumnState<*>,
  viewportTopInset: Dp = 0.dp,
  viewportBottomInset: Dp = 0.dp,
): Modifier =
  edgeAutoScroll(
    controller = state.edgeAutoScrollController,
    enabled = state.isDragging,
    viewportTopInset = viewportTopInset,
    viewportBottomInset = viewportBottomInset,
  )

@Composable
fun <K : Any> Modifier.reorderableDragHandle(
  state: ReorderableLazyColumnState<K>,
  key: K,
  enabled: Boolean = true,
  onDragStarted: () -> Unit = {},
  onDragMoved: () -> Unit = {},
  onDragStopped: (drop: ReorderDrop<K>?) -> Unit = {},
): Modifier {
  val haptic = LocalHapticFeedback.current
  val scrollGestureLockState = LocalScrollGestureLockState.current
  val releaseScope = rememberCoroutineScope()
  val onDragMovedUpdated by rememberUpdatedState(onDragMoved)
  val onDragStoppedUpdated by rememberUpdatedState(onDragStopped)
  var handleCoordinates by remember { mutableStateOf<LayoutCoordinates?>(null) }

  return this.onGloballyPositioned { coordinates -> handleCoordinates = coordinates }
    .pointerInput(state, key, enabled, scrollGestureLockState) {
      if (!enabled) return@pointerInput

      awaitEachGesture {
        var scrollLockHandle: ScrollGestureLockHandle? = null
        var started = false
        var releasePending = false

        try {
          val down = awaitFirstDown(requireUnconsumed = false)
          val pointerId = down.id
          val originWindow =
            handleCoordinates?.localToWindow(down.position) ?: return@awaitEachGesture
          var currentWindow = originWindow
          started =
            state.beginDrag(
              key = key,
              pointer = originWindow,
              onTargetChanged = { onDragMovedUpdated() },
              onTargetHaptic = {
                haptic.performHapticFeedback(HapticFeedbackType.SegmentFrequentTick)
              },
              onDragStopped = { drop -> onDragStoppedUpdated(drop) },
            )

          if (started) {
            scrollLockHandle = scrollGestureLockState.acquire()
            down.consume()
            haptic.performHapticFeedback(HapticFeedbackType.GestureThresholdActivate)
            onDragStarted()
          }

          while (true) {
            val event = awaitPointerEvent()
            val change = event.changes.find { it.id == pointerId } ?: break
            currentWindow += change.positionChangeIgnoreConsumed()

            if (!change.pressed) {
              if (started && state.isDragging(key)) {
                state.updatePointer(currentWindow)
                releasePending = true
                state.edgeAutoScrollController.pointer = null
                releaseScope.launch {
                  try {
                    state.awaitCurrentDraggedPlacement()
                    if (state.isDragging(key)) {
                      state.release()
                      haptic.performHapticFeedback(HapticFeedbackType.GestureEnd)
                    }
                  } finally {
                    if (state.isDragging(key)) state.cancel()
                  }
                }
              }
              break
            }

            if (started && state.isDragging(key)) {
              change.consume()
              state.updatePointer(currentWindow)
            }
          }
        } finally {
          if (started && !releasePending && state.isDragging(key)) state.cancel()
          scrollLockHandle?.release()
        }
      }
    }
}

private val ReorderReleaseSpring =
  spring<Float>(dampingRatio = 0.9f, stiffness = Spring.StiffnessMedium)

private val ReorderPlacementSpring =
  spring(
    dampingRatio = 0.9f,
    stiffness = Spring.StiffnessMedium,
    visibilityThreshold = IntOffset.VisibilityThreshold,
  )
