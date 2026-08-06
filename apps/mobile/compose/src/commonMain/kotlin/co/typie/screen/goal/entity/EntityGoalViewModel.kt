package co.typie.screen.goal.entity

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.datetime.atKstStartOfDay
import co.typie.graphql.Apollo
import co.typie.graphql.EntityGoalScreen_DeleteEntityGoal_Mutation
import co.typie.graphql.EntityGoalScreen_Query
import co.typie.graphql.EntityGoalScreen_UpdateEntityGoal_Mutation
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.type.DeleteEntityGoalInput
import co.typie.graphql.type.UpdateEntityGoalInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import kotlinx.datetime.LocalDate

class EntityGoalViewModel : ViewModel() {
  var entityId by mutableStateOf("")

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      skip = { entityId.isBlank() },
    ) {
      EntityGoalScreen_Query(entityId = entityId)
    }

  suspend fun save(targetCharacterCount: Int, dueDate: LocalDate?): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      EntityGoalScreen_UpdateEntityGoal_Mutation(
        input =
          UpdateEntityGoalInput.Builder()
            .entityId(entityId)
            .targetCharacterCount(targetCharacterCount)
            .dueAt(dueDate?.atKstStartOfDay())
            .build()
      )
    )
  }

  suspend fun delete(): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      EntityGoalScreen_DeleteEntityGoal_Mutation(input = DeleteEntityGoalInput(entityId = entityId))
    )
  }
}

private fun placeholderData() =
  EntityGoalScreen_Query.Data(PlaceholderResolver) {
    entity = buildEntity {
      id = "placeholder-goal-entity"
      goal = null
      characterCountHistory = emptyList()
      node = buildDocument {
        id = "placeholder-goal-document"
        title = text(5..12)
        characterCount = 0
      }
    }
  }
