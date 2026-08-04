package co.typie.ui.component.reorder

import kotlin.math.abs

internal data class ReorderTargetProposal<K : Any>(
  val draggedKey: K,
  val sourceOrderRevision: Long,
  val targetIndex: Int,
)

internal class ReorderInteraction<K : Any>(val orderState: ReorderState<K>) {
  private var blockOffset: Int? = null
  private var layoutSnapshot: ReorderLayoutSnapshot<K>? = null
  private var sourceSnapshot: ReorderLayoutSnapshot<K>? = null
  private var pointerY: Float? = null
  private var pointerOffsetInItemY = 0f
  private var draggedHeight: Float? = null
  private var draggedSlotTopY: Float? = null
  private var direction = 0

  fun beginDrag(key: K, pointerY: Float, pointerOffsetInItemY: Float): Boolean {
    if (blockOffset == null) {
      blockOffset = layoutSnapshot?.let {
        discoverReorderBlockOffset(orderState.keys, snapshot = it)
      }
    }
    if (blockOffset == null || !orderState.beginDrag(key)) return false

    this.pointerY = pointerY
    this.pointerOffsetInItemY = pointerOffsetInItemY
    draggedHeight = null
    draggedSlotTopY = layoutSnapshot?.items?.firstOrNull { it.key == key }?.top
    direction = 0
    return true
  }

  fun updatePointer(pointerY: Float): ReorderTargetProposal<K>? {
    if (!orderState.isDragging) return null
    val previousPointerY = this.pointerY
    direction =
      when {
        previousPointerY == null -> direction
        pointerY > previousPointerY + DragDirectionEpsilonPx -> 1
        pointerY < previousPointerY - DragDirectionEpsilonPx -> -1
        else -> direction
      }
    this.pointerY = pointerY
    return proposeTarget()
  }

  fun updateDraggedSize(height: Float, scrollDirection: Int = 0): ReorderTargetProposal<K>? {
    if (!orderState.isDragging) return null
    draggedHeight = height
    return proposeTarget(scrollDirection)
  }

  fun publishLayout(
    snapshot: ReorderLayoutSnapshot<K>,
    scrollDirection: Int = 0,
  ): ReorderTargetProposal<K>? {
    if (!orderState.isDragging) {
      layoutSnapshot = snapshot
      sourceSnapshot = snapshot
      draggedSlotTopY = null
      blockOffset = discoverReorderBlockOffset(orderState.keys, snapshot)
      return null
    }

    val currentBlockOffset = blockOffset ?: return null
    val previousSourceSnapshot = sourceSnapshot
    val projection = projectSnapshot(snapshot, currentBlockOffset)
    sourceSnapshot = snapshot
    layoutSnapshot = projection.snapshot
    updateDraggedSlotTop(
      projectedSlotTop = projection.draggedSlotTop,
      previousSourceSnapshot = previousSourceSnapshot,
      currentSourceSnapshot = snapshot,
    )
    return proposeTarget(scrollDirection)
  }

  fun draggedItemInCurrentSourceLayout(): ReorderLayoutItem<K>? {
    val draggedKey = orderState.draggingKey ?: return null
    val layoutIndex = orderState.layoutKeys.indexOf(draggedKey)
    val currentBlockOffset = blockOffset
    if (layoutIndex == -1 || currentBlockOffset == null) return null
    val expectedLazyIndex = currentBlockOffset + layoutIndex
    return sourceSnapshot?.items?.firstOrNull { item ->
      item.key == draggedKey && item.lazyIndex == expectedLazyIndex
    }
  }

  fun draggedDestinationTopY(): Float? = draggedSlotTopY.takeIf { orderState.isDragging }

  fun projectedViewportAnchor(): ReorderLayoutItem<K>? {
    val snapshot = layoutSnapshot ?: return null
    return snapshot.items
      .filter { item -> item.bottom > snapshot.viewportTop && item.top < snapshot.viewportBottom }
      .minByOrNull(ReorderLayoutItem<K>::top)
  }

  fun commitTarget(proposal: ReorderTargetProposal<K>): Boolean {
    if (orderState.draggingKey != proposal.draggedKey) return false
    if (orderState.orderRevision != proposal.sourceOrderRevision) return false
    val changed = orderState.moveDraggedTo(proposal.targetIndex)
    if (changed) {
      val currentBlockOffset = blockOffset
      val projection =
        if (currentBlockOffset == null) {
          null
        } else {
          sourceSnapshot?.let { snapshot -> projectSnapshot(snapshot, currentBlockOffset) }
        }
      layoutSnapshot = projection?.snapshot
      projection?.draggedSlotTop?.let { draggedSlotTopY = it }
    }
    return changed
  }

