package co.typie.auth.sso

import android.app.Activity
import co.typie.graphql.type.SingleSignOnProvider
import com.navercorp.nid.NidOAuth
import com.navercorp.nid.oauth.util.NidOAuthCallback
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class NaverSingleSignOnProvider : SingleSignOnAdapter {

  actual override suspend fun authenticate(ctx: Any?): SingleSignOnCredential {
    val context = ctx as Activity

    NidOAuth.logout(object : NidOAuthCallback {
      override fun onSuccess() {
      }

      override fun onFailure(errorCode: String, errorDesc: String) {
      }
    })

    val accessToken = suspendCancellableCoroutine { continuation ->
      NidOAuth.requestLogin(context, object : NidOAuthCallback {
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
      })
    }

    return SingleSignOnCredential(
      provider = SingleSignOnProvider.NAVER,
      params = mapOf("access_token" to accessToken),
    )
  }
}
