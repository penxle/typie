package co.typie.domain.entity

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.entitytransfer.EntityTransferSource
import co.typie.graphql.Apollo
import co.typie.graphql.EntityContainer_MoveEntity_Mutation
import co.typie.graphql.EntityMoveSheet_Folder_Query
import co.typie.graphql.EntityMoveSheet_Root_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildFolder
import co.typie.graphql.builder.buildSite
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.type.MoveEntityInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import co.typie.storage.Preference

class EntityMoveSheetViewModel(initialDestinationEntityId: String?) : ViewModel() {
  val destinationEntityId: String? = initialDestinationEntityId

  val rootQuery =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = moveRootPlaceholderData(),
      skip = { destinationEntityId != null || Preference.siteId == null },
    ) {
      EntityMoveSheet_Root_Query(siteId = Preference.siteId!!)
    }

  val entityQuery =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = moveFolderPlaceholderData(),
      skip = { destinationEntityId == null },
    ) {
      EntityMoveSheet_Folder_Query(entityId = requireNotNull(destinationEntityId))
    }

  fun refetch() {
    if (destinationEntityId == null) {
      rootQuery.refetch()
    } else {
      entityQuery.refetch()
    }
  }

  suspend fun moveEntity(
    source: EntityTransferSource,
    parentEntityId: String?,
    lowerOrder: String?,
    upperOrder: String?,
  ): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      EntityContainer_MoveEntity_Mutation(
        input =
          MoveEntityInput.Builder()
            .entityId(source.id)
            .parentEntityId(parentEntityId)
            .apply {
              if (parentEntityId == null) treatEmptyParentIdAsRoot(true)
              if (lowerOrder != null) lowerOrder(lowerOrder)
              if (upperOrder != null) upperOrder(upperOrder)
            }
            .build()
      )
    )
  }
}

private fun moveRootPlaceholderData() =
  EntityMoveSheet_Root_Query.Data(PlaceholderResolver) {
    site = buildSite {
      id = "placeholder-site"
      name = text(4..8)
      entities =
        List(5) { index ->
          buildEntity {
            id = "placeholder-root-folder-$index"
            depth = 0
            order = index.toString()
            slug = "placeholder-root-folder-$index"
            url = ""
            icon = "folder"
            iconColor = "gray"
            node = buildFolder {
              id = "placeholder-root-folder-node-$index"
              name = text(5..10)
              maxDescendantFoldersDepth = 0
              folderCount = 0
              documentCount = 0
            }
          }
        }
    }
  }

private fun moveFolderPlaceholderData() =
  EntityMoveSheet_Folder_Query.Data(PlaceholderResolver) {
    entity = buildEntity {
      id = "placeholder-folder"
      depth = 2
      site = buildSite {
        id = "placeholder-site"
        name = text(4..8)
      }
      ancestors =
        List(2) { index ->
          buildEntity {
            id = "placeholder-ancestor-$index"
            node = buildFolder {
              id = "placeholder-ancestor-node-$index"
              name = text(5..10)
            }
          }
        }
      node = buildFolder {
        id = "placeholder-folder-node"
        name = text(5..10)
      }
      children =
        List(5) { index ->
          buildEntity {
            id = "placeholder-child-folder-$index"
            depth = 3
            order = index.toString()
            slug = "placeholder-child-folder-$index"
            url = ""
            icon = "folder"
            iconColor = "gray"
            node = buildFolder {
              id = "placeholder-child-folder-node-$index"
              name = text(5..10)
              maxDescendantFoldersDepth = 0
              folderCount = 0
              documentCount = 0
            }
          }
        }
    }
  }
