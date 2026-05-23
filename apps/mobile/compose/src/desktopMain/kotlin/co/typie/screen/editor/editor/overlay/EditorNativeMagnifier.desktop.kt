package co.typie.screen.editor.editor.overlay

import androidx.compose.ui.Modifier

internal actual val EditorNativeMagnifierAvailable: Boolean = false

internal actual fun Modifier.editorNativeMagnifier(placement: EditorMagnifierPlacement?): Modifier =
  this
