package co.typie.domain.entity

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.graphql.Apollo
import co.typie.graphql.EntityItemActions_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.WatchQuery
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildFolder
import co.typie.graphql.builder.buildSite
import co.typie.graphql.fragment.EntityRow_entity
import co.typie.graphql.text
import co.typie.graphql.type.EntityAvailability
import co.typie.graphql.type.EntityVisibility
import co.typie.graphql.watchQuery
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.theme.AppTheme

private val EntityItemActionsStatusPadding = 24.dp

private class EntityItemActionsViewModel(initialEntity: EntityRow_entity) : ViewModel() {
  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = entityItemActionsPlaceholderData(initialEntity),
      skip = { initialEntity.id.isBlank() },
    ) {
      EntityItemActions_Query(entityId = initialEntity.id)
    }
}

@Composable
internal fun rememberEntityItemActionsQuery(
  initialEntity: EntityRow_entity
): WatchQuery<EntityItemActions_Query.Data, EntityItemActions_Query.Data> {
  val model =
    viewModel(key = "entity-item-actions:${initialEntity.id}") {
      EntityItemActionsViewModel(initialEntity)
    }

  return model.query
}

@Composable
context(_: SheetScope<Unit>)
internal fun EntityItemActionsStatusContent(message: String) {
  SheetLayout(padding = SheetPadding.None) {
    Text(
      text = message,
      modifier = Modifier.fillMaxWidth().padding(horizontal = EntityItemActionsStatusPadding),
      style = AppTheme.typography.body,
      color = AppTheme.colors.textSecondary,
    )
  }
}

private fun entityItemActionsPlaceholderData(initialEntity: EntityRow_entity) =
  EntityItemActions_Query.Data(PlaceholderResolver) {
    entity = buildEntity {
      id = initialEntity.id
      depth = initialEntity.depth
      order = initialEntity.order
      slug = initialEntity.slug
      url = initialEntity.url
      type = initialEntity.entityIcon_entity.type
      icon = initialEntity.entityIcon_entity.icon
      iconColor = initialEntity.entityIcon_entity.iconColor
      visibility = EntityVisibility.PRIVATE
      availability = EntityAvailability.PRIVATE
      ancestors = emptyList()
      site = buildSite {
        id = "placeholder-site"
        name = text(4..8)
      }
      node =
        initialEntity.folder?.let { folder ->
          buildFolder {
            id = folder.id
            name = folder.name
            maxDescendantFoldersDepth = folder.maxDescendantFoldersDepth
            folderCount = folder.folderCount
            documentCount = folder.documentCount
            characterCount = 0
          }
        }
          ?: initialEntity.document?.let { document ->
            buildDocument {
              id = document.id
              title = document.title
              subtitle = document.subtitle
              excerpt = document.excerpt
              updatedAt = document.updatedAt
              characterCount = 0
            }
          }
          ?: error("Unsupported entity type for item actions placeholder")
    }
  }
