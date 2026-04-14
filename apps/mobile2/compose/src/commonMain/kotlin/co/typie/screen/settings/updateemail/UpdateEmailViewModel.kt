package co.typie.screen.settings.updateemail

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.form.FormState
import co.typie.form.email
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TypieError
import co.typie.graphql.UpdateEmailScreen_Query
import co.typie.graphql.UpdateEmailScreen_SendEmailUpdateEmail_Mutation
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.type.SendEmailUpdateEmailInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.loading
import kotlinx.coroutines.CoroutineScope

sealed interface UpdateEmailError {
  data object ValidationFailed : UpdateEmailError
}

class UpdateEmailForm(scope: CoroutineScope) : FormState(scope) {
  val email =
    field("") {
      required("이메일 주소를 입력해주세요.")
      email("유효한 이메일 주소를 입력해주세요.")
    }
}

class UpdateEmailViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      UpdateEmailScreen_Query()
    }

  val form = UpdateEmailForm(viewModelScope)
  var isSubmitting by mutableStateOf(false)
    private set

  suspend fun submit(): Result<Unit, UpdateEmailError> {
    if (!form.validate()) return Result.Err(UpdateEmailError.ValidationFailed)

    return loading({ isSubmitting = it }) {
      try {
        Apollo.executeMutation(
          UpdateEmailScreen_SendEmailUpdateEmail_Mutation(
            input = SendEmailUpdateEmailInput(email = form.email.value)
          )
        )
      } catch (e: TypieError) {
        when (e.code) {
          "user_email_exists" -> {
            form.email.errors = listOf("이미 사용중인 이메일이에요.")
            form.focusFirstError()
            raise(UpdateEmailError.ValidationFailed)
          }
          else -> throw e
        }
      }

      form.commit()
    }
  }
}

private fun placeholderData() =
  UpdateEmailScreen_Query.Data(PlaceholderResolver) { me = buildUser { email = text(12..20) } }
