package co.typie.screen.login_with_email

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.form.FormState
import co.typie.form.email
import co.typie.graphql.LoginWithEmailScreen_LoginWithEmail_Mutation
import co.typie.graphql.MutationResult
import co.typie.graphql.executeMutation
import co.typie.graphql.type.LoginWithEmailInput
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

class LoginForm : FormState() {
  val email = field("") {
    required("이메일을 입력해주세요.")
    email("올바른 이메일 형식을 입력해주세요.")
  }
  val password = field("") {
    required("비밀번호를 입력해주세요.")
  }
}

class LoginWithEmailState {
  val form = LoginForm()
}

@KoinViewModel
class LoginWithEmailViewModel(
  private val apolloClient: ApolloClient,
  private val toast: Toast,
) : ViewModel() {
  val state = LoginWithEmailState()

  fun submit() {
    state.form.submit(viewModelScope) {
      val result = apolloClient.executeMutation(
        LoginWithEmailScreen_LoginWithEmail_Mutation(
          LoginWithEmailInput(email = state.form.email.value, password = state.form.password.value),
        ),
      )

      val message = when (result) {
        is MutationResult.Success -> null
        is MutationResult.Failure if result.error.code == "invalid_credentials" -> "이메일 또는 비밀번호가 올바르지 않아요."
        is MutationResult.Failure if result.error.code == "password_not_set" -> "비밀번호가 설정되지 않았어요."
        else -> "오류가 발생했어요. 잠시 후 다시 시도해주세요."
      }

      message?.let { toast.show(ToastType.Error, it) }
    }
  }
}
