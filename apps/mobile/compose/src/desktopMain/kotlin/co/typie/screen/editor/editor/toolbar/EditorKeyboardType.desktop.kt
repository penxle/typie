package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import co.typie.dev.DesktopDebugKeyboard

@Composable
internal actual fun rememberEditorKeyboardState(): EditorKeyboardState =
  EditorKeyboardState(
    type =
      if (DesktopDebugKeyboard.hardwareKeyboardConnected) {
        EditorKeyboardType.Hardware
      } else {
        EditorKeyboardType.Software
      }
  )
