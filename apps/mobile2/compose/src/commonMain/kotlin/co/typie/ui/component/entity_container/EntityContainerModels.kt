package co.typie.ui.component.entity_container

import co.typie.ui.component.EntityListItem

data class OrderedEntityItem(val id: String, val order: String, val item: EntityListItem)

data class EntityReorderOrders(val lowerOrder: String?, val upperOrder: String?)

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

private fun List<OrderedEntityItem>.isCompleteEntityOrder(expectedSize: Int): Boolean {
  return size == expectedSize && map { it.id }.toSet().size == expectedSize
}
