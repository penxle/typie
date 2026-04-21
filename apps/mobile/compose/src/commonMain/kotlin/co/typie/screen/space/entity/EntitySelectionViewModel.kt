package co.typie.screen.space.entity

import androidx.lifecycle.ViewModel
import co.typie.domain.entity.EntityIconPickerSheetModel
import co.typie.graphql.Apollo
import co.typie.graphql.EntitySelectionActions_DeleteEntities_Mutation
import co.typie.graphql.EntitySelectionActions_UpdateEntitiesIcon_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.DeleteEntitiesInput
import co.typie.graphql.type.UpdateEntitiesIconInput
import co.typie.result.Result
import co.typie.result.result

class EntitySelectionViewModel : ViewModel(), EntityIconPickerSheetModel {
  override suspend fun updateEntityIcons(
    entityIds: List<String>,
    icon: String?,
    iconColor: String?,
  ): Result<Unit, Nothing> = result {
    if (entityIds.isEmpty() || (icon == null && iconColor == null)) {
      return@result
    }

    Apollo.executeMutation(
      EntitySelectionActions_UpdateEntitiesIcon_Mutation(
        input =
          UpdateEntitiesIconInput.Builder()
            .entityIds(entityIds)
            .apply {
              if (icon != null) {
                icon(icon.trim())
              }
              if (iconColor != null) {
                iconColor(iconColor.trim())
              }
            }
            .build()
      )
    )
  }

  suspend fun deleteEntities(entityIds: List<String>): Result<Unit, Nothing> = result {
    if (entityIds.isEmpty()) {
      return@result
    }

    Apollo.executeMutation(
      EntitySelectionActions_DeleteEntities_Mutation(
        input = DeleteEntitiesInput(entityIds = entityIds)
      )
    )
  }
}
