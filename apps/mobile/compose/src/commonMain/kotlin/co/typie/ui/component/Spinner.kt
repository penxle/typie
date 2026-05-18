package co.typie.ui.component

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

@Composable
fun Spinner(
  color: Color,
  modifier: Modifier = Modifier,
  size: Dp = 16.dp,
  strokeWidth: Dp = 1.5.dp,
  sweepAngle: Float = 220f,
) {
  val transition = rememberInfiniteTransition()
  val rotation by
    transition.animateFloat(
      initialValue = 0f,
      targetValue = 360f,
      animationSpec =
        infiniteRepeatable(
          animation = tween(1000, easing = LinearEasing),
          repeatMode = RepeatMode.Restart,
        ),
    )

  Canvas(modifier.size(size)) {
    drawArc(
      color = color,
      startAngle = rotation,
      sweepAngle = sweepAngle,
      useCenter = false,
      style = Stroke(width = strokeWidth.toPx(), cap = StrokeCap.Round),
    )
  }
}
