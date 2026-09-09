// cspell:ignore NOKEYS
package co.typie.platform

import android.content.Context
import android.content.res.Configuration
import android.hardware.input.InputManager
import android.view.InputDevice
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext

@Composable
internal actual fun rememberHardwareKeyboardConnected(): Boolean {
  val external = rememberExternalKeyboardAttached()
  return external || LocalConfiguration.current.keyboard != Configuration.KEYBOARD_NOKEYS
}

@Composable
internal fun rememberExternalKeyboardAttached(): Boolean {
  val context = LocalContext.current
  // Configuration lags (or on some OEMs never reflects) external keyboard attach, so the
  // attached signal is tracked separately via InputDevice events.
  var externalKeyboardAttached by remember { mutableStateOf(isExternalKeyboardAttached()) }
  DisposableEffect(context) {
    val inputManager = context.getSystemService(Context.INPUT_SERVICE) as InputManager
    val listener =
      object : InputManager.InputDeviceListener {
        override fun onInputDeviceAdded(deviceId: Int) {
          externalKeyboardAttached = isExternalKeyboardAttached()
        }

        override fun onInputDeviceRemoved(deviceId: Int) {
          externalKeyboardAttached = isExternalKeyboardAttached()
        }

        override fun onInputDeviceChanged(deviceId: Int) {
          externalKeyboardAttached = isExternalKeyboardAttached()
        }
      }
    inputManager.registerInputDeviceListener(listener, null)
    externalKeyboardAttached = isExternalKeyboardAttached()
    onDispose { inputManager.unregisterInputDeviceListener(listener) }
  }
  return externalKeyboardAttached
}

private fun isExternalKeyboardAttached(): Boolean =
  InputDevice.getDeviceIds().any { deviceId ->
    val device = InputDevice.getDevice(deviceId)
    device != null &&
      !device.isVirtual &&
      device.supportsSource(InputDevice.SOURCE_KEYBOARD) &&
      device.keyboardType == InputDevice.KEYBOARD_TYPE_ALPHABETIC
  }
