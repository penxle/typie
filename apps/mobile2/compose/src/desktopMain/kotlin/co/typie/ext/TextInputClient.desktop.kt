package co.typie.ext

import co.typie.dev.DesktopDebugKeyboard

internal actual fun registerTextInputClient(owner: Any, client: TextInputClient?) {
  DesktopDebugKeyboard.registerClient(owner, client)
}
