package co.typie.domain.pushnotification

import kotlinx.coroutines.flow.Flow

expect object FirebaseMessaging {
  suspend fun requestPermission(): Boolean

  suspend fun token(): String?

  suspend fun deleteToken()

  val onMessage: Flow<PushNotificationMessage>

  val onTokenRefresh: Flow<String>
}
