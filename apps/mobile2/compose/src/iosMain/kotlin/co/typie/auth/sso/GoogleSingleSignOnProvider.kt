@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.auth.sso

import co.typie.graphql.type.SingleSignOnProvider
import kotlinx.coroutines.suspendCancellableCoroutine
import swiftPMImport.co.typie.compose.GoogleSingleSignOnBridge
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class GoogleSingleSignOnProvider : SingleSignOnAdapter {
  override suspend fun authenticate(ctx: Any?): SingleSignOnCredential {
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
