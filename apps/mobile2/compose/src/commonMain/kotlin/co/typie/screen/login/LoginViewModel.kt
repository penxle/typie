package co.typie.screen.login

import androidx.lifecycle.viewModelScope
import co.touchlab.kermit.Logger
import co.typie.auth.sso.AppleSingleSignOnProvider
import co.typie.auth.sso.GoogleSingleSignOnProvider
import co.typie.auth.sso.KakaoSingleSignOnProvider
import co.typie.auth.sso.NaverSingleSignOnProvider
import co.typie.form.FormState
import co.typie.form.email
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.LoginScreen_AuthorizeSingleSignOn_Mutation
import co.typie.graphql.LoginWithEmailScreen_LoginWithEmail_Mutation
import co.typie.graphql.TypieError
import co.typie.graphql.type.AuthorizeSingleSignOnInput
import co.typie.graphql.type.LoginWithEmailInput
import co.typie.graphql.type.SingleSignOnProvider
import co.typie.overlay.Loader
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.ui.state.AsyncAction
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class LoginSingleSignOnViewModel(
  private val toast: Toast,
  private val loader: Loader,
) : GraphQLViewModel() {
  fun loginWith(provider: SingleSignOnProvider, ctx: Any?, onSuccess: () -> Unit = {}) {
    viewModelScope.launch {
      try {
        loader.runWith {
          val provider = when (provider) {
            SingleSignOnProvider.GOOGLE -> GoogleSingleSignOnProvider()
            SingleSignOnProvider.KAKAO -> KakaoSingleSignOnProvider()
            SingleSignOnProvider.NAVER -> NaverSingleSignOnProvider()
            SingleSignOnProvider.APPLE -> AppleSingleSignOnProvider()
            else -> throw IllegalArgumentException("Unknown provider: $provider")
          }

          val credential = provider.authenticate(ctx)

          executeMutation(
            LoginScreen_AuthorizeSingleSignOn_Mutation(
              AuthorizeSingleSignOnInput(
                provider = credential.provider,
                params = JsonObject(credential.params.mapValues { (_, v) -> JsonPrimitive(v) }),
              )
            )
          )
        }

        onSuccess()
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.e(e) { "Failed to login with $provider" }
        toast.show(ToastType.Error, "로그인에 실패했어요. 다시 시도해주세요.")
      }
    }
  }
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
  private val toast: Toast,
) : GraphQLViewModel() {
  val state = LoginWithEmailState(viewModelScope)
  val submitAction = AsyncAction(viewModelScope)

  fun submit(onSubmit: () -> Unit) {
    submitAction.launch(
      onFailure = { e ->
        when (e) {
          is TypieError -> {
            when (e.code) {
              "invalid_credentials" -> toast.show(ToastType.Error, "이메일 또는 비밀번호가 올바르지 않아요.")
              "password_not_set" -> toast.show(ToastType.Error, "비밀번호가 설정되지 않았어요.")
              else -> toast.show(ToastType.Error, "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
            }
          }

          else -> {
            Logger.e(e) { "Failed to login with email" }
            toast.show(ToastType.Error, "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
          }
        }
      },
    ) {
        if (!state.form.validate()) return@launch

        executeMutation(
          LoginWithEmailScreen_LoginWithEmail_Mutation(
            LoginWithEmailInput(
              email = state.form.email.value,
              password = state.form.password.value
            ),
          ),
        )

        onSubmit()
    }
  }
}
