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
import kotlin.math.roundToInt
import kotlin.time.TimeSource
import kotlinx.coroutines.launch

private data class LazyLayoutObservation(
  val scrollEpoch: Long,
  val verticalScrollDirection: Int,
  val viewport: Rect?,
  val layoutInfo: LazyListLayoutInfo,
)

private data class ReorderReleasePresentation<K : Any>(
  val siblingTargetOffsets: Map<K, Float>,
  val layoutCommitPending: Boolean,
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
  private var draggedHeight by mutableFloatStateOf(0f)
  private var itemSpacing by mutableFloatStateOf(0f)
  private var dragSessionRevision by mutableLongStateOf(0L)
  private var releasePresentation by mutableStateOf<ReorderReleasePresentation<K>?>(null)
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

  val layoutKeys: List<K>
    get() {
      orderEpoch
      return orderState.layoutKeys
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

  internal val currentDragSessionRevision: Long
    get() = dragSessionRevision

  internal val suppressLazyPlacementAnimation: Boolean
    get() = isDragging || releasePresentation?.layoutCommitPending == true

  internal val isReleaseCommitPending: Boolean
    get() = releasePresentation?.layoutCommitPending == true

  internal fun releaseSiblingTargetOffsetY(key: K): Float =
    releasePresentation?.siblingTargetOffsets?.get(key) ?: 0f

  internal fun siblingTargetOffsetY(key: K): Float {
    orderEpoch
    val draggedKey = orderState.draggingKey ?: return 0f
    return reorderItemDisplacement(
      layoutKeys = orderState.layoutKeys,
      projectedKeys = orderState.keys,
      draggedKey = draggedKey,
      itemKey = key,
      draggedHeight = draggedHeight,
      itemSpacing = itemSpacing,
    )
  }

  internal fun updateInputKeys(keys: List<K>) {
    if (orderState.inputKeys != keys) {
      val retainedKeys = keys.toSet()
      placedItemTops.keys.retainAll(retainedKeys)
    }
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
    val displayedKeys = orderState.layoutKeys
    val displayedKeyByValue = displayedKeys.associateBy { it as Any }
    val snapshot =
      ReorderLayoutSnapshot(
        viewportTop = viewport.top + layoutInfo.viewportStartOffset,
        viewportBottom = viewport.top + layoutInfo.viewportEndOffset,
        itemSpacing = layoutInfo.mainAxisItemSpacing.toFloat(),
        items =
          layoutInfo.visibleItemsInfo.mapNotNull { item ->
            val key = displayedKeyByValue[item.key] ?: return@mapNotNull null
            val top = viewport.top + item.offset
            ReorderLayoutItem(
              key = key,
              lazyIndex = item.index,
              top = top,
              bottom = top + item.size,
            )
          },
      )
    if (!orderState.isDragging) {
      latestIdleLayout = snapshot
      interaction.publishLayout(snapshot)
      val currentReleasePresentation = releasePresentation
      if (
        currentReleasePresentation?.layoutCommitPending == true &&
          discoverReorderBlockOffset(orderState.layoutKeys, snapshot) != null
      ) {
        releasePresentation = currentReleasePresentation.copy(layoutCommitPending = false)
      }
      return
    }

    itemSpacing = snapshot.itemSpacing

    val layoutProposal = interaction.publishLayout(snapshot, scrollDirection)
    val placementProposal =
      interaction.draggedItemInCurrentSourceLayout()?.let { draggedItem ->
        draggedHeight = draggedItem.height
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
    dragSessionRevision += 1
    releasePresentation = null
    draggedHeight = item.height
    itemSpacing = latestIdleLayout?.itemSpacing ?: 0f
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
    val moved = orderState.keys != orderState.layoutKeys
    val releaseOffsetY = releaseSettlingOffsetY(moved)
    if (releaseOffsetY == null) {
      cancel()
      return
    }
    if (moved) {
      prepareReleaseCommit()
    } else {
      releasePresentation = null
    }
    val drop =
      try {
        endingDrag = true
        interaction.release(releaseOffsetY = releaseOffsetY)
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
    releasePresentation = null
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

    if (!interaction.commitTarget(proposal)) return
    refreshDraggedVisualOffset()
    onTargetChanged?.invoke()
    if (hapticPolicy.shouldEmit(proposal.targetIndex, nowMillis())) onTargetHaptic?.invoke()
  }

  private fun releaseSettlingOffsetY(moved: Boolean): Float? {
    if (!moved) return draggedVisualOffsetY
    val currentVisualTop =
      interaction.draggedTopY(orderState.draggingKey)
        ?: draggedPlacedTopY?.plus(draggedVisualOffsetY)
    val destinationTop = interaction.draggedDestinationTopY()
    if (currentVisualTop == null || destinationTop == null) return null
    return currentVisualTop - destinationTop
  }

  private fun prepareReleaseCommit() {
    val draggedKey = checkNotNull(orderState.draggingKey)
    releasePresentation =
      ReorderReleasePresentation(
        siblingTargetOffsets =
          orderState.layoutKeys.associateWith { key ->
            reorderItemDisplacement(
              layoutKeys = orderState.layoutKeys,
              projectedKeys = orderState.keys,
              draggedKey = draggedKey,
              itemKey = key,
              draggedHeight = draggedHeight,
              itemSpacing = itemSpacing,
            )
          },
        layoutCommitPending = true,
      )
    val viewportTop = viewport?.top
    val projectedAnchor = interaction.projectedViewportAnchor()
    if (viewportTop != null && projectedAnchor != null) {
      lazyListState.requestScrollToItem(
        index = projectedAnchor.lazyIndex,
        scrollOffset = -(projectedAnchor.top - viewportTop).roundToInt(),
      )
    }
  }

  private fun finishDrag(drop: ReorderDrop<K>?) {
    edgeAutoScrollController.pointer = null
    latestIdleLayout = null
    draggedVisualOffsetY = 0f
    draggedHeight = 0f
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
  val isAnyDragging = state.isDragging
  val settlingOffsetY = state.settlingOffsetY(key)
  val isSettling = settlingOffsetY != null
  val settlingAnim = remember(key, isSettling) { Animatable(settlingOffsetY ?: 0f) }
  val dragSessionRevision = state.currentDragSessionRevision
  val siblingTargetOffsetY = state.siblingTargetOffsetY(key)
  val siblingAnim = remember(key, dragSessionRevision) { Animatable(0f) }
  var siblingHandoffBaseOffsetY by
    remember(key, dragSessionRevision) { mutableStateOf<Float?>(null) }
  var siblingHandoffPending by remember(key, dragSessionRevision) { mutableStateOf(false) }
  val isReleaseCommitPending = state.isReleaseCommitPending
  val releaseSiblingTargetOffsetY = state.releaseSiblingTargetOffsetY(key)
  val siblingHandoffBase = siblingHandoffBaseOffsetY
  val siblingOffsetY =
    when {
      isAnyDragging && !isDragging -> siblingAnim.value
      isReleaseCommitPending -> siblingAnim.value - releaseSiblingTargetOffsetY
      siblingHandoffBase != null -> siblingAnim.value - siblingHandoffBase
      siblingHandoffPending -> siblingAnim.value
      else -> 0f
    }
  val pinnableContainer = LocalPinnableContainer.current
  val shouldPin = isDragging || isSettling

  SideEffect {
    if (isAnyDragging) {
      siblingHandoffPending = true
    }
  }

  DisposableEffect(pinnableContainer, shouldPin) {
    val pinnedHandle = if (shouldPin) pinnableContainer?.pin() else null
    onDispose { pinnedHandle?.release() }
  }

  LaunchedEffect(key, dragSessionRevision, isAnyDragging, siblingTargetOffsetY) {
    if (isAnyDragging) {
      siblingHandoffBaseOffsetY = null
      siblingAnim.animateTo(
        targetValue = siblingTargetOffsetY,
        animationSpec = ReorderSiblingSpring,
      )
      return@LaunchedEffect
    }
    val handoffBaseOffsetY = if (isReleaseCommitPending) releaseSiblingTargetOffsetY else 0f
    siblingHandoffBaseOffsetY = handoffBaseOffsetY
    siblingHandoffPending = false
    siblingAnim.animateTo(targetValue = handoffBaseOffsetY, animationSpec = ReorderSiblingSpring)
    siblingHandoffBaseOffsetY = null
  }

  LaunchedEffect(key, isSettling) {
    if (!isSettling) {
      settlingAnim.snapTo(0f)
      return@LaunchedEffect
    }
    settlingAnim.animateTo(targetValue = 0f, animationSpec = ReorderReleaseSpring)
    state.clearSettling(key)
  }

  return this.zIndex(if (isDragging || isSettling) 2f else 0f)
    .onPlaced { coordinates ->
      state.updateItemPlacement(key = key, placedTop = coordinates.positionInWindow().y)
    }
    .layout { measurable, constraints ->
      val placeable = measurable.measure(constraints)
      layout(placeable.width, placeable.height) {
        placeable.placeWithLayer(0, 0) {
          translationY = settlingAnim.value + state.draggedOffsetY(key) + siblingOffsetY
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
    if (state.suppressLazyPlacementAnimation || state.isSettling(key)) {
      null
    } else {
      ReorderPlacementSpring
    }
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

private val ReorderSiblingSpring =
  spring<Float>(dampingRatio = 0.9f, stiffness = Spring.StiffnessMedium)

private val ReorderPlacementSpring =
  spring(
    dampingRatio = 0.9f,
    stiffness = Spring.StiffnessMedium,
    visibilityThreshold = IntOffset.VisibilityThreshold,
  )
