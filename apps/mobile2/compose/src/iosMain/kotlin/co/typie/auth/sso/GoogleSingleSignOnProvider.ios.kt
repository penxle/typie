@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.auth.sso

import co.typie.graphql.type.SingleSignOnProvider
import co.typie.platform.ActivityContext
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.suspendCancellableCoroutine
import swiftPMImport.co.typie.compose.GoogleSingleSignOnBridge

actual class GoogleSingleSignOnProvider : SingleSignOnAdapter {
  context(activity: ActivityContext)
  actual override suspend fun authenticate(): SingleSignOnCredential {
    val code = suspendCancellableCoroutine { continuation ->
      GoogleSingleSignOnBridge().authenticateWithCompletion { code, error ->
        if (error != null) {
          continuation.resumeWithException(Exception(error.localizedDescription))
        } else if (code != null) {
          continuation.resume(code)
        } else {
          continuation.resumeWithException(IllegalStateException("No code received"))
        }
      }
    }

    return SingleSignOnCredential(
      provider = SingleSignOnProvider.GOOGLE,
      params = mapOf("code" to code),
    )
  }
}
