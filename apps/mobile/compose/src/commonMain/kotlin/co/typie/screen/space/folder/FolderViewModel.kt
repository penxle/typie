package co.typie.screen.space.folder

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.blob.BlobService
import co.typie.domain.entity.DocumentRenameSheetModel
import co.typie.domain.entity.EntityIconPickerSheetModel
import co.typie.domain.entity.FolderRenameSheetModel
import co.typie.graphql.Apollo
import co.typie.graphql.DocumentActions_DeleteDocument_Mutation
import co.typie.graphql.DocumentActions_UpdateDocument_Mutation
import co.typie.graphql.EntityContainer_MoveEntity_Mutation
import co.typie.graphql.FolderActions_DeleteEntities_Mutation
import co.typie.graphql.FolderActions_RenameFolder_Mutation
import co.typie.graphql.FolderActions_UpdateEntityIcon_Mutation
import co.typie.graphql.FolderScreen_Query
import co.typie.graphql.FolderShare_PersistBlobAsImage_Mutation
import co.typie.graphql.FolderShare_UpdateFoldersOption_Mutation
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildFolder
import co.typie.graphql.builder.buildSite
import co.typie.graphql.executeMutation
import co.typie.graphql.midpointOrder
import co.typie.graphql.text
import co.typie.graphql.type.DeleteDocumentInput
import co.typie.graphql.type.DeleteEntitiesInput
import co.typie.graphql.type.EntityAvailability
import co.typie.graphql.type.EntityState
import co.typie.graphql.type.EntityType
import co.typie.graphql.type.EntityVisibility
import co.typie.graphql.type.MoveEntityInput
import co.typie.graphql.type.PersistBlobAsImageInput
import co.typie.graphql.type.RenameFolderInput
import co.typie.graphql.type.UpdateDocumentInput
import co.typie.graphql.type.UpdateEntityIconInput
import co.typie.graphql.type.UpdateFoldersOptionInput
import co.typie.graphql.watchQuery
import co.typie.platform.PlatformFile
import co.typie.result.Result
import co.typie.result.result

data class FolderThumbnailResult(val id: String, val url: String)

