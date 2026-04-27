package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.Dp

internal enum class EditorKeyboardType {
  Software,
  Hardware,
}

internal data class EditorKeyboardState(
  val type: EditorKeyboardType,
  val hardwareModeGeneration: Int = 0,
) {
  val hardwareKeyboardConnected: Boolean
    get() = type == EditorKeyboardType.Hardware
}

@Composable internal expect fun rememberEditorKeyboardState(): EditorKeyboardState

internal fun isSoftwareKeyboardVisible(imeBottom: Dp, safeBottomInset: Dp): Boolean =
  imeBottom > safeBottomInset
