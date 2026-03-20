package co.typie.auth.sso

import android.app.Activity
import co.typie.graphql.type.SingleSignOnProvider
import com.kakao.sdk.auth.model.Prompt
import com.kakao.sdk.user.UserApiClient
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class KakaoSingleSignOnProvider : SingleSignOnAdapter {

  override suspend fun authenticate(ctx: Any?): SingleSignOnCredential {
    val context = ctx as Activity
    val accessToken = suspendCancellableCoroutine { continuation ->
      UserApiClient.instance.loginWithKakaoAccount(
        context = context,
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
