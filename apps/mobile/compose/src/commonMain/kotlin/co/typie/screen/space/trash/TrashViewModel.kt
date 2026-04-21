package co.typie.screen.space.trash

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.entity.isFolder
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TrashScreen_Folder_Query
import co.typie.graphql.TrashScreen_PurgeEntities_Mutation
import co.typie.graphql.TrashScreen_RecoverEntity_Mutation
import co.typie.graphql.TrashScreen_Root_Query
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildFolder
import co.typie.graphql.builder.buildSite
import co.typie.graphql.executeMutation
import co.typie.graphql.fragment.EntityRow_entity
import co.typie.graphql.text
import co.typie.graphql.type.EntityState
import co.typie.graphql.type.EntityType
import co.typie.graphql.type.PurgeEntitiesInput
import co.typie.graphql.type.RecoverEntityInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import co.typie.storage.Preference

internal class TrashViewModel : ViewModel() {
  var entityId: String? by mutableStateOf(null)

  val siteQuery =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = trashRootPlaceholderData(),
      skip = { entityId != null || Preference.siteId == null },
    ) {
      TrashScreen_Root_Query(siteId = Preference.siteId!!)
    }

  val entityQuery =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = trashFolderPlaceholderData(),
      skip = { entityId == null },
    ) {
      TrashScreen_Folder_Query(entityId = requireNotNull(entityId))
    }

  fun refetch() {
    if (entityId == null) {
      siteQuery.refetch()
    } else {
      entityQuery.refetch()
    }
  }

  suspend fun recoverEntity(item: EntityRow_entity): Result<String, Nothing> {
    return result {
      val data =
        Apollo.executeMutation(
          TrashScreen_RecoverEntity_Mutation(input = RecoverEntityInput(entityId = item.id))
        )
      "\"${trashRecoveryPath(data.recoverEntity)}\" ${if (item.isFolder()) "폴더" else "문서"}를 복원했어요"
    }
  }

  suspend fun purgeEntities(entityIds: List<String>): Result<Unit, Nothing> {
    return result {
      Apollo.executeMutation(
        TrashScreen_PurgeEntities_Mutation(input = PurgeEntitiesInput(entityIds = entityIds))
      )
    }
  }
}

private fun trashRecoveryPath(entity: TrashScreen_RecoverEntity_Mutation.RecoverEntity): String {
  val segments = buildList {
    addAll(entity.ancestors.mapNotNull { it.node.onFolder?.name })
    add(entity.node.onFolder?.name ?: entity.node.onDocument?.title ?: "삭제된 항목")
  }

  return segments.joinToString(" › ")
}

internal fun trashRootPlaceholderData() =
  TrashScreen_Root_Query.Data(PlaceholderResolver) {
    site = buildSite {
      id = "placeholder-site"
      name = text(4..8)
      deletedEntities =
        List(5) { index ->
          buildEntity {
            val isFolder = index % 3 == 0

            id = "placeholder-trash-item-$index"
            depth = 1
            order = index.toString()
            slug = "placeholder-trash-item-$index"
            url = ""
            type = if (isFolder) EntityType.FOLDER else EntityType.DOCUMENT
            icon = if (isFolder) "folder" else "file"
            iconColor = "gray"
            node =
              if (isFolder) {
                buildFolder {
                  id = "placeholder-trash-folder-node-$index"
                  name = text(5..10)
                  maxDescendantFoldersDepth = 0
                  folderCount = 0
                  documentCount = 0
                }
              } else {
                buildDocument {
                  id = "placeholder-trash-document-node-$index"
                  title = text(5..14)
                  subtitle = if (index % 2 == 0) text(4..8) else null
                  excerpt = text(18..28)
                }
              }
          }
        }
    }
  }

internal fun trashFolderPlaceholderData() =
  TrashScreen_Folder_Query.Data(PlaceholderResolver) {
    entity = buildEntity {
      id = "placeholder-trash-folder"
      type = EntityType.FOLDER
      state = EntityState.DELETED
      depth = 0
      order = "0"
      slug = "placeholder-trash-folder"
      url = ""
      icon = "folder"
      iconColor = "gray"
      site = buildSite {
        id = "placeholder-site"
        name = text(4..8)
      }
      node = buildFolder {
        id = "placeholder-trash-folder-node"
        name = text(5..10)
        maxDescendantFoldersDepth = 0
        folderCount = 0
        documentCount = 0
      }
      deletedChildren =
        List(5) { index ->
          buildEntity {
            val isFolder = index % 3 == 0

            id = "placeholder-trash-item-$index"
            depth = 1
            order = index.toString()
            slug = "placeholder-trash-item-$index"
            url = ""
            type = if (isFolder) EntityType.FOLDER else EntityType.DOCUMENT
            icon = if (isFolder) "folder" else "file"
            iconColor = "gray"
            node =
              if (isFolder) {
                buildFolder {
                  id = "placeholder-trash-folder-node-$index"
                  name = text(5..10)
                  maxDescendantFoldersDepth = 0
                  folderCount = 0
                  documentCount = 0
                }
              } else {
                buildDocument {
                  id = "placeholder-trash-document-node-$index"
                  title = text(5..14)
                  subtitle = if (index % 2 == 0) text(4..8) else null
                  excerpt = text(18..28)
                }
              }
          }
        }
    }
  }
