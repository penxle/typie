package co.typie.ext

import co.typie.dev.DesktopDebugKeyboard

internal actual fun notifyTextInputFocusChanged(owner: Any, isFocused: Boolean) {
  DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused)
}
