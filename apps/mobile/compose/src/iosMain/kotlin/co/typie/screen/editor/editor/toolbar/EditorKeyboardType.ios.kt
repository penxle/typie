package co.typie.screen.editor.editor.toolbar

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

@Composable
internal actual fun rememberEditorKeyboardState(): EditorKeyboardState {
  var hardwareKeyboardConnected by remember { mutableStateOf(isEditorHardwareKeyboardConnected()) }
  var hardwareModeGeneration by remember { mutableStateOf(0) }

  fun syncKeyboardState() {
    val previous = hardwareKeyboardConnected
    val next = isEditorHardwareKeyboardConnected()
    if (!previous && next) {
      hardwareModeGeneration += 1
    }
    hardwareKeyboardConnected = next
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
      }
    val disconnectObserver =
      center.addObserverForName(
        name = GCKeyboardDidDisconnectNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        syncKeyboardState()
      }

    onDispose {
      center.removeObserver(connectObserver)
      center.removeObserver(disconnectObserver)
    }
  }

  return EditorKeyboardState(
    type =
      if (hardwareKeyboardConnected) {
        EditorKeyboardType.Hardware
      } else {
        EditorKeyboardType.Software
      },
    hardwareModeGeneration = hardwareModeGeneration,
  )
}

private fun isEditorHardwareKeyboardConnected(): Boolean = GCKeyboard.coalescedKeyboard != null
