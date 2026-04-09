package co.typie.screen.space

import co.typie.graphql.SpaceScreen_Query
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.entity_container.OrderedEntityItem

fun normalizeSpaceEntities(
  entities: List<SpaceScreen_Query.Entity>,
): List<OrderedEntityItem> {
  return entities.mapNotNull(::normalizeSpaceEntity)
}

private fun normalizeSpaceEntity(
  entity: SpaceScreen_Query.Entity,
): OrderedEntityItem? {
  val folder = entity.node.onFolder
  if (folder != null) {
    return OrderedEntityItem(
      id = entity.id,
      order = entity.order,
      item = EntityListItem.Folder(
        id = entity.id,
        iconName = entity.icon,
        iconColor = entity.iconColor,
        name = folder.name,
        folderCount = folder.folderCount,
        documentCount = folder.documentCount,
      ),
    )
  }

  val document = entity.node.onDocument
  if (document != null) {
    return OrderedEntityItem(
      id = entity.id,
      order = entity.order,
      item = EntityListItem.Document(
        id = entity.id,
        iconName = entity.icon,
        iconColor = entity.iconColor,
        slug = entity.slug,
        title = document.title,
        subtitle = document.subtitle,
        excerpt = document.excerpt,
        updatedAt = document.updatedAt,
      ),
    )
  }

  return null
}
