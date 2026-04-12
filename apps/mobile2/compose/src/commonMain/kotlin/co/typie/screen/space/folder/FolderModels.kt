package co.typie.screen.space.folder

import co.typie.graphql.FolderScreen_Query
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.entity_container.OrderedEntityItem

fun normalizeFolderChildren(
  siteName: String,
  children: List<FolderScreen_Query.Child>,
): List<OrderedEntityItem> {
  return children.mapNotNull { child -> normalizeFolderChild(siteName = siteName, child = child) }
}

private fun normalizeFolderChild(
  siteName: String,
  child: FolderScreen_Query.Child,
): OrderedEntityItem? {
  val childFolder = child.node.onFolder
  if (childFolder != null) {
    return OrderedEntityItem(
      id = child.id,
      order = child.order,
      item =
        EntityListItem.Folder(
          id = child.id,
          folderId = childFolder.id,
          iconName = child.icon,
          iconColor = child.iconColor,
          name = childFolder.name,
          folderCount = childFolder.folderCount,
          documentCount = childFolder.documentCount,
          siteName = siteName,
          ancestorFolderNames = child.ancestors.mapNotNull { it.node.onFolder?.name },
          depth = child.depth,
          url = child.url,
          visibility = child.visibility,
          availability = child.availability,
          characterCount = childFolder.characterCount,
          maxDescendantFoldersDepth = childFolder.maxDescendantFoldersDepth,
          thumbnailUrl = childFolder.thumbnail?.url,
        ),
    )
  }

  val document = child.node.onDocument
  if (document != null) {
    return OrderedEntityItem(
      id = child.id,
      order = child.order,
      item =
        EntityListItem.Document(
          id = child.id,
          documentId = document.id,
          iconName = child.icon,
          iconColor = child.iconColor,
          slug = child.slug,
          title = document.title,
          subtitle = document.subtitle,
          excerpt = document.excerpt,
          updatedAt = document.updatedAt,
          siteName = siteName,
          ancestorFolderNames = child.ancestors.mapNotNull { it.node.onFolder?.name },
          depth = child.depth,
          url = child.url,
          visibility = child.visibility,
          availability = child.availability,
          characterCount = document.characterCount,
        ),
    )
  }

  return null
}
