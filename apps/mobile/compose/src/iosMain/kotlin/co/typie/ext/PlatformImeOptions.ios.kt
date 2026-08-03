package co.typie.ext

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.text.input.PlatformImeOptions

@OptIn(ExperimentalComposeUiApi::class)
internal actual fun nativeTextInputPlatformImeOptions(): PlatformImeOptions? = PlatformImeOptions {
  usingNativeTextInput(true)
}
