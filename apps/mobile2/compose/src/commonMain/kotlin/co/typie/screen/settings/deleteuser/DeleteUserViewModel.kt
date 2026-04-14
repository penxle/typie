package co.typie.screen.settings.deleteuser

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.form.FormState
import co.typie.graphql.Apollo
import co.typie.graphql.DeleteUserScreen_DeleteUser_Mutation
import co.typie.graphql.TypieError
import co.typie.graphql.executeMutation
import co.typie.result.Result
import co.typie.result.loading
import kotlinx.coroutines.CoroutineScope

sealed interface DeleteUserError {
  data class ValidationFailed(val errorMessage: String) : DeleteUserError

  data object OverdueInvoicesExist : DeleteUserError
}

class DeleteUserForm(scope: CoroutineScope) : FormState(scope) {
  val acknowledged =
    field(false) {
      required("유의사항을 모두 확인해주세요.")
      rule { if (!it) "유의사항을 모두 확인해주세요." else null }
      focusable = false
    }
}

class DeleteUserViewModel : ViewModel() {
  val form = DeleteUserForm(viewModelScope)
  var isSubmitting by mutableStateOf(false)
    private set

  suspend fun submit(): Result<Unit, DeleteUserError> {
    if (!form.validate()) return Result.Err(DeleteUserError.ValidationFailed(form.errorMessage!!))

    return loading({ isSubmitting = it }) {
      try {
        Apollo.executeMutation(DeleteUserScreen_DeleteUser_Mutation())
      } catch (e: TypieError) {
        when (e.code) {
          "overdue_invoices_exist" -> raise(DeleteUserError.OverdueInvoicesExist)
          else -> throw e
        }
      }
    }
  }
}
