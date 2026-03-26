package co.typie.screen.update_password

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewModelScope
import co.touchlab.kermit.Logger
import co.typie.form.FormState
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TypieError
import co.typie.graphql.UpdatePasswordScreen_Query
import co.typie.graphql.UpdatePasswordScreen_UpdatePassword_Mutation
import co.typie.graphql.type.UpdatePasswordInput
import co.typie.graphql.type.buildUser
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import com.apollographql.apollo.api.Optional
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
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
  var isSubmitting by mutableStateOf(false)
}

@KoinViewModel
class UpdatePasswordViewModel(
  private val toast: Toast,
) : GraphQLViewModel() {
  val state = UpdatePasswordScreenState(viewModelScope)

  val query =
    watchQuery(
      placeholderData = placeholderData(),
      onInitialData = { data ->
        state.form.updateHasPassword(data.me.hasPassword)
      },
    ) { UpdatePasswordScreen_Query() }

  fun submit(onSubmit: suspend () -> Unit) {
    viewModelScope.launch {
      state.isSubmitting = true
      try {
        val hasPassword = query.data.me.hasPassword
        state.form.updateHasPassword(hasPassword)

        if (!state.form.validate()) return@launch

        val validationError = validatePasswordSubmission(
          hasPassword = hasPassword,
          currentPassword = state.form.currentPassword.value,
          newPassword = state.form.newPassword.value,
          confirmPassword = state.form.confirmPassword.value,
        )
        if (validationError != null) {
          applyValidationError(validationError)
          return@launch
        }

        executeMutation(
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

        toast.show(ToastType.Success, "비밀번호가 변경되었어요.")
        onSubmit()
      } catch (e: CancellationException) {
        throw e
      } catch (e: TypieError) {
        if (!applyServerError(e)) {
          toast.show(ToastType.Error, e.message ?: "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
        }
      } catch (e: Exception) {
        Logger.e(e) { "Failed to update password" }
        toast.show(ToastType.Error, "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
      } finally {
        state.isSubmitting = false
      }
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

  private fun applyServerError(error: TypieError): Boolean {
    val validationError = when (error.code) {
      "invalid_password" -> PasswordValidationError(
        field = PasswordField.CurrentPassword,
        message = "비밀번호가 일치하지 않습니다.",
      )

      "current_password_required" -> PasswordValidationError(
        field = PasswordField.CurrentPassword,
        message = "현재 비밀번호를 입력해주세요.",
      )

      else -> null
    }

    if (validationError == null) return false

    applyValidationError(validationError)
    return true
  }
}

private fun placeholderData() = UpdatePasswordScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    hasPassword = true
  }
}
