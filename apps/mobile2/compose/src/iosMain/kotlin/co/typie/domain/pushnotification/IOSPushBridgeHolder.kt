@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.domain.pushnotification

import swiftPMImport.co.typie.compose.PushNotificationBridge

internal object IOSPushBridgeHolder {
  var bridge: PushNotificationBridge? = null
}
