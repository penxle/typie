package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.systemGestureExclusion
import androidx.compose.ui.Modifier

internal actual fun Modifier.editorSelectionHandleGestureExclusion(): Modifier =
  systemGestureExclusion()
