@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.auth.sso

import co.typie.graphql.type.SingleSignOnProvider
import kotlinx.coroutines.suspendCancellableCoroutine
import swiftPMImport.co.typie.compose.AppleSingleSignOnBridge
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class AppleSingleSignOnProvider : SingleSignOnAdapter {
  override suspend fun authenticate(ctx: Any?): SingleSignOnCredential {
    val code = suspendCancellableCoroutine { continuation ->
      AppleSingleSignOnBridge().authenticateWithCompletion { code, error ->
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
      provider = SingleSignOnProvider.APPLE,
      params = mapOf("code" to code),
    )
  }
}
