// cspell:ignore NOKEYS

package co.typie.screen.editor.editor.toolbar

import android.content.res.Configuration
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalConfiguration

@Composable
internal actual fun rememberEditorKeyboardState(): EditorKeyboardState {
  val configuration = LocalConfiguration.current
  val hardwareKeyboardVisible =
    configuration.keyboard != Configuration.KEYBOARD_NOKEYS &&
      configuration.hardKeyboardHidden != Configuration.HARDKEYBOARDHIDDEN_YES
  return EditorKeyboardState(
    type =
      if (hardwareKeyboardVisible) {
        EditorKeyboardType.Hardware
      } else {
        EditorKeyboardType.Software
      }
  )
}
