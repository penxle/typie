@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import platform.Foundation.NSNotificationCenter
import platform.Foundation.NSOperationQueue
import platform.GameController.GCKeyboardDidConnectNotification
import platform.GameController.GCKeyboardDidDisconnectNotification
import platform.UIKit.UIKeyboardWillChangeFrameNotification
import platform.UIKit.UIKeyboardWillHideNotification
import swiftPMImport.co.typie.compose.EditorKeyboardBridge

@Composable
internal actual fun rememberEditorKeyboardState(): EditorKeyboardState {
  var hardwareKeyboardMode by remember { mutableStateOf(isEditorHardwareKeyboardConnected()) }
  var imeFrameVisible by remember { mutableStateOf(false) }
  var imeHideEventVersion by remember { mutableIntStateOf(0) }

  fun syncKeyboardState() {
    hardwareKeyboardMode = isEditorHardwareKeyboardConnected()
  }

  fun syncKeyboardFrame(notification: platform.Foundation.NSNotification?) {
    imeFrameVisible =
      notification?.let(EditorKeyboardBridge::isImeFrameVisibleWithNotification) ?: false
    syncKeyboardState()
  }

  DisposableEffect(Unit) {
    val center = NSNotificationCenter.defaultCenter
    val connectObserver =
      center.addObserverForName(
        name = GCKeyboardDidConnectNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        syncKeyboardState()
        if (hardwareKeyboardMode) {
          imeFrameVisible = false
        }
      }
    val disconnectObserver =
      center.addObserverForName(
        name = GCKeyboardDidDisconnectNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        syncKeyboardState()
        if (hardwareKeyboardMode) {
          imeFrameVisible = false
        }
      }
    val frameObserver =
      center.addObserverForName(
        name = UIKeyboardWillChangeFrameNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        syncKeyboardFrame(it)
      }
    val hideObserver =
      center.addObserverForName(
        name = UIKeyboardWillHideNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        imeFrameVisible = false
        imeHideEventVersion++
        syncKeyboardState()
      }

    onDispose {
      center.removeObserver(connectObserver)
      center.removeObserver(disconnectObserver)
      center.removeObserver(frameObserver)
      center.removeObserver(hideObserver)
    }
  }

  return EditorKeyboardState(
    type =
      if (hardwareKeyboardMode) {
        EditorKeyboardType.Hardware
      } else {
        EditorKeyboardType.Software
      },
    imeFrameVisible = imeFrameVisible,
    imeHideEventVersion = imeHideEventVersion,
  )
}

private fun isEditorHardwareKeyboardConnected(): Boolean =
  EditorKeyboardBridge.isInHardwareKeyboardMode()
