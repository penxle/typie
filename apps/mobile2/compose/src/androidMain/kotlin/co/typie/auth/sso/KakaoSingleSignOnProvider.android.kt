package co.typie.auth.sso

import co.typie.graphql.type.SingleSignOnProvider
import co.typie.platform.ActivityContext
import com.kakao.sdk.auth.model.Prompt
import com.kakao.sdk.user.UserApiClient
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.suspendCancellableCoroutine

actual class KakaoSingleSignOnProvider : SingleSignOnAdapter {
  context(activity: ActivityContext)
  actual override suspend fun authenticate(): SingleSignOnCredential {
    val accessToken = suspendCancellableCoroutine { continuation ->
      UserApiClient.instance.loginWithKakaoAccount(
        context = activity,
        prompts = listOf(Prompt.SELECT_ACCOUNT),
      ) { token, error ->
        if (error != null) {
          continuation.resumeWithException(error)
        } else if (token != null) {
          continuation.resume(token.accessToken)
        } else {
          continuation.resumeWithException(IllegalStateException("No token received"))
        }
      }
    }

    return SingleSignOnCredential(
      provider = SingleSignOnProvider.KAKAO,
      params = mapOf("access_token" to accessToken),
    )
  }
}