class FolderViewModel :
  ViewModel(), DocumentRenameSheetModel, EntityIconPickerSheetModel, FolderRenameSheetModel {
  private val blobService = BlobService
  var entityId by mutableStateOf("")

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      skip = { entityId.isBlank() },
    ) {
      FolderScreen_Query(entityId = entityId)
    }

  fun refetch() {
    if (entityId.isNotBlank()) {
      query.refetch()
    }
  }

  suspend fun updateFolderVisibility(
    folderId: String,
    visibility: EntityVisibility,
  ): Result<Unit, Nothing> =
    updateFoldersVisibility(folderIds = listOf(folderId), visibility = visibility)

  suspend fun updateFoldersVisibility(
    folderIds: List<String>,
    visibility: EntityVisibility,
  ): Result<Unit, Nothing> = result {
    if (folderIds.isEmpty()) {
      return@result
    }
    Apollo.executeMutation(
      FolderShare_UpdateFoldersOption_Mutation(
        input = folderOptionsInput(folderIds) { visibility(visibility) }
      )
    )
  }

  suspend fun uploadFolderThumbnail(
    folderId: String,
    file: PlatformFile,
  ): Result<FolderThumbnailResult, Nothing> =
    uploadFoldersThumbnail(folderIds = listOf(folderId), file = file)

  suspend fun uploadFoldersThumbnail(
    folderIds: List<String>,
    file: PlatformFile,
  ): Result<FolderThumbnailResult, Nothing> = result {
    if (folderIds.isEmpty()) {
      return@result FolderThumbnailResult(id = "", url = "")
    }
    val path =
      blobService.uploadBytes(
        bytes = file.bytes,
        filename = file.filename,
        mimeType = file.mimeType,
      )

    val image =
      Apollo.executeMutation(
          FolderShare_PersistBlobAsImage_Mutation(input = PersistBlobAsImageInput(path = path))
        )
        .persistBlobAsImage

    Apollo.executeMutation(
      FolderShare_UpdateFoldersOption_Mutation(
        input = folderOptionsInput(folderIds) { thumbnailId(image.id) }
      )
    )

    FolderThumbnailResult(id = image.id, url = image.url)
  }

  suspend fun removeFolderThumbnail(folderId: String): Result<Unit, Nothing> =
    removeFoldersThumbnail(folderIds = listOf(folderId))

  suspend fun removeFoldersThumbnail(folderIds: List<String>): Result<Unit, Nothing> = result {
    if (folderIds.isEmpty()) {
      return@result
    }
    Apollo.executeMutation(
      FolderShare_UpdateFoldersOption_Mutation(
        input = folderOptionsInput(folderIds) { thumbnailId(null) }
      )
    )
  }

  suspend fun applyFolderVisibilityRecursively(
    folderId: String,
    visibility: EntityVisibility,
  ): Result<Unit, Nothing> =
    applyFoldersVisibilityRecursively(folderIds = listOf(folderId), visibility = visibility)

  suspend fun applyFoldersVisibilityRecursively(
    folderIds: List<String>,
    visibility: EntityVisibility,
  ): Result<Unit, Nothing> = result {
    if (folderIds.isEmpty()) {
      return@result
    }
    Apollo.executeMutation(
      FolderShare_UpdateFoldersOption_Mutation(
        input =
          folderOptionsInput(folderIds) {
            visibility(visibility)
            recursive(true)
          }
      )
    )
  }

  override suspend fun renameFolder(
    folderId: String,
    currentName: String,
    name: String,
  ): Result<Unit, Nothing> = result {
    val trimmedName = name.trim()
    val normalizedCurrentName = currentName.trim()
    if (trimmedName.isEmpty() || trimmedName == normalizedCurrentName) return@result
    Apollo.executeMutation(
      FolderActions_RenameFolder_Mutation(
        input = RenameFolderInput(folderId = folderId, name = trimmedName)
      )
    )
  }

  override suspend fun updateDocument(
    documentId: String,
    currentTitle: String,
    title: String,
  ): Result<Unit, Nothing> = result {
    val trimmedTitle = title.trim()
    val normalizedCurrentTitle = currentTitle.trim()
    if (trimmedTitle.isEmpty() || trimmedTitle == normalizedCurrentTitle) return@result
    Apollo.executeMutation(
      DocumentActions_UpdateDocument_Mutation(
        input = UpdateDocumentInput.Builder().documentId(documentId).title(trimmedTitle).build()
      )
    )
  }

  override suspend fun updateEntityIcons(
    entityIds: List<String>,
    icon: String?,
    iconColor: String?,
  ): Result<Unit, Nothing> = result {
    val entityId = entityIds.singleOrNull() ?: return@result
    val resolvedIcon = icon?.trim()?.takeIf { it.isNotEmpty() } ?: return@result
    val resolvedIconColor = iconColor?.trim()?.takeIf { it.isNotEmpty() } ?: return@result
    Apollo.executeMutation(
      FolderActions_UpdateEntityIcon_Mutation(
        input =
          UpdateEntityIconInput(
            entityId = entityId,
            icon = resolvedIcon,
            iconColor = resolvedIconColor,
          )
      )
    )
  }

  suspend fun deleteDocument(documentId: String): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      DocumentActions_DeleteDocument_Mutation(input = DeleteDocumentInput(documentId = documentId))
    )
  }

  suspend fun deleteFolderEntity(entityId: String): Result<Unit, Nothing> =
    deleteEntities(listOf(entityId))

  suspend fun deleteEntities(entityIds: List<String>): Result<Unit, Nothing> = result {
    if (entityIds.isEmpty()) {
      return@result
    }
    Apollo.executeMutation(
      FolderActions_DeleteEntities_Mutation(input = DeleteEntitiesInput(entityIds = entityIds))
    )
  }

  suspend fun moveChildEntity(
    entityId: String,
    parentEntityId: String,
    lowerOrder: String?,
    upperOrder: String?,
  ): Result<Unit, Nothing> = result {
    val newOrder = midpointOrder(lowerOrder, upperOrder)
    Apollo.executeMutation(
      EntityContainer_MoveEntity_Mutation(
        input =
          MoveEntityInput.Builder()
            .entityId(entityId)
            .parentEntityId(parentEntityId)
            .apply {
              if (lowerOrder != null) lowerOrder(lowerOrder)
              if (upperOrder != null) upperOrder(upperOrder)
            }
            .build()
      ),
      optimisticUpdate =
        EntityContainer_MoveEntity_Mutation.Data(PlaceholderResolver) {
          moveEntity = buildEntity {
            id = entityId
            order = newOrder
          }
        },
    )
  }

  private fun folderOptionsInput(
    folderIds: List<String>,
    block: UpdateFoldersOptionInput.Builder.() -> Unit,
  ): UpdateFoldersOptionInput =
    UpdateFoldersOptionInput.Builder().folderIds(folderIds).apply(block).build()
}

private fun placeholderData() =
  FolderScreen_Query.Data(PlaceholderResolver) {
    entity = buildEntity {
      id = "placeholder-folder"
      type = EntityType.FOLDER
      state = EntityState.ACTIVE
      depth = 0
      order = "0"
      slug = "placeholder-folder"
      url = ""
      icon = "folder"
      iconColor = "gray"
      visibility = EntityVisibility.PRIVATE
      availability = EntityAvailability.PRIVATE
      ancestors = emptyList()
      children = emptyList()
      site = buildSite {
        id = "placeholder-site"
        name = text(4..8)
      }
      node = buildFolder {
        id = "placeholder-folder-node"
        name = text(5..10)
        maxDescendantFoldersDepth = 0
        folderCount = 0
        documentCount = 0
        characterCount = 0
      }
    }
  }
