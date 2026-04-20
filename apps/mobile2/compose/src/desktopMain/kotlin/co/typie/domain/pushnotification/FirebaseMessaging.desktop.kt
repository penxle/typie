package co.typie.domain.pushnotification

import kotlinx.coroutines.flow.Flow

actual object FirebaseMessaging {
  actual suspend fun requestPermission(): Boolean = false

  actual suspend fun token(): String? = null

  actual suspend fun deleteToken() {}

  actual val onMessage: Flow<PushNotificationMessage> = FirebaseMessagingEvents.message

  actual val onTokenRefresh: Flow<String> = FirebaseMessagingEvents.tokenRefresh
}
