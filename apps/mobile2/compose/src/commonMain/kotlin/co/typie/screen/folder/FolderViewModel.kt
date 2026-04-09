package co.typie.screen.folder

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewModelScope
import co.typie.blob.BlobService
import co.touchlab.kermit.Logger
import co.typie.graphql.FolderActions_UpdateEntityIcon_Mutation
import co.typie.graphql.EntityContainer_MoveEntity_Mutation
import co.typie.graphql.FolderShare_PersistBlobAsImage_Mutation
import co.typie.graphql.FolderShare_UpdateFoldersOption_Mutation
import co.typie.graphql.FolderScreen_Query
import co.typie.graphql.FolderActions_RenameFolder_Mutation
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.type.EntityVisibility
import co.typie.graphql.type.MoveEntityInput
import co.typie.graphql.type.PersistBlobAsImageInput
import co.typie.graphql.type.UpdateEntityIconInput
import co.typie.graphql.type.UpdateFoldersOptionInput
import co.typie.graphql.type.RenameFolderInput
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.platform.PlatformFile
import co.typie.service.SiteService
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import org.koin.core.annotation.KoinViewModel

private const val GENERIC_MUTATION_ERROR_MESSAGE = "오류가 발생했어요. 잠시 후 다시 시도해주세요."
private const val RENAME_FOLDER_SUCCESS_MESSAGE = "폴더 이름이 변경되었어요."
private const val THUMBNAIL_UPLOAD_ERROR_MESSAGE = "썸네일 업로드에 실패했어요. 다시 시도해주세요."
private const val THUMBNAIL_REMOVE_SUCCESS_MESSAGE = "썸네일이 삭제되었어요."
private const val THUMBNAIL_REMOVE_ERROR_MESSAGE = "썸네일을 삭제할 수 없어요."
private const val RECURSIVE_VISIBILITY_SUCCESS_MESSAGE = "하위 요소에도 동일한 설정이 적용되었어요."

data class FolderThumbnailResult(
  val id: String,
  val url: String,
)

