package co.typie.screen.settings.updatepassword

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.ext.presence
import co.typie.form.FormState
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TypieError
import co.typie.graphql.UpdatePasswordScreen_Query
import co.typie.graphql.UpdatePasswordScreen_UpdatePassword_Mutation
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.type.UpdatePasswordInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.loading
import com.apollographql.apollo.api.Optional
import io.ktor.client.request.invoke
import kotlinx.coroutines.CoroutineScope

class UpdatePasswordForm(scope: CoroutineScope) : FormState(scope) {
  val currentPassword = field("")
  val newPassword = field("") { required("새 비밀번호를 입력해주세요.") }
  val confirmPassword = field("") { required("비밀번호 확인을 입력해주세요.") }
}

sealed interface UpdatePasswordError {
  data object ValidationFailed : UpdatePasswordError
}

class UpdatePasswordViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      onInitialData = {},
    ) {
      UpdatePasswordScreen_Query()
    }

  val form = UpdatePasswordForm(viewModelScope)
  var isSubmitting by mutableStateOf(false)
    private set

  suspend fun submit(): Result<Unit, UpdatePasswordError> {
    if (!form.validate()) return Result.Err(UpdatePasswordError.ValidationFailed)

    return loading({ isSubmitting = it }) {
      try {
        Apollo.executeMutation(
          UpdatePasswordScreen_UpdatePassword_Mutation(
            input =
              UpdatePasswordInput(
                currentPassword = Optional.presentIfNotNull(form.currentPassword.value.presence),
                newPassword = form.newPassword.value,
              )
          )
        )
      } catch (e: TypieError) {
        when (e.code) {
          "invalid_password" -> {
            form.currentPassword.errors = listOf("비밀번호가 일치하지 않아요.")
            form.focusFirstError()
            raise(UpdatePasswordError.ValidationFailed)
          }

          else -> throw e
        }
      }
    }
  }
}

private fun placeholderData() =
  UpdatePasswordScreen_Query.Data(PlaceholderResolver) { me = buildUser { hasPassword = true } }
