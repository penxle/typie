package co.typie.screen.editor.editor.overlay

import android.os.Build
import androidx.compose.foundation.magnifier
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.DpSize

internal actual val EditorNativeMagnifierAvailable: Boolean
  get() = Build.VERSION.SDK_INT >= 28

internal actual fun Modifier.editorNativeMagnifier(placement: EditorMagnifierPlacement?): Modifier {
  if (placement == null || !EditorNativeMagnifierAvailable) {
    return this
  }

  return magnifier(
    sourceCenter = { placement.sourceCenter },
    magnifierCenter = { placement.magnifierCenter },
    zoom = EditorMagnifierZoom,
    size = DpSize(width = EditorMagnifierWidth, height = EditorMagnifierHeight),
    cornerRadius = EditorMagnifierHeight / 2,
    clip = true,
  )
}
