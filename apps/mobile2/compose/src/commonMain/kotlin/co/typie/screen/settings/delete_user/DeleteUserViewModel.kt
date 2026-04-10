package co.typie.screen.settings.delete_user

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import co.typie.graphql.DeleteUserScreen_DeleteUser_Mutation
import co.typie.graphql.executeMutation
import co.typie.result.Result
import co.typie.result.loading
import co.typie.result.result
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class DeleteUserViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  var isSubmitting by mutableStateOf(false)
    private set

  suspend fun deleteUser(): Result<Unit, Nothing> = loading({ isSubmitting = it }) {
    apolloClient.executeMutation(DeleteUserScreen_DeleteUser_Mutation())
  }
}
