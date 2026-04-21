package co.typie.domain.entity

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.blob.BlobService
import co.typie.graphql.Apollo
import co.typie.graphql.DocumentShare_PersistBlobAsImage_Mutation
import co.typie.graphql.DocumentShare_UpdateDocumentsOption_Mutation
import co.typie.graphql.EntityShareFolder_PersistBlobAsImage_Mutation
import co.typie.graphql.EntityShareFolder_UpdateFoldersOption_Mutation
import co.typie.graphql.EntityShare_Query
import co.typie.graphql.executeMutation
import co.typie.graphql.fragment.DocumentShare_entity
import co.typie.graphql.fragment.FolderShare_entity
import co.typie.graphql.type.DocumentContentRating
import co.typie.graphql.type.EntityVisibility
import co.typie.graphql.type.PersistBlobAsImageInput
import co.typie.graphql.type.UpdateDocumentsOptionInput
import co.typie.graphql.type.UpdateFoldersOptionInput
import co.typie.graphql.watchQuery
import co.typie.platform.PlatformFile
import co.typie.result.Result
import co.typie.result.result

internal class EntityShareViewModel(
  entityIds: List<String>,
  placeholderData: EntityShare_Query.Data,
) : ViewModel(), FolderShareSheetModel, DocumentShareSheetModel {
  private val blobService = BlobService
  private val resolvedEntityIds = entityIds.map(String::trim).filter(String::isNotEmpty).distinct()

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData,
      skip = { resolvedEntityIds.isEmpty() },
    ) {
      EntityShare_Query(entityIds = resolvedEntityIds)
    }

  fun refetch() {
    if (resolvedEntityIds.isNotEmpty()) {
      query.refetch()
    }
  }

  override suspend fun updateFoldersVisibility(
    folderIds: List<String>,
    visibility: EntityVisibility,
  ): Result<Unit, Nothing> = result {
    if (folderIds.isEmpty()) {
      return@result
    }

    Apollo.executeMutation(
      EntityShareFolder_UpdateFoldersOption_Mutation(
        input = folderOptionsInput(folderIds) { visibility(visibility) }
      )
    )
  }

  override suspend fun uploadFoldersThumbnail(
    folderIds: List<String>,
    file: PlatformFile,
  ): Result<ShareThumbnailResult, Nothing> = result {
    if (folderIds.isEmpty()) {
      return@result ShareThumbnailResult(id = "", url = "")
    }

    val path =
      blobService.uploadBytes(
        bytes = file.bytes,
        filename = file.filename,
        mimeType = file.mimeType,
      )
    val image =
      Apollo.executeMutation(
          EntityShareFolder_PersistBlobAsImage_Mutation(
            input = PersistBlobAsImageInput(path = path)
          )
        )
        .persistBlobAsImage

    Apollo.executeMutation(
      EntityShareFolder_UpdateFoldersOption_Mutation(
        input = folderOptionsInput(folderIds) { thumbnailId(image.id) }
      )
    )

    ShareThumbnailResult(id = image.id, url = image.url)
  }

  override suspend fun removeFoldersThumbnail(folderIds: List<String>): Result<Unit, Nothing> =
    result {
      if (folderIds.isEmpty()) {
        return@result
      }

      Apollo.executeMutation(
        EntityShareFolder_UpdateFoldersOption_Mutation(
          input = folderOptionsInput(folderIds) { thumbnailId(null) }
        )
      )
    }

  override suspend fun applyFoldersVisibilityRecursively(
    folderIds: List<String>,
    visibility: EntityVisibility,
  ): Result<Unit, Nothing> = result {
    if (folderIds.isEmpty()) {
      return@result
    }

    Apollo.executeMutation(
      EntityShareFolder_UpdateFoldersOption_Mutation(
        input =
          folderOptionsInput(folderIds) {
            visibility(visibility)
            recursive(true)
          }
      )
    )
  }

  override suspend fun updateDocumentsVisibility(
    documentIds: List<String>,
    visibility: EntityVisibility,
  ): Result<Unit, Nothing> = result {
    if (documentIds.isEmpty()) {
      return@result
    }

    Apollo.executeMutation(
      DocumentShare_UpdateDocumentsOption_Mutation(
        input = documentOptionsInput(documentIds) { visibility(visibility) }
      )
    )
  }

  override suspend fun updateDocumentsContentRating(
    documentIds: List<String>,
    contentRating: DocumentContentRating,
  ): Result<Unit, Nothing> = result {
    if (documentIds.isEmpty()) {
      return@result
    }

    Apollo.executeMutation(
      DocumentShare_UpdateDocumentsOption_Mutation(
        input = documentOptionsInput(documentIds) { contentRating(contentRating) }
      )
    )
  }

  override suspend fun updateDocumentsPassword(
    documentIds: List<String>,
    password: String?,
  ): Result<Unit, Nothing> = result {
    if (documentIds.isEmpty()) {
      return@result
    }

    Apollo.executeMutation(
      DocumentShare_UpdateDocumentsOption_Mutation(
        input = documentOptionsInput(documentIds) { password(password) }
      )
    )
  }

  override suspend fun updateDocumentsAllowReaction(
    documentIds: List<String>,
    allowReaction: Boolean,
  ): Result<Unit, Nothing> = result {
    if (documentIds.isEmpty()) {
      return@result
    }

    Apollo.executeMutation(
      DocumentShare_UpdateDocumentsOption_Mutation(
        input = documentOptionsInput(documentIds) { allowReaction(allowReaction) }
      )
    )
  }

  override suspend fun updateDocumentsProtectContent(
    documentIds: List<String>,
    protectContent: Boolean,
  ): Result<Unit, Nothing> = result {
    if (documentIds.isEmpty()) {
      return@result
    }

    Apollo.executeMutation(
      DocumentShare_UpdateDocumentsOption_Mutation(
        input = documentOptionsInput(documentIds) { protectContent(protectContent) }
      )
    )
  }

  override suspend fun uploadDocumentsThumbnail(
    documentIds: List<String>,
    file: PlatformFile,
  ): Result<ShareThumbnailResult, Nothing> = result {
    if (documentIds.isEmpty()) {
      return@result ShareThumbnailResult(id = "", url = "")
    }

    val path =
      blobService.uploadBytes(
        bytes = file.bytes,
        filename = file.filename,
        mimeType = file.mimeType,
      )
    val image =
      Apollo.executeMutation(
          DocumentShare_PersistBlobAsImage_Mutation(input = PersistBlobAsImageInput(path = path))
        )
        .persistBlobAsImage

    Apollo.executeMutation(
      DocumentShare_UpdateDocumentsOption_Mutation(
        input = documentOptionsInput(documentIds) { thumbnailId(image.id) }
      )
    )

    ShareThumbnailResult(id = image.id, url = image.url)
  }

  override suspend fun removeDocumentsThumbnail(documentIds: List<String>): Result<Unit, Nothing> =
    result {
      if (documentIds.isEmpty()) {
        return@result
      }

      Apollo.executeMutation(
        DocumentShare_UpdateDocumentsOption_Mutation(
          input = documentOptionsInput(documentIds) { thumbnailId(null) }
        )
      )
    }

  private fun folderOptionsInput(
    folderIds: List<String>,
    block: UpdateFoldersOptionInput.Builder.() -> Unit,
  ): UpdateFoldersOptionInput {
    return UpdateFoldersOptionInput.Builder().folderIds(folderIds).apply(block).build()
  }

  private fun documentOptionsInput(
    documentIds: List<String>,
    block: UpdateDocumentsOptionInput.Builder.() -> Unit,
  ): UpdateDocumentsOptionInput {
    return UpdateDocumentsOptionInput.Builder().documentIds(documentIds).apply(block).build()
  }
}