  fun release(releaseOffsetY: Float = 0f): ReorderDrop<K>? {
    if (!orderState.isDragging) {
      clearDragInputs()
      return null
    }
    val drop = orderState.endDrag(releaseOffsetY)
    clearDragInputs()
    return drop
  }

  fun cancel() {
    orderState.cancelDrag()
    clearDragInputs()
  }

  fun acknowledgeExternalCancellation() {
    if (!orderState.isDragging) clearDragInputs()
  }

  fun updateInputKeys(keys: List<K>) {
    orderState.inputKeys = keys
    if (!orderState.isDragging) clearDragInputs()
  }

  fun draggedTopY(key: K?): Float? {
    if (key == null || orderState.draggingKey != key) return null
    val pointerY = pointerY ?: return null
    return pointerY - pointerOffsetInItemY
  }

  private fun proposeTarget(scrollDirection: Int = 0): ReorderTargetProposal<K>? {
    val draggedKey = orderState.draggingKey ?: return null
    val currentPointerY = pointerY ?: return null
    val currentHeight = draggedHeight ?: return null
    val currentBlockOffset = blockOffset ?: return null
    val currentLayout = layoutSnapshot ?: return null
    val draggedTop = currentPointerY - pointerOffsetInItemY
    val resolvedDirection = scrollDirection.takeUnless { it == 0 } ?: direction
    val targetIndex =
      targetIndexForDrag(
        displayedKeys = orderState.keys,
        draggedKey = draggedKey,
        direction = resolvedDirection,
        draggedTop = draggedTop,
        draggedBottom = draggedTop + currentHeight,
        blockOffset = currentBlockOffset,
        snapshot = currentLayout,
      )
    if (targetIndex == orderState.keys.indexOf(draggedKey)) return null
    return ReorderTargetProposal(
      draggedKey = draggedKey,
      sourceOrderRevision = orderState.orderRevision,
      targetIndex = targetIndex,
    )
  }

  private fun projectSnapshot(
    snapshot: ReorderLayoutSnapshot<K>,
    blockOffset: Int,
  ): ReorderLayoutProjection<K> {
    val draggedKey =
      orderState.draggingKey
        ?: return ReorderLayoutProjection(snapshot = snapshot, draggedSlotTop = null)
    val currentHeight =
      draggedHeight ?: return ReorderLayoutProjection(snapshot = snapshot, draggedSlotTop = null)
    return projectReorderLayoutFromStableSource(
      layoutKeys = orderState.layoutKeys,
      projectedKeys = orderState.keys,
      draggedKey = draggedKey,
      draggedHeight = currentHeight,
      itemSpacing = snapshot.itemSpacing,
      blockOffset = blockOffset,
      snapshot = snapshot,
    )
  }

  private fun updateDraggedSlotTop(
    projectedSlotTop: Float?,
    previousSourceSnapshot: ReorderLayoutSnapshot<K>?,
    currentSourceSnapshot: ReorderLayoutSnapshot<K>,
  ) {
    draggedSlotTopY =
      projectedSlotTop
        ?: draggedSlotTopY?.let { previousSlotTop ->
          sourceLayoutTranslation(
              previousSnapshot = previousSourceSnapshot,
              currentSnapshot = currentSourceSnapshot,
              referenceY = previousSlotTop,
            )
            ?.let(previousSlotTop::plus) ?: previousSlotTop
        }
  }

  private fun sourceLayoutTranslation(
    previousSnapshot: ReorderLayoutSnapshot<K>?,
    currentSnapshot: ReorderLayoutSnapshot<K>,
    referenceY: Float,
  ): Float? =
    previousSnapshot
      ?.items
      ?.mapNotNull { previousItem ->
        currentSnapshot.items
          .firstOrNull { currentItem ->
            currentItem.key == previousItem.key && currentItem.lazyIndex == previousItem.lazyIndex
          }
          ?.let { currentItem ->
            abs(previousItem.top - referenceY) to (currentItem.top - previousItem.top)
          }
      }
      ?.minByOrNull { (distanceFromSlot) -> distanceFromSlot }
      ?.second

  private fun clearDragInputs() {
    pointerY = null
    draggedHeight = null
    draggedSlotTopY = null
    direction = 0
    layoutSnapshot = null
    sourceSnapshot = null
  }
}

private const val DragDirectionEpsilonPx = 0.5f