@KoinViewModel
class FolderViewModel(
  private val siteService: SiteService,
  private val blobService: BlobService,
  private val toast: Toast,
) : GraphQLViewModel() {
  private var hasEnteredScreen = false
  var entityId by mutableStateOf("")

  val siteId: String
    get() = siteService.siteId

  val query = watchQuery(
    skip = { entityId.isBlank() },
  ) {
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

  fun updateFolderVisibility(
    folderId: String,
    visibility: EntityVisibility,
    onFinished: (Boolean) -> Unit = {},
  ) {
    viewModelScope.launch {
      onFinished(
        updateFolderVisibilityInternal(
          folderId = folderId,
          visibility = visibility,
        ),
      )
    }
  }

  private suspend fun updateFolderVisibilityInternal(
    folderId: String,
    visibility: EntityVisibility,
  ): Boolean {
    // TODO: Track folder visibility updates.
    try {
      executeMutation(
        FolderShare_UpdateFoldersOption_Mutation(
          input = folderOptionsInput(folderId) {
            visibility(visibility)
          },
        ),
      )

      refetch()
      return true
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to update folder visibility" }
      toast.show(ToastType.Error, GENERIC_MUTATION_ERROR_MESSAGE)
      return false
    }
  }

  fun uploadFolderThumbnail(
    folderId: String,
    file: PlatformFile,
    onFinished: (FolderThumbnailResult?) -> Unit = {},
  ) {
    viewModelScope.launch {
      onFinished(
        uploadFolderThumbnailInternal(
          folderId = folderId,
          file = file,
        ),
      )
    }
  }

  private suspend fun uploadFolderThumbnailInternal(
    folderId: String,
    file: PlatformFile,
  ): FolderThumbnailResult? {
    // TODO: Track folder thumbnail upload.
    try {
      val path = blobService.uploadBytes(
        bytes = file.bytes,
        filename = file.filename,
        mimeType = file.mimeType,
      )

      val image = executeMutation(
        FolderShare_PersistBlobAsImage_Mutation(
          input = PersistBlobAsImageInput(
            path = path,
          ),
        ),
      ).persistBlobAsImage

      executeMutation(
        FolderShare_UpdateFoldersOption_Mutation(
          input = folderOptionsInput(folderId) {
            thumbnailId(image.id)
          },
        ),
      )

      refetch()

      return FolderThumbnailResult(
        id = image.id,
        url = image.url,
      )
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to upload folder thumbnail" }
      toast.show(ToastType.Error, THUMBNAIL_UPLOAD_ERROR_MESSAGE)
      return null
    }
  }

  fun removeFolderThumbnail(
    folderId: String,
    onFinished: (Boolean) -> Unit = {},
  ) {
    viewModelScope.launch {
      onFinished(
        removeFolderThumbnailInternal(folderId = folderId),
      )
    }
  }

  private suspend fun removeFolderThumbnailInternal(
    folderId: String,
  ): Boolean {
    // TODO: Track folder thumbnail removal.
    try {
      executeMutation(
        FolderShare_UpdateFoldersOption_Mutation(
          input = folderOptionsInput(folderId) {
            thumbnailId(null)
          },
        ),
      )

      refetch()
      toast.show(ToastType.Success, THUMBNAIL_REMOVE_SUCCESS_MESSAGE)
      return true
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to remove folder thumbnail" }
      toast.show(ToastType.Error, THUMBNAIL_REMOVE_ERROR_MESSAGE)
      return false
    }
  }

  fun applyFolderVisibilityRecursively(
    folderId: String,
    visibility: EntityVisibility,
    onFinished: (Boolean) -> Unit = {},
  ) {
    viewModelScope.launch {
      onFinished(
        applyFolderVisibilityRecursivelyInternal(
          folderId = folderId,
          visibility = visibility,
        ),
      )
    }
  }

  private suspend fun applyFolderVisibilityRecursivelyInternal(
    folderId: String,
    visibility: EntityVisibility,
  ): Boolean {
    // TODO: Track recursive folder visibility application.
    try {
      executeMutation(
        FolderShare_UpdateFoldersOption_Mutation(
          input = folderOptionsInput(folderId) {
            visibility(visibility)
            recursive(true)
          },
        ),
      )

      refetch()
      toast.show(ToastType.Success, RECURSIVE_VISIBILITY_SUCCESS_MESSAGE)
      return true
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to apply folder visibility recursively" }
      toast.show(ToastType.Error, GENERIC_MUTATION_ERROR_MESSAGE)
      return false
    }
  }

  fun renameFolder(
    folderId: String,
    currentName: String,
    name: String,
    onFinished: (Boolean) -> Unit = {},
  ) {
    viewModelScope.launch {
      onFinished(
        renameFolderInternal(
          folderId = folderId,
          currentName = currentName,
          name = name,
        ),
      )
    }
  }

  private suspend fun renameFolderInternal(
    folderId: String,
    currentName: String,
    name: String,
  ): Boolean {
    val trimmedName = name.trim()
    val normalizedCurrentName = currentName.trim()
    if (trimmedName.isEmpty()) {
      return false
    }

    if (trimmedName == normalizedCurrentName) {
      return true
    }

    try {
      executeMutation(
        FolderActions_RenameFolder_Mutation(
          input = RenameFolderInput(
            folderId = folderId,
            name = trimmedName,
          ),
        ),
      )

      refetch()
      toast.show(ToastType.Success, RENAME_FOLDER_SUCCESS_MESSAGE)
      return true
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to rename folder" }
      toast.show(ToastType.Error, GENERIC_MUTATION_ERROR_MESSAGE)
      return false
    }
  }

  fun updateEntityIcon(
    entityId: String,
    icon: String,
    iconColor: String,
    onFinished: (Boolean) -> Unit = {},
  ) {
    viewModelScope.launch {
      onFinished(
        updateEntityIconInternal(
          entityId = entityId,
          icon = icon,
          iconColor = iconColor,
        ),
      )
    }
  }

  private suspend fun updateEntityIconInternal(
    entityId: String,
    icon: String,
    iconColor: String,
  ): Boolean {
    val normalizedIcon = icon.trim()
    val normalizedColor = iconColor.trim()

    try {
      executeMutation(
        FolderActions_UpdateEntityIcon_Mutation(
          input = UpdateEntityIconInput(
            entityId = entityId,
            icon = normalizedIcon,
            iconColor = normalizedColor,
          ),
        ),
      )

      refetch()
      return true
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to update entity icon" }
      toast.show(ToastType.Error, GENERIC_MUTATION_ERROR_MESSAGE)
      return false
    }
  }

  fun moveChildEntity(
    entityId: String,
    parentEntityId: String,
    lowerOrder: String?,
    upperOrder: String?,
    onFinished: (Boolean) -> Unit = {},
  ) {
    viewModelScope.launch {
      onFinished(
        moveChildEntityInternal(
          entityId = entityId,
          parentEntityId = parentEntityId,
          lowerOrder = lowerOrder,
          upperOrder = upperOrder,
        ),
      )
    }
  }

  private suspend fun moveChildEntityInternal(
    entityId: String,
    parentEntityId: String,
    lowerOrder: String?,
    upperOrder: String?,
  ): Boolean {
    try {
      executeMutation(
        EntityContainer_MoveEntity_Mutation(
          input = MoveEntityInput.Builder()
            .entityId(entityId)
            .parentEntityId(parentEntityId)
            .apply {
              if (lowerOrder != null) {
                lowerOrder(lowerOrder)
              }
              if (upperOrder != null) {
                upperOrder(upperOrder)
              }
            }
            .build(),
        ),
      )

      refetch()
      return true
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to move folder child entity" }
      toast.show(ToastType.Error, GENERIC_MUTATION_ERROR_MESSAGE)
      return false
    }
  }

  private fun folderOptionsInput(
    folderId: String,
    block: UpdateFoldersOptionInput.Builder.() -> Unit,
  ): UpdateFoldersOptionInput {
    return UpdateFoldersOptionInput.Builder()
      .folderIds(listOf(folderId))
      .apply(block)
      .build()
  }
}
