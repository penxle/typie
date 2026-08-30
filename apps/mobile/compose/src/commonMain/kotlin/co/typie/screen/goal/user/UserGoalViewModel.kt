package co.typie.screen.goal.user

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.UserGoalScreen_DeleteUserGoal_Mutation
import co.typie.graphql.UserGoalScreen_Query
import co.typie.graphql.UserGoalScreen_UpdateUserGoal_Mutation
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.type.UpdateUserGoalInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result

class UserGoalViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      UserGoalScreen_Query()
    }

  suspend fun save(targetCharacterCount: Int): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      UserGoalScreen_UpdateUserGoal_Mutation(
        input = UpdateUserGoalInput(targetCharacterCount = targetCharacterCount)
      )
    )
  }

  suspend fun delete(): Result<Unit, Nothing> = result {
    Apollo.executeMutation(UserGoalScreen_DeleteUserGoal_Mutation())
  }
}

private fun placeholderData() =
  UserGoalScreen_Query.Data(PlaceholderResolver) {
    me = buildUser {
      goal = null
      goalHistory = emptyList()
      characterCountChanges = emptyList()
    }
  }
