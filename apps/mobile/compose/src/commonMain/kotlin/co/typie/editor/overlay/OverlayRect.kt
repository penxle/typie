package co.typie.editor.overlay

import androidx.compose.foundation.layout.size
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.editor.ffi.Rect

internal fun Modifier.editorOverlayRect(rect: Rect): Modifier =
  graphicsLayer {
      translationX = rect.x.dp.toPx()
      translationY = rect.y.dp.toPx()
    }
    .size(width = Dp(rect.width.coerceAtLeast(1f)), height = Dp(rect.height))

internal fun Rect.scale(displayZoom: Float): Rect =
  Rect(
    x = x * displayZoom,
    y = y * displayZoom,
    width = width * displayZoom,
    height = height * displayZoom,
  )
