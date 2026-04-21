package co.typie.editor.overlay

import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

@Composable
internal fun EditorCursorOverlay(offset: Offset, size: Size) {
  val alpha = remember { Animatable(1f) }

  LaunchedEffect(offset) {
    alpha.snapTo(1f)
    while (true) {
      delay(500)
      alpha.snapTo(0f)
      delay(500)
      alpha.snapTo(1f)
    }
  }

  Box(
    Modifier.graphicsLayer {
        translationX = offset.x.dp.toPx()
        translationY = offset.y.dp.toPx()
        this.alpha = alpha.value
      }
      .size(width = Dp(size.width.coerceAtLeast(1f)), height = Dp(size.height))
      .background(Color.Black)
  )
}
