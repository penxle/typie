package co.typie.domain.pushnotification

import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage

class AppMessagingService : FirebaseMessagingService() {
  override fun onNewToken(token: String) {
    FirebaseMessagingEvents.tokenRefresh.tryEmit(token)
  }

  override fun onMessageReceived(message: RemoteMessage) {
    FirebaseMessagingEvents.message.tryEmit(
      PushNotificationMessage(
        title = message.notification?.title,
        body = message.notification?.body,
        data = message.data,
      )
    )
  }
}
