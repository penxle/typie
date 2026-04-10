package co.typie.screen.update_password

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.form.FormState
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TypieError
import co.typie.graphql.UpdatePasswordScreen_Query
import co.typie.graphql.UpdatePasswordScreen_UpdatePassword_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.UpdatePasswordInput
import co.typie.graphql.type.buildUser
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.loading
import co.typie.result.result
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.Optional
import kotlinx.coroutines.CoroutineScope
import org.koin.core.annotation.KoinViewModel

enum class PasswordField {
  CurrentPassword,
  ConfirmPassword,
}

data class PasswordValidationError(
  val field: PasswordField,
  val message: String,
)

internal fun validatePasswordSubmission(
  hasPassword: Boolean,
  currentPassword: String,
  newPassword: String,
  confirmPassword: String,
): PasswordValidationError? {
  if (hasPassword && currentPassword.isBlank()) {
    return PasswordValidationError(
      field = PasswordField.CurrentPassword,
      message = "현재 비밀번호를 입력해주세요.",
    )
  }

  if (newPassword != confirmPassword) {
    return PasswordValidationError(
      field = PasswordField.ConfirmPassword,
      message = "비밀번호가 일치하지 않아요.",
    )
  }

  return null
}

class UpdatePasswordForm(scope: CoroutineScope) : FormState(scope) {
  val currentPassword = field("")
  val newPassword = field("") {
    required("새 비밀번호를 입력해주세요.")
  }
  val confirmPassword = field("") {
    required("비밀번호 확인을 입력해주세요.")
  }

  fun updateHasPassword(hasPassword: Boolean) {
    currentPassword.focusable = hasPassword
  }
}

class UpdatePasswordScreenState(scope: CoroutineScope) {
  val form = UpdatePasswordForm(scope)
}

sealed interface UpdatePasswordError {
  data object InvalidPassword : UpdatePasswordError
  data object CurrentPasswordRequired : UpdatePasswordError
}

@KoinViewModel
class UpdatePasswordViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  val state = UpdatePasswordScreenState(viewModelScope)
  var isSubmitting by mutableStateOf(false)
    private set

  val query =
    apolloClient.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      onInitialData = { data ->
        state.form.updateHasPassword(data.me.hasPassword)
      },
    ) { UpdatePasswordScreen_Query() }

  suspend fun submit(): Result<Unit, UpdatePasswordError> {
    val hasPassword = query.data.me.hasPassword
    state.form.updateHasPassword(hasPassword)

    if (!state.form.validate()) return Result.Ok(Unit)

    val validationError = validatePasswordSubmission(
      hasPassword = hasPassword,
      currentPassword = state.form.currentPassword.value,
      newPassword = state.form.newPassword.value,
      confirmPassword = state.form.confirmPassword.value,
    )
    if (validationError != null) {
      applyValidationError(validationError)
      return Result.Ok(Unit)
    }

    return loading({ isSubmitting = it }) {
      try {
        apolloClient.executeMutation(
          UpdatePasswordScreen_UpdatePassword_Mutation(
            input = UpdatePasswordInput(
              currentPassword = if (hasPassword) {
                Optional.present(state.form.currentPassword.value)
              } else {
                Optional.Absent
              },
              newPassword = state.form.newPassword.value,
            ),
          ),
        )
      } catch (e: TypieError) {
        val serverError = applyServerError(e)
        if (serverError != null) raise(serverError)
        throw e
      }

      // TODO: 비밀번호 변경 트래킹
    }
  }

  private fun applyValidationError(error: PasswordValidationError) {
    when (error.field) {
      PasswordField.CurrentPassword -> {
        state.form.currentPassword.setErrors(listOf(error.message))
        state.form.currentPassword.focusRequester.requestFocus()
      }

      PasswordField.ConfirmPassword -> {
        state.form.confirmPassword.setErrors(listOf(error.message))
        state.form.confirmPassword.focusRequester.requestFocus()
      }
    }
  }

  private fun applyServerError(error: TypieError): UpdatePasswordError? {
    val (validationError, typedError) = when (error.code) {
      "invalid_password" -> PasswordValidationError(
        field = PasswordField.CurrentPassword,
        message = "비밀번호가 일치하지 않습니다.",
      ) to UpdatePasswordError.InvalidPassword

      "current_password_required" -> PasswordValidationError(
        field = PasswordField.CurrentPassword,
        message = "현재 비밀번호를 입력해주세요.",
      ) to UpdatePasswordError.CurrentPasswordRequired

      else -> return null
    }

    applyValidationError(validationError)
    return typedError
  }
}

private fun placeholderData() = UpdatePasswordScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    hasPassword = true
  }
}
