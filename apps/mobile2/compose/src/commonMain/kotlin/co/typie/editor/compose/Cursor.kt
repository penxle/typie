package co.typie.editor.compose

import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import kotlinx.coroutines.delay

@Composable
internal fun Cursor(
  offset: Offset,
  size: Size,
) {
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
    Modifier
      .offset(x = Dp(offset.x), y = Dp(offset.y))
      .size(
        width = Dp(size.width.coerceAtLeast(1f)),
        height = Dp(size.height),
      )
      .background(Color.Black.copy(alpha = alpha.value))
  )
}
