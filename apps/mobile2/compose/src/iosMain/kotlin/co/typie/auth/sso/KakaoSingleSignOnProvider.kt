@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.auth.sso

import co.typie.graphql.type.SingleSignOnProvider
import kotlinx.coroutines.suspendCancellableCoroutine
import swiftPMImport.co.typie.compose.KakaoSingleSignOnBridge
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class KakaoSingleSignOnProvider : SingleSignOnAdapter {
  actual override suspend fun authenticate(ctx: Any?): SingleSignOnCredential {
    val accessToken = suspendCancellableCoroutine { continuation ->
      KakaoSingleSignOnBridge().authenticateWithCompletion { accessToken, error ->
        if (error != null) {
          continuation.resumeWithException(Exception(error.localizedDescription))
        } else if (accessToken != null) {
          continuation.resume(accessToken)
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
