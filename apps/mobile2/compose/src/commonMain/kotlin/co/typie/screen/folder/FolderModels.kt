package co.typie.screen.folder

import co.typie.graphql.FolderScreen_Query
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.entity_container.EntityReorderOrders
import co.typie.ui.component.entity_container.OrderedEntityItem
import co.typie.ui.component.entity_container.calculateEntityReorderOrdersFromOrderedKeys
import co.typie.ui.component.entity_container.displayOrderedEntityItems

typealias NormalizedFolderChild = OrderedEntityItem
typealias FolderReorderOrders = EntityReorderOrders

fun normalizeFolderChildren(
  children: List<FolderScreen_Query.Child>,
): List<NormalizedFolderChild> {
  return children.mapNotNull(::normalizeFolderChild)
}

fun displayFolderChildren(
  items: List<NormalizedFolderChild>,
  orderedKeys: List<String>,
): List<NormalizedFolderChild> {
  return displayOrderedEntityItems(items, orderedKeys)
}

fun calculateFolderReorderOrdersFromOrderedKeys(
  items: List<NormalizedFolderChild>,
  orderedKeys: List<String>,
  movedKey: String,
): FolderReorderOrders? {
  return calculateEntityReorderOrdersFromOrderedKeys(items, orderedKeys, movedKey)
}

private fun normalizeFolderChild(
  child: FolderScreen_Query.Child,
): NormalizedFolderChild? {
  val childFolder = child.node.onFolder
  if (childFolder != null) {
    return NormalizedFolderChild(
      id = child.id,
      order = child.order,
      item = EntityListItem.Folder(
        id = child.id,
        iconName = child.icon,
        iconColor = child.iconColor,
        name = childFolder.name,
        folderCount = childFolder.folderCount,
        documentCount = childFolder.documentCount,
      ),
    )
  }

  val document = child.node.onDocument
  if (document != null) {
    return NormalizedFolderChild(
      id = child.id,
      order = child.order,
      item = EntityListItem.Document(
        id = child.id,
        iconName = child.icon,
        iconColor = child.iconColor,
        slug = child.slug,
        title = document.title,
        subtitle = document.subtitle,
        excerpt = document.excerpt,
        updatedAt = document.updatedAt,
      ),
    )
  }

  return null
}
