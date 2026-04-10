@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.auth.sso

import co.typie.graphql.type.SingleSignOnProvider
import co.typie.platform.ActivityContext
import kotlinx.coroutines.suspendCancellableCoroutine
import swiftPMImport.co.typie.compose.AppleSingleSignOnBridge
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class AppleSingleSignOnProvider : SingleSignOnAdapter {
  context(_: ActivityContext)
  actual override suspend fun authenticate(): SingleSignOnCredential {
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
