package co.typie.screen.login_with_email

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.LoginWithEmailScreen_LoginWithEmail_Mutation
import co.typie.graphql.MutationResult
import co.typie.graphql.executeMutation
import co.typie.graphql.type.LoginWithEmailInput
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import com.apollographql.apollo.ApolloClient
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.koin.core.annotation.KoinViewModel

data class LoginWithEmailState(
  val email: String = "",
  val password: String = "",
  val emailError: String? = null,
  val passwordError: String? = null,
)

@KoinViewModel
class LoginWithEmailViewModel(
  private val apolloClient: ApolloClient,
  private val toast: Toast,
) : ViewModel() {
  private val _state = MutableStateFlow(LoginWithEmailState())
  val state: StateFlow<LoginWithEmailState> = _state

  fun setEmail(value: String) {
    _state.update { it.copy(email = value, emailError = null) }
  }

  fun setPassword(value: String) {
    _state.update { it.copy(password = value, passwordError = null) }
  }

  fun submit() {
    val current = _state.value

    val emailError = validateEmail(current.email)
    val passwordError = validatePassword(current.password)

    if (emailError != null || passwordError != null) {
      _state.update { it.copy(emailError = emailError, passwordError = passwordError) }
      return
    }

    viewModelScope.launch {
      val result = apolloClient.executeMutation(
        LoginWithEmailScreen_LoginWithEmail_Mutation(
          LoginWithEmailInput(email = current.email, password = current.password),
        ),
      )

      when (result) {
        is MutationResult.Success -> {}
        is MutationResult.Failure -> {
          val message = when (result.error.code) {
            "invalid_credentials" -> "이메일 또는 비밀번호가 올바르지 않아요."
            "password_not_set" -> "비밀번호가 설정되지 않았어요."
            else -> "오류가 발생했어요. 잠시 후 다시 시도해주세요."
          }
          toast.show(ToastType.Error, message)
        }
        is MutationResult.Error -> {
          toast.show(ToastType.Error, "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
        }
      }
    }
  }

  private fun validateEmail(email: String): String? {
    if (email.isBlank()) return "이메일을 입력해주세요."
    val emailRegex = Regex("^[A-Za-z0-9+_.-]+@[A-Za-z0-9.-]+\\.[A-Za-z]{2,}$")
    if (!emailRegex.matches(email)) return "올바른 이메일 형식을 입력해주세요."
    return null
  }

  private fun validatePassword(password: String): String? {
    if (password.isBlank()) return "비밀번호를 입력해주세요."
    return null
  }
}
