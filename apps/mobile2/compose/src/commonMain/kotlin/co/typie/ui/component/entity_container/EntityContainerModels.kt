package co.typie.ui.component.entity_container

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.entity_transfer.EntityTransferSource
import co.typie.ui.component.EntityListItem

data class OrderedEntityItem(val id: String, val order: String, val item: EntityListItem)

data class EntityReorderOrders(val lowerOrder: String?, val upperOrder: String?)

data class EntityContainerSelectionState(
  val isSelecting: Boolean = false,
  val selectedIds: Set<String> = emptySet(),
)

data class EntityContainerSelectionSummary(
  val selectedItems: List<OrderedEntityItem>,
  val folderItems: List<EntityListItem.Folder>,
  val documentItems: List<EntityListItem.Document>,
  val commonIconName: String?,
  val commonIconColor: String?,
)

data class EntityContainerBottomOverlayMetrics(
  val occupiedHeight: Dp,
  val reservedSpacerHeight: Dp,
  val toastBottomInset: Dp,
)

class EntityContainerSelection(
  val state: EntityContainerSelectionState,
  val summary: EntityContainerSelectionSummary,
  private val onStateChange: (EntityContainerSelectionState) -> Unit,
) {
  val isSelectionBarVisible: Boolean = state.isSelecting && summary.selectedItems.isNotEmpty()

  fun start(initialIds: Set<String> = emptySet()) {
    onStateChange(startEntityContainerSelection(initialIds))
  }

  fun clear() {
    onStateChange(clearEntityContainerSelection(state))
  }

  fun reset() {
    onStateChange(EntityContainerSelectionState())
  }

  fun toggle(itemId: String) {
    onStateChange(toggleEntityContainerSelection(state, itemId))
  }
}

internal val EntityContainerBottomOverlayGap = 8.dp
internal val EntityContainerBottomOverlayBottomOffset = 44.dp
internal val EntityContainerBottomOverlayReserveExtra = 12.dp
internal val EntityContainerDefaultBottomSpacerHeight = 140.dp

@Composable
fun rememberEntityContainerSelection(items: List<OrderedEntityItem>): EntityContainerSelection {
  var state by remember { mutableStateOf(EntityContainerSelectionState()) }
  val summary =
    remember(items, state.selectedIds) {
      resolveEntityContainerSelectionSummary(items, state.selectedIds)
    }

  return remember(state, summary) { EntityContainerSelection(state, summary) { state = it } }
}

fun displayOrderedEntityItems(
  items: List<OrderedEntityItem>,
  orderedKeys: List<String>,
): List<OrderedEntityItem> {
  val itemsById = items.associateBy { it.id }
  if (orderedKeys.size != itemsById.size) {
    return items
  }
  val orderedItems = orderedKeys.mapNotNull(itemsById::get)
  return if (orderedItems.isCompleteEntityOrder(itemsById.size)) orderedItems else items
}

fun calculateEntityReorderOrdersFromOrderedKeys(
  items: List<OrderedEntityItem>,
  orderedKeys: List<String>,
  movedKey: String,
): EntityReorderOrders? {
  val itemsById = items.associateBy { it.id }
  if (orderedKeys.size != itemsById.size) {
    return null
  }
  val orderedItems = orderedKeys.mapNotNull(itemsById::get)
  if (!orderedItems.isCompleteEntityOrder(itemsById.size)) {
    return null
  }

  val movedIndex = orderedItems.indexOfFirst { item -> item.id == movedKey }
  if (movedIndex == -1) {
    return null
  }

  return EntityReorderOrders(
    lowerOrder = orderedItems.getOrNull(movedIndex - 1)?.order,
    upperOrder = orderedItems.getOrNull(movedIndex + 1)?.order,
  )
}

fun resolveEntityContainerSelectionSummary(
  items: List<OrderedEntityItem>,
  selectedIds: Set<String>,
): EntityContainerSelectionSummary {
  val selectedItems = items.filter { it.id in selectedIds }
  val folderItems = selectedItems.mapNotNull { it.item as? EntityListItem.Folder }
  val documentItems = selectedItems.mapNotNull { it.item as? EntityListItem.Document }
  val commonIconName =
    selectedItems.map { it.item.iconName }.distinct().singleOrNull()?.takeIf { it.isNotBlank() }
  val commonIconColor =
    selectedItems.map { it.item.iconColor }.distinct().singleOrNull()?.takeIf { it.isNotBlank() }

  return EntityContainerSelectionSummary(
    selectedItems = selectedItems,
    folderItems = folderItems,
    documentItems = documentItems,
    commonIconName = commonIconName,
    commonIconColor = commonIconColor,
  )
}

fun startEntityContainerSelection(
  initialIds: Set<String> = emptySet()
): EntityContainerSelectionState {
  return EntityContainerSelectionState(isSelecting = true, selectedIds = initialIds)
}

fun clearEntityContainerSelection(
  state: EntityContainerSelectionState
): EntityContainerSelectionState {
  return state.copy(selectedIds = emptySet())
}

fun toggleEntityContainerSelection(
  state: EntityContainerSelectionState,
  itemId: String,
): EntityContainerSelectionState {
  val nextSelectedIds = state.selectedIds.toMutableSet()
  if (!nextSelectedIds.add(itemId)) {
    nextSelectedIds.remove(itemId)
  }

  return state.copy(selectedIds = nextSelectedIds)
}

fun calculateEntityContainerBottomOverlayMetrics(
  baseBottomInset: Dp,
  hasPasteBar: Boolean,
  pasteBarHeight: Dp,
  hasSelectionBar: Boolean,
  selectionBarHeight: Dp,
): EntityContainerBottomOverlayMetrics {
  val visibleHeights = buildList {
    if (hasSelectionBar) add(selectionBarHeight)
    if (hasPasteBar) add(pasteBarHeight)
  }
  if (visibleHeights.isEmpty()) {
    return EntityContainerBottomOverlayMetrics(
      occupiedHeight = 0.dp,
      reservedSpacerHeight = EntityContainerDefaultBottomSpacerHeight,
      toastBottomInset = baseBottomInset,
    )
  }

  val stackHeight =
    visibleHeights.fold(0.dp) { total, height -> total + height } +
      EntityContainerBottomOverlayGap * (visibleHeights.size - 1)
  val occupiedHeight = baseBottomInset + EntityContainerBottomOverlayBottomOffset + stackHeight

  return EntityContainerBottomOverlayMetrics(
    occupiedHeight = occupiedHeight,
    reservedSpacerHeight =
      maxOf(
        EntityContainerDefaultBottomSpacerHeight,
        occupiedHeight + EntityContainerBottomOverlayReserveExtra,
      ),
    toastBottomInset = baseBottomInset + stackHeight + EntityContainerBottomOverlayGap,
  )
}

fun resolveEntityContainerTransferSources(
  summary: EntityContainerSelectionSummary
): List<EntityTransferSource> {
  return summary.selectedItems.map { selectedItem ->
    when (val item = selectedItem.item) {
      is EntityListItem.Document ->
        EntityTransferSource.Document(id = item.id, title = item.title, depth = item.depth)
      is EntityListItem.Folder ->
        EntityTransferSource.Folder(
          id = item.id,
          title = item.name,
          depth = item.depth,
          maxDescendantFoldersDepth = item.maxDescendantFoldersDepth,
        )
    }
  }
}

private fun List<OrderedEntityItem>.isCompleteEntityOrder(expectedSize: Int): Boolean {
  return size == expectedSize && map { it.id }.toSet().size == expectedSize
}
