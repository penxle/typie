@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.auth.sso

import co.typie.graphql.type.SingleSignOnProvider
import co.typie.platform.ActivityContext
import kotlinx.coroutines.suspendCancellableCoroutine
import swiftPMImport.co.typie.compose.NaverSingleSignOnBridge
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class NaverSingleSignOnProvider : SingleSignOnAdapter {
  context(_: ActivityContext)
  actual override suspend fun authenticate(): SingleSignOnCredential {
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
