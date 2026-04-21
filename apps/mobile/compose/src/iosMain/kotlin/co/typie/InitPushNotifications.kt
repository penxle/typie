@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie

import co.typie.domain.pushnotification.FirebaseMessagingEvents
import co.typie.domain.pushnotification.IOSPushBridgeHolder
import co.typie.domain.pushnotification.PushNotificationMessage
import platform.UIKit.UIApplication
import swiftPMImport.co.typie.compose.PushNotificationBridge

fun doInitPushNotifications(application: UIApplication) {
  val bridge = PushNotificationBridge()

  bridge.onToken = { token ->
    if (token != null) {
      FirebaseMessagingEvents.tokenRefresh.tryEmit(token)
    }
  }

  bridge.onMessage = { payload ->
    if (payload != null) {
      val rawData = payload.data()
      val data = buildMap {
        for ((key, value) in rawData) {
          if (key is String && value is String) {
            put(key, value)
          }
        }
      }
      FirebaseMessagingEvents.message.tryEmit(
        PushNotificationMessage(title = payload.title(), body = payload.body(), data = data)
      )
    }
  }

  bridge.attachTo(application)
  IOSPushBridgeHolder.bridge = bridge
}
