package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import platform.Foundation.NSNotificationCenter
import platform.GameController.GCKeyboard
import platform.GameController.GCKeyboardDidConnectNotification
import platform.GameController.GCKeyboardDidDisconnectNotification

@Composable
internal actual fun rememberEditorKeyboardType(): EditorKeyboardType {
  var keyboardType by remember { mutableStateOf(resolveEditorKeyboardType()) }

  DisposableEffect(Unit) {
    val center = NSNotificationCenter.defaultCenter
    val connectObserver =
      center.addObserverForName(
        name = GCKeyboardDidConnectNotification,
        `object` = null,
        queue = null,
      ) {
        keyboardType = resolveEditorKeyboardType()
      }
    val disconnectObserver =
      center.addObserverForName(
        name = GCKeyboardDidDisconnectNotification,
        `object` = null,
        queue = null,
      ) {
        keyboardType = resolveEditorKeyboardType()
      }

    onDispose {
      center.removeObserver(connectObserver)
      center.removeObserver(disconnectObserver)
    }
  }

  return keyboardType
}

private fun resolveEditorKeyboardType(): EditorKeyboardType =
  if (GCKeyboard.coalescedKeyboard != null) {
    EditorKeyboardType.Hardware
  } else {
    EditorKeyboardType.Software
  }
