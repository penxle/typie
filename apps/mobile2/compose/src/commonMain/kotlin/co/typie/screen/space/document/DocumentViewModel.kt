package co.typie.screen.space.document

import androidx.lifecycle.ViewModel
import co.typie.graphql.Apollo
import co.typie.graphql.DocumentActions_DeleteDocument_Mutation
import co.typie.graphql.DocumentActions_UpdateDocument_Mutation
import co.typie.graphql.DocumentActions_UpdateEntityIcon_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.DeleteDocumentInput
import co.typie.graphql.type.UpdateDocumentInput
import co.typie.graphql.type.UpdateEntityIconInput
import co.typie.result.Result
import co.typie.result.result
import co.typie.screen.space.entity.EntityIconPickerSheetModel

class DocumentViewModel : ViewModel(), DocumentRenameSheetModel, EntityIconPickerSheetModel {
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
      DocumentActions_UpdateEntityIcon_Mutation(
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
}
