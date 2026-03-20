package co.typie.screen.login

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.touchlab.kermit.Logger
import co.typie.auth.sso.AppleSingleSignOnProvider
import co.typie.auth.sso.GoogleSingleSignOnProvider
import co.typie.auth.sso.KakaoSingleSignOnProvider
import co.typie.auth.sso.NaverSingleSignOnProvider
import co.typie.auth.sso.SingleSignOnCredential
import co.typie.graphql.LoginScreen_AuthorizeSingleSignOn_Mutation
import co.typie.graphql.type.AuthorizeSingleSignOnInput
import co.typie.overlay.Loader
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import com.apollographql.apollo.ApolloClient
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import org.koin.core.annotation.KoinViewModel

private class UserFacingException(message: String) : Exception(message)

@KoinViewModel
class LoginViewModel(
  private val apolloClient: ApolloClient,
  private val toast: Toast,
  private val loader: Loader,
) : ViewModel() {

  fun loginWithGoogle(ctx: Any? = null) = loginWith(GoogleSingleSignOnProvider(), ctx)
  fun loginWithKakao(ctx: Any? = null) = loginWith(KakaoSingleSignOnProvider(), ctx)
  fun loginWithNaver(ctx: Any? = null) = loginWith(NaverSingleSignOnProvider(), ctx)
  fun loginWithApple(ctx: Any? = null) = loginWith(AppleSingleSignOnProvider(), ctx)

  private fun loginWith(provider: co.typie.auth.sso.SingleSignOnAdapter, ctx: Any?) {
    viewModelScope.launch {
      try {
        loader.runWith {
          val credential = provider.authenticate(ctx)
          executeMutation(credential)
        }
      } catch (e: CancellationException) {
        throw e
      } catch (e: UserFacingException) {
        toast.show(ToastType.Error, e.message!!)
      } catch (e: Exception) {
        Logger.e(e) { "Failed to login with $provider" }
        toast.show(ToastType.Error, "로그인에 실패했어요. 다시 시도해주세요.")
      }
    }
  }

  private suspend fun executeMutation(credential: SingleSignOnCredential) {
    val input = AuthorizeSingleSignOnInput(
      provider = credential.provider,
      params = credential.params,
    )

    val response = apolloClient
      .mutation(LoginScreen_AuthorizeSingleSignOn_Mutation(input))
      .execute()

    val gqlError = response.errors?.firstOrNull()
    if (gqlError != null) {
      val type = gqlError.extensions?.get("type") as? String
      val code = gqlError.extensions?.get("code") as? String

      val message = if (type == "TypieError") {
        when (code) {
          else -> "로그인에 실패했어요. 다시 시도해주세요."
        }
      } else {
        "오류가 발생했어요. 잠시 후 다시 시도해주세요."
      }

      throw UserFacingException(message)
    }
  }
}
