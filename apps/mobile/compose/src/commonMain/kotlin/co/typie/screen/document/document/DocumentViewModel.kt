package co.typie.screen.document.document

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.entity.EntityIconPickerSheetModel
import co.typie.editor.ffi.CharacterCounts
import co.typie.graphql.Apollo
import co.typie.graphql.DocumentActions_DeleteDocument_Mutation
import co.typie.graphql.DocumentActions_DuplicateDocument_Mutation
import co.typie.graphql.DocumentActions_UpdateDocumentType_Mutation
import co.typie.graphql.DocumentActions_UpdateEntityIcon_Mutation
import co.typie.graphql.DocumentExportSheet_ExportDocument_Mutation
import co.typie.graphql.DocumentScreen_Query
import co.typie.graphql.DocumentScreen_UpdateDocumentLock_Mutation
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildCharacterCountChange
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildSite
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.type.DeleteDocumentInput
import co.typie.graphql.type.DocumentExportFormat
import co.typie.graphql.type.DocumentType
import co.typie.graphql.type.DuplicateDocumentInput
import co.typie.graphql.type.EntityAvailability
import co.typie.graphql.type.EntityType
import co.typie.graphql.type.EntityVisibility
import co.typie.graphql.type.ExportDocumentInput
import co.typie.graphql.type.ExportDocumentPageLayoutInput
import co.typie.graphql.type.UpdateDocumentInput
import co.typie.graphql.type.UpdateDocumentTypeInput
import co.typie.graphql.type.UpdateEntityIconInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import kotlin.time.Clock

class DocumentViewModel : ViewModel(), EntityIconPickerSheetModel {
  var entityId by mutableStateOf("")
  var characterCounts by mutableStateOf<CharacterCounts?>(null)
    private set

  private var characterCountsEntityId: String? = null

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      skip = { entityId.isBlank() },
    ) {
      DocumentScreen_Query(entityId = entityId)
    }

  fun refetch() {
    if (entityId.isNotBlank()) {
      query.refetch()
    }
  }

  fun setInitialCharacterCounts(entityId: String, counts: CharacterCounts?) {
    if (characterCountsEntityId != entityId || counts != null) {
      characterCountsEntityId = entityId
      characterCounts = counts
    }
  }

  override suspend fun updateEntityIcons(
    entityIds: List<String>,
    icon: String?,
    iconColor: String?,
  ): Result<Unit, Nothing> = result {
    val resolvedEntityId = entityIds.singleOrNull() ?: return@result
    val resolvedIcon = icon?.trim()?.takeIf(String::isNotEmpty) ?: return@result
    val resolvedColor = iconColor?.trim()?.takeIf(String::isNotEmpty) ?: return@result

    Apollo.executeMutation(
      DocumentActions_UpdateEntityIcon_Mutation(
        input =
          UpdateEntityIconInput(
            entityId = resolvedEntityId,
            icon = resolvedIcon,
            iconColor = resolvedColor,
          )
      )
    )
  }

  suspend fun updateDocumentType(documentId: String, type: DocumentType): Result<Unit, Nothing> =
    result {
      Apollo.executeMutation(
        DocumentActions_UpdateDocumentType_Mutation(
          input = UpdateDocumentTypeInput(documentId = documentId, type = type)
        )
      )
    }

  suspend fun updateDocumentLock(documentId: String, locked: Boolean): Result<Boolean, Nothing> =
    result {
      Apollo.executeMutation(
          DocumentScreen_UpdateDocumentLock_Mutation(
            input = UpdateDocumentInput.Builder().documentId(documentId).locked(locked).build()
          )
        )
        .updateDocument
        .locked
    }

  suspend fun duplicateDocument(documentId: String): Result<String, Nothing> = result {
    val response =
      Apollo.executeMutation(
        DocumentActions_DuplicateDocument_Mutation(
          input = DuplicateDocumentInput(documentId = documentId)
        )
      )

    response.duplicateDocument.entity.id
  }

  suspend fun deleteDocument(documentId: String): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      DocumentActions_DeleteDocument_Mutation(input = DeleteDocumentInput(documentId = documentId))
    )
  }

  internal suspend fun exportDocument(
    documentId: String,
    format: DocumentExportFormat,
    layout: ExportDocumentPageLayoutInput?,
  ): Result<DocumentExportFile, Nothing> = result {
    val response =
      Apollo.executeMutation(
        DocumentExportSheet_ExportDocument_Mutation(
          input =
            ExportDocumentInput.Builder()
              .documentId(documentId)
              .format(format)
              .layout(layout)
              .build()
        ),
        doNotStore = true,
      )

    DocumentExportFile(
      bytes = response.exportDocument.data,
      filename = response.exportDocument.filename,
      mimeType = response.exportDocument.mimeType,
    )
  }
}

internal class DocumentExportFile(
  val bytes: ByteArray,
  val filename: String,
  val mimeType: String,
)

private fun placeholderData() =
  DocumentScreen_Query.Data(PlaceholderResolver) {
    entity = buildEntity {
      val now = Clock.System.now()

      id = "placeholder-document-entity"
      depth = 1
      url = ""
      type = EntityType.DOCUMENT
      icon = "file-text"
      iconColor = "gray"
      visibility = EntityVisibility.PRIVATE
      availability = EntityAvailability.PRIVATE
      goal = null
      site = buildSite {
        id = "placeholder-site"
        name = text(4..8)
      }
      ancestors = emptyList()
      node = buildDocument {
        id = "placeholder-document"
        title = text(5..12)
        subtitle = text(8..16)
        type = DocumentType.NORMAL
        locked = false
        createdAt = now
        updatedAt = now
        characterCount = 0
        characterCountChange = buildCharacterCountChange {
          additions = 0
          date = now
          deletions = 0
        }
      }
    }
  }
