package co.typie.editor.interaction.gestures

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.ImageBitmap

@Composable
internal expect fun rememberEditorSelectionHandleImages():
  Map<EditorSelectionHandleType, ImageBitmap>
