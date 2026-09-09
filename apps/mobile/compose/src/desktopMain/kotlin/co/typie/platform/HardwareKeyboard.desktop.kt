package co.typie.platform

import androidx.compose.runtime.Composable
import co.typie.dev.DesktopDebugKeyboard

@Composable
internal actual fun rememberHardwareKeyboardConnected(): Boolean =
  DesktopDebugKeyboard.hardwareKeyboardConnected
