package co.typie.screen.login

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
import co.typie.graphql.LoginScreen_AuthorizeSingleSignOn_Mutation
import co.typie.graphql.LoginWithEmailScreen_LoginWithEmail_Mutation
import co.typie.graphql.TypieError
import co.typie.graphql.executeMutation
import co.typie.graphql.type.AuthorizeSingleSignOnInput
import co.typie.graphql.type.LoginWithEmailInput
import co.typie.graphql.type.SingleSignOnProvider
import co.typie.result.Result
import co.typie.result.loading
import co.typie.result.result
import com.apollographql.apollo.ApolloClient
import kotlinx.coroutines.CoroutineScope
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class LoginSingleSignOnViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  suspend fun loginWith(provider: SingleSignOnProvider, ctx: Any?): Result<Unit, Nothing> = result {
    val ssoProvider = when (provider) {
      SingleSignOnProvider.GOOGLE -> GoogleSingleSignOnProvider()
      SingleSignOnProvider.KAKAO -> KakaoSingleSignOnProvider()
      SingleSignOnProvider.NAVER -> NaverSingleSignOnProvider()
      SingleSignOnProvider.APPLE -> AppleSingleSignOnProvider()
      else -> throw IllegalArgumentException("Unknown provider: $provider")
    }

    val credential = ssoProvider.authenticate(ctx)

    apolloClient.executeMutation(
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
  data object InvalidCredentials : LoginWithEmailError
  data object PasswordNotSet : LoginWithEmailError
  data class Unknown(val code: String) : LoginWithEmailError
}

class LoginWithEmailForm(scope: CoroutineScope) : FormState(scope) {
  val email = field("") {
    required("이메일을 입력해주세요.")
    email("올바른 이메일 형식을 입력해주세요.")
  }

  val password = field("") {
    required("비밀번호를 입력해주세요.")
  }
}

class LoginWithEmailState(scope: CoroutineScope) {
  val form = LoginWithEmailForm(scope)
}

@KoinViewModel
class LoginWithEmailViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  val state = LoginWithEmailState(viewModelScope)
  var isSubmitting by mutableStateOf(false)
    private set

  suspend fun submit(): Result<Unit, LoginWithEmailError> {
    if (!state.form.validate()) return Result.Ok(Unit)

    return loading({ isSubmitting = it }) {
      try {
        apolloClient.executeMutation(
          LoginWithEmailScreen_LoginWithEmail_Mutation(
            LoginWithEmailInput(
              email = state.form.email.value,
              password = state.form.password.value
            ),
          ),
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