internal fun folderEntitySharePlaceholderData(entityIds: List<String>): EntityShare_Query.Data {
  val placeholderIds =
    if (entityIds.isEmpty()) {
      listOf("placeholder-entity")
    } else {
      entityIds
    }

  return EntityShare_Query.Data(
    entities =
      placeholderIds.mapIndexed { index, entityId ->
        folderPlaceholderEntity(
          entityId = entityId.ifBlank { "placeholder-folder-entity-$index" },
          index = index,
        )
      }
  )
}

internal fun documentEntitySharePlaceholderData(entityIds: List<String>): EntityShare_Query.Data {
  val placeholderIds =
    if (entityIds.isEmpty()) {
      listOf("placeholder-entity")
    } else {
      entityIds
    }

  return EntityShare_Query.Data(
    entities =
      placeholderIds.mapIndexed { index, entityId ->
        documentPlaceholderEntity(
          entityId = entityId.ifBlank { "placeholder-document-entity-$index" },
          index = index,
        )
      }
  )
}

private fun folderPlaceholderEntity(entityId: String, index: Int): EntityShare_Query.Entity {
  val visibility = EntityVisibility.PRIVATE
  val url = ""
  return EntityShare_Query.Entity(
    __typename = "Entity",
    documentShare_entity =
      DocumentShare_entity(
        __typename = "Entity",
        id = entityId,
        url = url,
        visibility = visibility,
        node = DocumentShare_entity.Node(__typename = "Folder", onDocument = null),
      ),
    folderShare_entity =
      FolderShare_entity(
        __typename = "Entity",
        id = entityId,
        url = url,
        visibility = visibility,
        node =
          FolderShare_entity.Node(
            __typename = "Folder",
            onFolder =
              FolderShare_entity.OnFolder(
                id = entityId,
                name = "가".repeat(6 + (index % 3)),
                thumbnail = null,
              ),
          ),
      ),
  )
}

private fun documentPlaceholderEntity(entityId: String, index: Int): EntityShare_Query.Entity {
  val visibility = EntityVisibility.PRIVATE
  val url = ""
  return EntityShare_Query.Entity(
    __typename = "Entity",
    documentShare_entity =
      DocumentShare_entity(
        __typename = "Entity",
        id = entityId,
        url = url,
        visibility = visibility,
        node =
          DocumentShare_entity.Node(
            __typename = "Document",
            onDocument =
              DocumentShare_entity.OnDocument(
                id = entityId,
                title = "가".repeat(6 + (index % 3)),
                password = null,
                contentRating = DocumentContentRating.ALL,
                allowReaction = true,
                protectContent = true,
                thumbnail = null,
              ),
          ),
      ),
    folderShare_entity =
      FolderShare_entity(
        __typename = "Entity",
        id = entityId,
        url = url,
        visibility = visibility,
        node = FolderShare_entity.Node(__typename = "Document", onFolder = null),
      ),
  )
}
