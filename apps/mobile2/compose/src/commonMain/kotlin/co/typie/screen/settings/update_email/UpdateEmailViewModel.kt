package co.typie.screen.settings.update_email

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.form.FormState
import co.typie.form.email
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TypieError
import co.typie.graphql.UpdateEmailScreen_Query
import co.typie.graphql.UpdateEmailScreen_SendEmailUpdateEmail_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.type.SendEmailUpdateEmailInput
import co.typie.graphql.type.buildUser
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.loading
import co.typie.result.result
import com.apollographql.apollo.ApolloClient
import kotlinx.coroutines.CoroutineScope
import org.koin.core.annotation.KoinViewModel

sealed interface UpdateEmailError {
  data object EmailAlreadyExists : UpdateEmailError
  data class Unknown(val code: String) : UpdateEmailError
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
  private val apolloClient: ApolloClient,
) : ViewModel() {
  val state = UpdateEmailScreenState(viewModelScope)
  var isSubmitting by mutableStateOf(false)
    private set

  val query = apolloClient.watchQuery(
    scope = viewModelScope,
    placeholderData = placeholderData(),
  ) { UpdateEmailScreen_Query() }

  suspend fun submit(): Result<Unit, UpdateEmailError> {
    if (!state.form.validate()) return Result.Ok(Unit)

    return loading({ isSubmitting = it }) {
      try {
        apolloClient.executeMutation(
          UpdateEmailScreen_SendEmailUpdateEmail_Mutation(
            input = SendEmailUpdateEmailInput(
              email = state.form.email.value.trim(),
            ),
          ),
        )
      } catch (e: TypieError) {
        when (e.code) {
          "user_email_exists" -> raise(UpdateEmailError.EmailAlreadyExists)
          else -> raise(UpdateEmailError.Unknown(e.code))
        }
      }

      // TODO: 이메일 변경 트래킹
      state.form.commit()
    }
  }
}

private fun placeholderData() = UpdateEmailScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    email = text(12..20)
  }
}
