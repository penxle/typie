package co.typie.screen.space.space

import co.typie.graphql.SpaceScreen_Query
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.entitycontainer.OrderedEntityItem

fun normalizeSpaceEntities(
  siteName: String,
  entities: List<SpaceScreen_Query.Entity>,
): List<OrderedEntityItem> {
  return entities.mapNotNull { entity ->
    normalizeSpaceEntity(siteName = siteName, entity = entity)
  }
}

private fun normalizeSpaceEntity(
  siteName: String,
  entity: SpaceScreen_Query.Entity,
): OrderedEntityItem? {
  val folder = entity.node.onFolder
  if (folder != null) {
    return OrderedEntityItem(
      id = entity.id,
      order = entity.order,
      item =
        EntityListItem.Folder(
          id = entity.id,
          folderId = folder.id,
          iconName = entity.icon,
          iconColor = entity.iconColor,
          name = folder.name,
          folderCount = folder.folderCount,
          documentCount = folder.documentCount,
          siteName = siteName,
          ancestorFolderNames = entity.ancestors.mapNotNull { it.node.onFolder?.name },
          depth = entity.depth,
          url = entity.url,
          visibility = entity.visibility,
          availability = entity.availability,
          characterCount = folder.characterCount,
          maxDescendantFoldersDepth = folder.maxDescendantFoldersDepth,
          thumbnailUrl = folder.thumbnail?.url,
        ),
    )
  }

  val document = entity.node.onDocument
  if (document != null) {
    return OrderedEntityItem(
      id = entity.id,
      order = entity.order,
      item =
        EntityListItem.Document(
          id = entity.id,
          documentId = document.id,
          iconName = entity.icon,
          iconColor = entity.iconColor,
          slug = entity.slug,
          title = document.title,
          subtitle = document.subtitle,
          excerpt = document.excerpt,
          updatedAt = document.updatedAt,
          siteName = siteName,
          ancestorFolderNames = entity.ancestors.mapNotNull { it.node.onFolder?.name },
          depth = entity.depth,
          url = entity.url,
          visibility = entity.visibility,
          availability = entity.availability,
          characterCount = document.characterCount,
        ),
    )
  }

  return null
}
