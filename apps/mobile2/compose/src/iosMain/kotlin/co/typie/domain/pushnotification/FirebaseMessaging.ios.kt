@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.domain.pushnotification

import kotlin.coroutines.resume
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext

actual object FirebaseMessaging {
  actual val onMessage: Flow<PushNotificationMessage> =
    FirebaseMessagingEvents.message.asSharedFlow()

  actual val onTokenRefresh: Flow<String> = FirebaseMessagingEvents.tokenRefresh.asSharedFlow()

  actual suspend fun requestPermission(): Boolean =
    withContext(Dispatchers.Main) {
      val bridge = IOSPushBridgeHolder.bridge ?: return@withContext false
      suspendCancellableCoroutine { cont ->
        bridge.requestAuthorizationWithCompletion { granted -> cont.resume(granted) }
      }
    }

  actual suspend fun token(): String? =
    withContext(Dispatchers.Main) {
      val bridge = IOSPushBridgeHolder.bridge ?: return@withContext null
      suspendCancellableCoroutine { cont ->
        bridge.fetchTokenWithCompletion { token -> cont.resume(token) }
      }
    }

  actual suspend fun deleteToken() {
    withContext(Dispatchers.Main) {
      val bridge = IOSPushBridgeHolder.bridge ?: return@withContext
      suspendCancellableCoroutine<Unit> { cont ->
        bridge.deleteTokenWithCompletion { cont.resume(Unit) }
      }
    }
  }
}
