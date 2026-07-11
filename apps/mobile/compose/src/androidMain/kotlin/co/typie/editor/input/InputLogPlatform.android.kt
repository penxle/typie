package co.typie.editor.input

import android.provider.Settings
import co.typie.platform.PlatformModule

internal actual fun currentKeyboardId(): String? =
  Settings.Secure.getString(
    PlatformModule.context.contentResolver,
    Settings.Secure.DEFAULT_INPUT_METHOD,
  )
