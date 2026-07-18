package co.typie.editor.interaction

import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.PointerKeyboardModifiers
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isMetaPressed

internal actual fun Modifier.editorPlatformIndirectScale(
  bridge: EditorPlatformIndirectScaleBridge,
  enabled: Boolean,
  density: Float,
): Modifier = this

internal actual fun PointerKeyboardModifiers.isEditorIndirectZoomModifierPressed(): Boolean =
  if (isMacOs()) isMetaPressed else isCtrlPressed

private fun isMacOs(): Boolean = System.getProperty("os.name").startsWith("Mac", ignoreCase = true)
