package co.typie.platform

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.platform.PlatformTextInputInterceptor
import androidx.compose.ui.text.input.KeyboardType

object KeyboardExtensionPolicy {
  private var activeDecimalInputSessions = 0

  val allowsExtensions: Boolean
    get() = activeDecimalInputSessions == 0

  @OptIn(ExperimentalComposeUiApi::class)
  internal val interceptor = PlatformTextInputInterceptor { request, nextHandler ->
    // UIKit can crash while resolving decimalPad input modes for a keyboard extension with a
    // hardware keyboard. Restrict extensions before UIKit starts input and restore them on
    // teardown.
    val requiresSystemKeyboard = request.imeOptions.keyboardType == KeyboardType.Decimal
    if (requiresSystemKeyboard) activeDecimalInputSessions++
    try {
      nextHandler.startInputMethod(request)
    } finally {
      if (requiresSystemKeyboard) activeDecimalInputSessions--
    }
  }
}
