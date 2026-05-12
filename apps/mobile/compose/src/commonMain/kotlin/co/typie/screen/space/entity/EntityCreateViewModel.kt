package co.typie.screen.space.entity

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import co.typie.graphql.Apollo
import co.typie.graphql.EntityContainer_CreateDocument_Mutation
import co.typie.graphql.EntityContainer_CreateFolder_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.CreateDocumentInput
import co.typie.graphql.type.CreateFolderInput
import co.typie.result.Result
import co.typie.result.loading

class EntityCreateViewModel : ViewModel() {
  var isCreating by mutableStateOf(false)
    private set

  suspend fun createDocument(
    siteId: String,
    parentEntityId: String? = null,
  ): Result<String, Nothing> =
    loading({ isCreating = it }) {
      val input =
        CreateDocumentInput.Builder()
          .siteId(siteId)
          .apply {
            if (parentEntityId != null) {
              this.parentEntityId(parentEntityId)
            }
          }
          .v2(true)
          .build()

      Apollo.executeMutation(EntityContainer_CreateDocument_Mutation(input = input))
        .createDocument
        .entity
        .id
    }

  suspend fun createFolder(
    siteId: String,
    parentEntityId: String? = null,
  ): Result<String, Nothing> =
    loading({ isCreating = it }) {
      val input =
        CreateFolderInput.Builder()
          .siteId(siteId)
          .name("새 폴더")
          .apply {
            if (parentEntityId != null) {
              this.parentEntityId(parentEntityId)
            }
          }
          .build()

      Apollo.executeMutation(EntityContainer_CreateFolder_Mutation(input = input))
        .createFolder
        .entity
        .id
    }
}
