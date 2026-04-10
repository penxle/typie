package co.typie.screen.auth.login

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.auth.sso.AppleSingleSignOnProvider
import co.typie.auth.sso.GoogleSingleSignOnProvider
import co.typie.auth.sso.KakaoSingleSignOnProvider
import co.typie.auth.sso.NaverSingleSignOnProvider
import co.typie.form.FormState
import co.typie.form.email
import co.typie.graphql.Apollo
import co.typie.graphql.LoginScreen_AuthorizeSingleSignOn_Mutation
import co.typie.graphql.LoginWithEmailScreen_LoginWithEmail_Mutation
import co.typie.graphql.TypieError
import co.typie.graphql.executeMutation
import co.typie.graphql.type.AuthorizeSingleSignOnInput
import co.typie.graphql.type.LoginWithEmailInput
import co.typie.graphql.type.SingleSignOnProvider
import co.typie.platform.ActivityContext
import co.typie.result.Result
import co.typie.result.loading
import co.typie.result.result
import kotlinx.coroutines.CoroutineScope
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

class LoginSingleSignOnViewModel : ViewModel() {
  context(_: ActivityContext)
  suspend fun loginWith(provider: SingleSignOnProvider): Result<Unit, Nothing> = result {
    val ssoProvider =
      when (provider) {
        SingleSignOnProvider.GOOGLE -> GoogleSingleSignOnProvider()
        SingleSignOnProvider.KAKAO -> KakaoSingleSignOnProvider()
        SingleSignOnProvider.NAVER -> NaverSingleSignOnProvider()
        SingleSignOnProvider.APPLE -> AppleSingleSignOnProvider()
        else -> throw IllegalArgumentException("Unknown provider: $provider")
      }

    val credential = ssoProvider.authenticate()

    Apollo.executeMutation(
      LoginScreen_AuthorizeSingleSignOn_Mutation(
        AuthorizeSingleSignOnInput(
          provider = credential.provider,
          params = JsonObject(credential.params.mapValues { (_, v) -> JsonPrimitive(v) }),
        )
      )
    )
  }
}

sealed interface LoginWithEmailError {
  data object ValidationFailed : LoginWithEmailError

  data object InvalidCredentials : LoginWithEmailError

  data object PasswordNotSet : LoginWithEmailError

  data class Unknown(val code: String) : LoginWithEmailError
}

class LoginWithEmailForm(scope: CoroutineScope) : FormState(scope) {
  val email =
    field("") {
      required("이메일을 입력해주세요.")
      email("올바른 이메일 형식을 입력해주세요.")
    }

  val password = field("") { required("비밀번호를 입력해주세요.") }
}

class LoginWithEmailState(scope: CoroutineScope) {
  val form = LoginWithEmailForm(scope)
}

class LoginWithEmailViewModel : ViewModel() {
  val state = LoginWithEmailState(viewModelScope)
  var isSubmitting by mutableStateOf(false)
    private set

  suspend fun submit(): Result<Unit, LoginWithEmailError> {
    if (!state.form.validate()) return Result.Err(LoginWithEmailError.ValidationFailed)

    return loading({ isSubmitting = it }) {
      try {
        Apollo.executeMutation(
          LoginWithEmailScreen_LoginWithEmail_Mutation(
            LoginWithEmailInput(
              email = state.form.email.value,
              password = state.form.password.value,
            )
          )
        )
      } catch (e: TypieError) {
        when (e.code) {
          "invalid_credentials" -> raise(LoginWithEmailError.InvalidCredentials)
          "password_not_set" -> raise(LoginWithEmailError.PasswordNotSet)
          else -> raise(LoginWithEmailError.Unknown(e.code))
        }
      }
    }
  }
}
