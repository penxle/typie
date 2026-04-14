package co.typie.domain.auth.sso

import co.typie.graphql.type.SingleSignOnProvider
import co.typie.platform.ActivityContext
import com.navercorp.nid.NidOAuth
import com.navercorp.nid.oauth.util.NidOAuthCallback
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.suspendCancellableCoroutine

actual class NaverSingleSignOnProvider : SingleSignOnAdapter {
  context(activity: ActivityContext)
  actual override suspend fun authenticate(): SingleSignOnCredential {
    NidOAuth.logout(
      object : NidOAuthCallback {
        override fun onSuccess() {}

        override fun onFailure(errorCode: String, errorDesc: String) {}
      }
    )

    val accessToken = suspendCancellableCoroutine { continuation ->
      NidOAuth.requestLogin(
        activity,
        object : NidOAuthCallback {
          override fun onSuccess() {
            val accessToken = NidOAuth.getAccessToken()
            if (accessToken != null) {
              continuation.resume(accessToken)
            } else {
              continuation.resumeWithException(IllegalStateException("No token received"))
            }
          }

          override fun onFailure(errorCode: String, errorDesc: String) {
            continuation.resumeWithException(RuntimeException(errorDesc))
          }
        },
      )
    }

    return SingleSignOnCredential(
      provider = SingleSignOnProvider.NAVER,
      params = mapOf("access_token" to accessToken),
    )
  }
}
