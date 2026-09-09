package co.typie.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import platform.Foundation.NSNotificationCenter
import platform.Foundation.NSOperationQueue
import platform.GameController.GCKeyboard
import platform.GameController.GCKeyboardDidConnectNotification
import platform.GameController.GCKeyboardDidDisconnectNotification
import platform.UIKit.UIApplicationDidBecomeActiveNotification

@Composable
internal actual fun rememberHardwareKeyboardConnected(): Boolean {
  var connected by remember { mutableStateOf(GCKeyboard.coalescedKeyboard != null) }
  DisposableEffect(Unit) {
    val center = NSNotificationCenter.defaultCenter
    val observers =
      listOf(
          GCKeyboardDidConnectNotification,
          GCKeyboardDidDisconnectNotification,
          UIApplicationDidBecomeActiveNotification,
        )
        .map { name ->
          center.addObserverForName(name, null, NSOperationQueue.mainQueue) {
            connected = GCKeyboard.coalescedKeyboard != null
          }
        }
    connected = GCKeyboard.coalescedKeyboard != null
    onDispose { observers.forEach(center::removeObserver) }
  }
  return connected
}
