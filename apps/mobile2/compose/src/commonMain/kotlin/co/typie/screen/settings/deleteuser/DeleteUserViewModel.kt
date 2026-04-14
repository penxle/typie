package co.typie.screen.settings.deleteuser

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import co.typie.graphql.Apollo
import co.typie.graphql.DeleteUserScreen_DeleteUser_Mutation
import co.typie.graphql.executeMutation
import co.typie.result.Result
import co.typie.result.loading

class DeleteUserViewModel : ViewModel() {
  var isSubmitting by mutableStateOf(false)
    private set

  suspend fun deleteUser(): Result<Unit, Nothing> =
    loading({ isSubmitting = it }) {
      Apollo.executeMutation(DeleteUserScreen_DeleteUser_Mutation())
    }
}
