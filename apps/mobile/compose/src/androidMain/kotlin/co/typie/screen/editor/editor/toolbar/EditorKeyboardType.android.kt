// cspell:ignore NOKEYS

package co.typie.screen.editor.editor.toolbar

import android.content.Context
import android.content.res.Configuration
import android.hardware.input.InputManager
import android.view.InputDevice
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.imeAnimationSource
import androidx.compose.foundation.layout.imeAnimationTarget
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal actual fun rememberEditorKeyboardState(
  isEditorInputSessionActive: () -> Boolean
): EditorKeyboardState {
  val configuration = LocalConfiguration.current
  val context = LocalContext.current
  val density = LocalDensity.current
  val imeBottom = WindowInsets.ime.getBottom(density)
  val imeAnimationSourceBottom = WindowInsets.imeAnimationSource.getBottom(density)
  val imeAnimationTargetBottom = WindowInsets.imeAnimationTarget.getBottom(density)
  val imeBottomDp = with(density) { imeBottom.toDp() }
  val imeAnimationSourceBottomDp = with(density) { imeAnimationSourceBottom.toDp() }
  val imeAnimationTargetBottomDp = with(density) { imeAnimationTargetBottom.toDp() }
  val presentation =
    resolveKeyboardPresentation(
      imeBottom = imeBottomDp,
      animationSourceBottom = imeAnimationSourceBottomDp,
      animationTargetBottom = imeAnimationTargetBottomDp,
    )
  val imeHideOwnershipTracker = remember { EditorImeHideOwnershipTracker() }
  val imeHideEventOwner =
    imeHideOwnershipTracker.observe(
      presentation = presentation,
      editorInputSessionActive = isEditorInputSessionActive(),
    )
  val hardwareKeyboardVisible =
    configuration.keyboard != Configuration.KEYBOARD_NOKEYS &&
      configuration.hardKeyboardHidden != Configuration.HARDKEYBOARDHIDDEN_YES
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
  return EditorKeyboardState(
    type =
      if (hardwareKeyboardVisible) {
        EditorKeyboardType.Hardware
      } else {
        EditorKeyboardType.Software
      },
    imeFrameVisible = imeBottom > 0 || imeAnimationTargetBottom > 0,
    imeHideEventOwner = imeHideEventOwner,
    presentation = presentation,
    hardwareKeyboardAttached = hardwareKeyboardVisible || externalKeyboardAttached,
  )
}

private fun isExternalKeyboardAttached(): Boolean =
  InputDevice.getDeviceIds().any { deviceId ->
    val device = InputDevice.getDevice(deviceId)
    device != null &&
      !device.isVirtual &&
      device.supportsSource(InputDevice.SOURCE_KEYBOARD) &&
      device.keyboardType == InputDevice.KEYBOARD_TYPE_ALPHABETIC
  }
