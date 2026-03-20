@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.auth.sso

import co.typie.graphql.type.SingleSignOnProvider
import kotlinx.coroutines.suspendCancellableCoroutine
import swiftPMImport.co.typie.compose.NaverSingleSignOnBridge
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class NaverSingleSignOnProvider : SingleSignOnAdapter {
  override suspend fun authenticate(ctx: Any?): SingleSignOnCredential {
    val accessToken = suspendCancellableCoroutine { continuation ->
      NaverSingleSignOnBridge().authenticateWithCompletion { accessToken, error ->
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
      provider = SingleSignOnProvider.NAVER,
      params = mapOf("access_token" to accessToken),
    )
  }
}
