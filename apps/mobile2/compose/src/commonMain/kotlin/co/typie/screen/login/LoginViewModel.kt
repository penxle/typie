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

          val result =
            apolloClient.executeMutation(
              LoginScreen_AuthorizeSingleSignOn_Mutation(
                AuthorizeSingleSignOnInput(
                  provider = credential.provider,
                  params = credential.params,
                )
              )
            )

          when (result) {
            is MutationResult.Success -> {}
            is MutationResult.Failure -> {
              val message = when (result.error.code) {
                else -> "로그인에 실패했어요. 다시 시도해주세요."
              }
              toast.show(ToastType.Error, message)
            }

            is MutationResult.Error -> {
              toast.show(ToastType.Error, "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
            }
          }
        }
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        toast.show(ToastType.Error, "로그인에 실패했어요. 다시 시도해주세요.")
      }
    }
  }
}
