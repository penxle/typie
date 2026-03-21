package co.typie.screen.login

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.auth.sso.AppleSingleSignOnProvider
import co.typie.auth.sso.GoogleSingleSignOnProvider
import co.typie.auth.sso.KakaoSingleSignOnProvider
import co.typie.auth.sso.NaverSingleSignOnProvider
import co.typie.graphql.LoginScreen_AuthorizeSingleSignOn_Mutation
import co.typie.graphql.MutationResult
import co.typie.graphql.executeMutation
import co.typie.graphql.type.AuthorizeSingleSignOnInput
import co.typie.graphql.type.SingleSignOnProvider
import co.typie.overlay.Loader
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import com.apollographql.apollo.ApolloClient
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class LoginViewModel(
  private val apolloClient: ApolloClient,
  private val toast: Toast,
  private val loader: Loader,
) : ViewModel() {
  fun loginWith(provider: SingleSignOnProvider, ctx: Any?) {
    viewModelScope.launch {
      val result = try {
        loader.runWith {
          val provider = when (provider) {
            SingleSignOnProvider.GOOGLE -> GoogleSingleSignOnProvider()
            SingleSignOnProvider.KAKAO -> KakaoSingleSignOnProvider()
            SingleSignOnProvider.NAVER -> NaverSingleSignOnProvider()
            SingleSignOnProvider.APPLE -> AppleSingleSignOnProvider()
            else -> throw IllegalArgumentException("Unknown provider: $provider")
          }

          val credential = provider.authenticate(ctx)

          apolloClient.executeMutation(
            LoginScreen_AuthorizeSingleSignOn_Mutation(
              AuthorizeSingleSignOnInput(
                provider = credential.provider,
                params = credential.params,
              )
            )
          )
        }
      } catch (e: CancellationException) {
        throw e
      } catch (_: Exception) {
        null
      }

      val message = when (result) {
        is MutationResult.Success -> null
        is MutationResult.Error -> "오류가 발생했어요. 잠시 후 다시 시도해주세요."
        else -> "로그인에 실패했어요. 다시 시도해주세요."
      }

      message?.let { toast.show(ToastType.Error, it) }
    }
  }
}
