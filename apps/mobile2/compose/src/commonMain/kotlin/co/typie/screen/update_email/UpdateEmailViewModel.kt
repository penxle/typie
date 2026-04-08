package co.typie.screen.update_email

import androidx.lifecycle.viewModelScope
import co.touchlab.kermit.Logger
import co.typie.form.FormState
import co.typie.form.email
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TypieError
import co.typie.graphql.UpdateEmailScreen_Query
import co.typie.graphql.UpdateEmailScreen_SendEmailUpdateEmail_Mutation
import co.typie.graphql.text
import co.typie.graphql.type.SendEmailUpdateEmailInput
import co.typie.graphql.type.buildUser
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.ui.state.AsyncAction
import kotlinx.coroutines.CoroutineScope
import org.koin.core.annotation.KoinViewModel

internal fun updateEmailErrorMessage(code: String): String? {
  return when (code) {
    "user_email_exists" -> "이미 사용중인 이메일이에요."
    else -> null
  }
}

class UpdateEmailForm(scope: CoroutineScope) : FormState(scope) {
  val email = field("") {
    required("이메일 주소를 입력해주세요.")
    email("유효한 이메일 주소를 입력해주세요.")
  }
}

class UpdateEmailScreenState(scope: CoroutineScope) {
  val form = UpdateEmailForm(scope)
}

@KoinViewModel
class UpdateEmailViewModel(
  private val toast: Toast,
) : GraphQLViewModel() {
  val state = UpdateEmailScreenState(viewModelScope)
  val submitAction = AsyncAction(viewModelScope)

  val query = watchQuery(
    placeholderData = placeholderData(),
  ) { UpdateEmailScreen_Query() }

  fun submit(onSuccess: suspend () -> Unit) {
    submitAction.launch(
      onFailure = { e ->
        when (e) {
          is TypieError -> {
            val message = updateEmailErrorMessage(e.code)
            if (message != null) {
              toast.show(ToastType.Error, message)
            } else {
              toast.show(ToastType.Error, e.message ?: "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
            }
          }

          else -> {
            Logger.e(e) { "Failed to send update email request" }
            toast.show(ToastType.Error, "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
          }
        }
      },
    ) {
        if (!state.form.validate()) return@launch

        executeMutation(
          UpdateEmailScreen_SendEmailUpdateEmail_Mutation(
            input = SendEmailUpdateEmailInput(
              email = state.form.email.value.trim(),
            ),
          ),
        )

        // TODO: 이메일 변경 트래킹
        state.form.commit()
        onSuccess()
    }
  }
}

private fun placeholderData() = UpdateEmailScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    email = text(12..20)
  }
}
