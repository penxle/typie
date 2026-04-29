package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.Dp

internal enum class EditorKeyboardType {
  Software,
  Hardware,
}

internal data class EditorKeyboardState(
  val type: EditorKeyboardType,
  val imeFrameVisible: Boolean = false,
  val imeHideEventVersion: Int = 0,
) {
  val usesImeInset: Boolean
    get() = type == EditorKeyboardType.Software || imeFrameVisible
}

@Composable internal expect fun rememberEditorKeyboardState(): EditorKeyboardState

internal fun isImeVisible(imeBottom: Dp, safeBottomInset: Dp): Boolean = imeBottom > safeBottomInset
