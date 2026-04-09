package co.typie.screen.folder

import co.typie.graphql.FolderScreen_Query
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.entity_container.OrderedEntityItem

fun normalizeFolderChildren(
  children: List<FolderScreen_Query.Child>,
): List<OrderedEntityItem> {
  return children.mapNotNull(::normalizeFolderChild)
}

private fun normalizeFolderChild(
  child: FolderScreen_Query.Child,
): OrderedEntityItem? {
  val childFolder = child.node.onFolder
  if (childFolder != null) {
    return OrderedEntityItem(
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
    return OrderedEntityItem(
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
