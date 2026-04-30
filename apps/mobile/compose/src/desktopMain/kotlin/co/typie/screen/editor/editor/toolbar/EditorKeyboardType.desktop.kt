package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp
import co.typie.dev.DesktopDebugKeyboard

@Composable
internal actual fun rememberEditorKeyboardState(): EditorKeyboardState =
  EditorKeyboardState(
    type =
      if (DesktopDebugKeyboard.hardwareKeyboardConnected) {
        EditorKeyboardType.Hardware
      } else {
        EditorKeyboardType.Software
      },
    presentation = EditorKeyboardPresentation.Shown(0.dp),
  )
