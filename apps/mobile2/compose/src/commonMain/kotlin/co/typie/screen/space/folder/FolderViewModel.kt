package co.typie.screen.space.folder

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.blob.BlobService
import co.typie.graphql.Apollo
import co.typie.graphql.EntityContainer_MoveEntity_Mutation
import co.typie.graphql.FolderActions_DeleteEntities_Mutation
import co.typie.graphql.FolderActions_RenameFolder_Mutation
import co.typie.graphql.FolderActions_UpdateEntityIcon_Mutation
import co.typie.graphql.FolderScreen_Query
import co.typie.graphql.FolderShare_PersistBlobAsImage_Mutation
import co.typie.graphql.FolderShare_UpdateFoldersOption_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.DeleteEntitiesInput
import co.typie.graphql.type.EntityVisibility
import co.typie.graphql.type.MoveEntityInput
import co.typie.graphql.type.PersistBlobAsImageInput
import co.typie.graphql.type.RenameFolderInput
import co.typie.graphql.type.UpdateEntityIconInput
import co.typie.graphql.type.UpdateFoldersOptionInput
import co.typie.graphql.watchQuery
import co.typie.platform.PlatformFile
import co.typie.result.Result
import co.typie.result.result
import co.typie.service.SiteService

data class FolderThumbnailResult(val id: String, val url: String)

class FolderViewModel : ViewModel() {
  private val blobService = BlobService
  private var hasEnteredScreen = false
  var entityId by mutableStateOf("")

  val siteId: String
    get() = SiteService.siteId

  val query =
    Apollo.watchQuery(scope = viewModelScope, skip = { entityId.isBlank() }) {
      FolderScreen_Query(entityId = entityId)
    }

  fun refetch() {
    if (entityId.isNotBlank()) {
      query.refetch()
    }
  }

  fun onScreenEntered() {
    if (hasEnteredScreen) {
      refetch()
      return
    }
    hasEnteredScreen = true
  }

  suspend fun updateFolderVisibility(
    folderId: String,
    visibility: EntityVisibility,
  ): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      FolderShare_UpdateFoldersOption_Mutation(
        input = folderOptionsInput(folderId) { visibility(visibility) }
      )
    )
    refetch()
  }

  suspend fun uploadFolderThumbnail(
    folderId: String,
    file: PlatformFile,
  ): Result<FolderThumbnailResult, Nothing> = result {
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
        input = folderOptionsInput(folderId) { thumbnailId(image.id) }
      )
    )

    refetch()

    FolderThumbnailResult(id = image.id, url = image.url)
  }

  suspend fun removeFolderThumbnail(folderId: String): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      FolderShare_UpdateFoldersOption_Mutation(
        input = folderOptionsInput(folderId) { thumbnailId(null) }
      )
    )
    refetch()
  }

  suspend fun applyFolderVisibilityRecursively(
    folderId: String,
    visibility: EntityVisibility,
  ): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      FolderShare_UpdateFoldersOption_Mutation(
        input =
          folderOptionsInput(folderId) {
            visibility(visibility)
            recursive(true)
          }
      )
    )
    refetch()
  }

  suspend fun renameFolder(
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
    refetch()
  }

  suspend fun updateEntityIcon(
    entityId: String,
    icon: String,
    iconColor: String,
  ): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      FolderActions_UpdateEntityIcon_Mutation(
        input =
          UpdateEntityIconInput(
            entityId = entityId,
            icon = icon.trim(),
            iconColor = iconColor.trim(),
          )
      )
    )
    refetch()
  }

  suspend fun deleteFolderEntity(entityId: String): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      FolderActions_DeleteEntities_Mutation(
        input = DeleteEntitiesInput(entityIds = listOf(entityId))
      )
    )
  }

  suspend fun moveChildEntity(
    entityId: String,
    parentEntityId: String,
    lowerOrder: String?,
    upperOrder: String?,
  ): Result<Unit, Nothing> = result {
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
      )
    )
    refetch()
  }

  private fun folderOptionsInput(
    folderId: String,
    block: UpdateFoldersOptionInput.Builder.() -> Unit,
  ): UpdateFoldersOptionInput =
    UpdateFoldersOptionInput.Builder().folderIds(listOf(folderId)).apply(block).build()
}
