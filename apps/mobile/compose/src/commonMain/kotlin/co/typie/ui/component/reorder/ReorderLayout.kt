package co.typie.ui.component.reorder

internal data class ReorderLayoutItem<K : Any>(
  val key: K,
  val lazyIndex: Int,
  val top: Float,
  val bottom: Float,
) {
  val height: Float
    get() = bottom - top
}

internal data class ReorderLayoutSnapshot<K : Any>(
  val viewportTop: Float,
  val viewportBottom: Float,
  val items: List<ReorderLayoutItem<K>>,
)

internal fun <K : Any> discoverReorderBlockOffset(
  displayedKeys: List<K>,
  snapshot: ReorderLayoutSnapshot<K>,
): Int? {
  val offsets =
    snapshot.items.mapNotNull { item ->
      val displayedIndex = displayedKeys.indexOf(item.key)
      if (displayedIndex == -1) null else item.lazyIndex - displayedIndex
    }
  if (offsets.isEmpty()) return null
  val offset = offsets.first()
  return offset.takeIf { candidate -> offsets.all { it == candidate } }
}

internal fun <K : Any> isCompatibleReorderSnapshot(
  displayedKeys: List<K>,
  blockOffset: Int,
  snapshot: ReorderLayoutSnapshot<K>,
  ignoredKey: K? = null,
): Boolean =
  snapshot.items.all { item ->
    if (item.key == ignoredKey) return@all true
    val displayedIndex = displayedKeys.indexOf(item.key)
    displayedIndex != -1 && item.lazyIndex == blockOffset + displayedIndex
  }

internal fun <K : Any> projectReorderSnapshotOntoDisplayedSlots(
  displayedKeys: List<K>,
  blockOffset: Int,
  snapshot: ReorderLayoutSnapshot<K>,
): ReorderLayoutSnapshot<K> =
  snapshot.copy(
    items =
      snapshot.items
        .mapNotNull { item ->
          val displayedIndex = item.lazyIndex - blockOffset
          val displayedKey = displayedKeys.getOrNull(displayedIndex) ?: return@mapNotNull null
          item.copy(key = displayedKey)
        }
        .groupBy(ReorderLayoutItem<K>::lazyIndex)
        .values
        .map { candidates ->
          candidates.maxBy { item ->
            verticalOverlap(item.top, item.bottom, snapshot.viewportTop, snapshot.viewportBottom)
          }
        }
        .sortedBy(ReorderLayoutItem<K>::lazyIndex)
  )

internal fun <K : Any> targetIndexForDrag(
  displayedKeys: List<K>,
  draggedKey: K,
  direction: Int,
  draggedTop: Float,
  draggedBottom: Float,
  blockOffset: Int,
  snapshot: ReorderLayoutSnapshot<K>,
): Int {
  val currentIndex = displayedKeys.indexOf(draggedKey)
  if (currentIndex == -1 || direction == 0) return currentIndex
  if (
    !isCompatibleReorderSnapshot(
      displayedKeys = displayedKeys,
      blockOffset = blockOffset,
      snapshot = snapshot,
      ignoredKey = draggedKey,
    )
  ) {
    return currentIndex
  }

  val draggedHeight = draggedBottom - draggedTop
  if (draggedHeight <= 0f) return currentIndex
  val itemsByKey = snapshot.items.associateBy { it.key }
  val publishedIndices =
    snapshot.items.mapNotNull { item ->
      if (item.key == draggedKey) null else displayedKeys.indexOf(item.key).takeIf { it >= 0 }
    }
  val firstPublishedIndex = publishedIndices.minOrNull()
  val lastPublishedIndex = publishedIndices.maxOrNull()
  var targetIndex = currentIndex

  if (direction > 0) {
    while (targetIndex < displayedKeys.lastIndex) {
      val nextIndex = targetIndex + 1
      val next = itemsByKey[displayedKeys[nextIndex]]
      if (
        next == null &&
          firstPublishedIndex != null &&
          nextIndex < firstPublishedIndex &&
          draggedTop >= snapshot.viewportTop
      ) {
        targetIndex = nextIndex
      } else if (next != null && crossesNext(draggedTop, draggedBottom, draggedHeight, next)) {
        targetIndex += 1
      } else {
        break
      }
    }
  } else {
    while (targetIndex > 0) {
      val previousIndex = targetIndex - 1
      val previous = itemsByKey[displayedKeys[previousIndex]]
      if (
        previous == null &&
          lastPublishedIndex != null &&
          previousIndex > lastPublishedIndex &&
          draggedBottom <= snapshot.viewportBottom
      ) {
        targetIndex = previousIndex
      } else if (
        previous != null && crossesPrevious(draggedTop, draggedBottom, draggedHeight, previous)
      ) {
        targetIndex -= 1
      } else {
        break
      }
    }
  }

  return targetIndex
}

private fun crossesNext(
  draggedTop: Float,
  draggedBottom: Float,
  draggedHeight: Float,
  next: ReorderLayoutItem<*>,
): Boolean {
  val requiredOverlap = minOf(draggedHeight, next.height) / 2f
  return verticalOverlap(draggedTop, draggedBottom, next.top, next.bottom) > requiredOverlap ||
    draggedTop >= next.bottom
}

private fun crossesPrevious(
  draggedTop: Float,
  draggedBottom: Float,
  draggedHeight: Float,
  previous: ReorderLayoutItem<*>,
): Boolean {
  val requiredOverlap = minOf(draggedHeight, previous.height) / 2f
  return verticalOverlap(draggedTop, draggedBottom, previous.top, previous.bottom) >
    requiredOverlap || draggedBottom <= previous.top
}

private fun verticalOverlap(
  firstTop: Float,
  firstBottom: Float,
  secondTop: Float,
  secondBottom: Float,
): Float = (minOf(firstBottom, secondBottom) - maxOf(firstTop, secondTop)).coerceAtLeast(0f)
