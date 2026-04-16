package co.typie.domain.entity

import co.typie.graphql.fragment.EntityRow_entity

fun displayEntityRows(
  items: List<EntityRow_entity>,
  orderedKeys: List<String>,
): List<EntityRow_entity> {
  val itemsById = items.associateBy { it.id }
  if (orderedKeys.size != itemsById.size) {
    return items
  }

  val orderedItems = orderedKeys.mapNotNull(itemsById::get)
  return if (orderedItems.isCompleteEntityOrder(itemsById.size)) orderedItems else items
}

fun calculateEntityReorderOrdersFromOrderedKeys(
  items: List<EntityRow_entity>,
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
  items: List<EntityRow_entity>,
  selectedIds: Set<String>,
): EntityContainerSelectionSummary {
  val selectedItems = items.filter { it.id in selectedIds }
  val folderItems = selectedItems.filter { it.folder != null }
  val documentItems = selectedItems.filter { it.document != null }
  val commonIconName =
    selectedItems
      .map { it.entityIcon_entity.icon }
      .distinct()
      .singleOrNull()
      ?.takeIf { it.isNotBlank() }
  val commonIconColor =
    selectedItems
      .map { it.entityIcon_entity.iconColor }
      .distinct()
      .singleOrNull()
      ?.takeIf { it.isNotBlank() }

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

private fun List<EntityRow_entity>.isCompleteEntityOrder(expectedSize: Int): Boolean {
  return size == expectedSize && map { it.id }.toSet().size == expectedSize
}
