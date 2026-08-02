package co.typie.ui.component.reorder

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
      blockOffset = discoverReorderBlockOffset(orderState.keys, snapshot)
      return null
    }

    val currentBlockOffset = blockOffset ?: return null
    sourceSnapshot = snapshot
    layoutSnapshot =
      projectReorderSnapshotOntoDisplayedSlots(
        displayedKeys = orderState.keys,
        blockOffset = currentBlockOffset,
        snapshot = snapshot,
      )
    return proposeTarget(scrollDirection)
  }

  fun draggedItemInPublishedLayout(): ReorderLayoutItem<K>? =
    layoutSnapshot?.items?.firstOrNull { it.key == orderState.draggingKey }

  fun commitTarget(proposal: ReorderTargetProposal<K>): Boolean {
    if (orderState.draggingKey != proposal.draggedKey) return false
    if (orderState.orderRevision != proposal.sourceOrderRevision) return false
    val changed = orderState.moveDraggedTo(proposal.targetIndex)
    if (changed) {
      val currentBlockOffset = blockOffset
      layoutSnapshot =
        if (currentBlockOffset == null) {
          null
        } else {
          sourceSnapshot?.let { snapshot ->
            projectReorderSnapshotOntoDisplayedSlots(
              displayedKeys = orderState.keys,
              blockOffset = currentBlockOffset,
              snapshot = snapshot,
            )
          }
        }
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

  private fun clearDragInputs() {
    pointerY = null
    draggedHeight = null
    direction = 0
    layoutSnapshot = null
    sourceSnapshot = null
  }
}

private const val DragDirectionEpsilonPx = 0.5f
