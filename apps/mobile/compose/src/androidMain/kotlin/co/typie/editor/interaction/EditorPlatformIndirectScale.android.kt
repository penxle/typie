package co.typie.editor.interaction

import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.PointerKeyboardModifiers
import androidx.compose.ui.input.pointer.isCtrlPressed

internal actual fun Modifier.editorPlatformIndirectScale(
  bridge: EditorPlatformIndirectScaleBridge,
  enabled: Boolean,
  density: Float,
): Modifier = this

internal actual fun PointerKeyboardModifiers.isEditorIndirectZoomModifierPressed(): Boolean =
  isCtrlPressed
