package co.typie.screen.document.document

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.entity.EntityIconPickerSheetModel
import co.typie.graphql.Apollo
import co.typie.graphql.DocumentActions_DeleteDocument_Mutation
import co.typie.graphql.DocumentActions_DuplicateDocument_Mutation
import co.typie.graphql.DocumentActions_UpdateDocumentType_Mutation
import co.typie.graphql.DocumentActions_UpdateEntityIcon_Mutation
import co.typie.graphql.DocumentScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildCharacterCountChange
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildSite
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.type.DeleteDocumentInput
import co.typie.graphql.type.DocumentType
import co.typie.graphql.type.DuplicateDocumentInput
import co.typie.graphql.type.EntityAvailability
import co.typie.graphql.type.EntityType
import co.typie.graphql.type.EntityVisibility
import co.typie.graphql.type.UpdateDocumentTypeInput
import co.typie.graphql.type.UpdateEntityIconInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import kotlin.time.Clock

class DocumentViewModel : ViewModel(), EntityIconPickerSheetModel {
  var entityId by mutableStateOf("")

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
}

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
