package co.typie.screen.update_email

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
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
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
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
  var isSubmitting by mutableStateOf(false)
}

@KoinViewModel
class UpdateEmailViewModel(
  private val toast: Toast,
) : GraphQLViewModel() {
  val state = UpdateEmailScreenState(viewModelScope)

  val query = watchQuery(
    placeholderData = placeholderData(),
  ) { UpdateEmailScreen_Query() }

  fun submit(onSuccess: suspend () -> Unit) {
    viewModelScope.launch {
      state.isSubmitting = true
      try {
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
      } catch (e: CancellationException) {
        throw e
      } catch (e: TypieError) {
        val message = updateEmailErrorMessage(e.code)
        if (message != null) {
          toast.show(ToastType.Error, message)
        } else {
          toast.show(ToastType.Error, e.message ?: "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
        }
      } catch (e: Exception) {
        Logger.e(e) { "Failed to send update email request" }
        toast.show(ToastType.Error, "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
      } finally {
        state.isSubmitting = false
      }
    }
  }
}

private fun placeholderData() = UpdateEmailScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    email = text(12..20)
  }
}
