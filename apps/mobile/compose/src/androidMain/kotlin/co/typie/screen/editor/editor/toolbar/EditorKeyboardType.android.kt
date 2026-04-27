// cspell:ignore NOKEYS

package co.typie.screen.editor.editor.toolbar

import android.content.res.Configuration
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalConfiguration

@Composable
internal actual fun rememberEditorKeyboardType(): EditorKeyboardType {
  val configuration = LocalConfiguration.current
  val hardwareKeyboardVisible =
    configuration.keyboard != Configuration.KEYBOARD_NOKEYS &&
      configuration.hardKeyboardHidden != Configuration.HARDKEYBOARDHIDDEN_YES
  return if (hardwareKeyboardVisible) EditorKeyboardType.Hardware else EditorKeyboardType.Software
}
