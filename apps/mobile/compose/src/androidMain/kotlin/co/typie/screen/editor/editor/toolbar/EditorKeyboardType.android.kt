// cspell:ignore NOKEYS

package co.typie.screen.editor.editor.toolbar

import android.content.res.Configuration
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.ime
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity

@Composable
internal actual fun rememberEditorKeyboardState(): EditorKeyboardState {
  val configuration = LocalConfiguration.current
  val density = LocalDensity.current
  val hardwareKeyboardVisible =
    configuration.keyboard != Configuration.KEYBOARD_NOKEYS &&
      configuration.hardKeyboardHidden != Configuration.HARDKEYBOARDHIDDEN_YES
  return EditorKeyboardState(
    type =
      if (hardwareKeyboardVisible) {
        EditorKeyboardType.Hardware
      } else {
        EditorKeyboardType.Software
      },
    imeFrameVisible = WindowInsets.ime.getBottom(density) > 0,
  )
}
