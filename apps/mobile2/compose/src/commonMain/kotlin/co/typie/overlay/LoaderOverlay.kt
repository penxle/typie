package co.typie.overlay

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseIn
import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.AppTheme

@Composable
fun LoaderOverlay() {
  val loader = LocalLoader.current
  val loading = loader.loading
  var visible by remember { mutableStateOf(false) }
  val alpha = remember { Animatable(0f) }

  LaunchedEffect(loading) {
    if (loading) {
      visible = true
      alpha.animateTo(1f, tween(200, easing = EaseOut))
    } else if (visible) {
      alpha.animateTo(0f, tween(200, easing = EaseIn))
      visible = false
    }
  }

  if (visible) {
    Box(
      modifier =
        Modifier.fillMaxSize()
          .alpha(alpha.value)
          .pointerInput(Unit) {}
          .background(AppColor.black.copy(alpha = 0.3f)),
      contentAlignment = Alignment.Center,
    ) {
      Spinner(color = AppTheme.colors.textPrimary)
    }
  }
}

@Composable
private fun Spinner(color: Color, modifier: Modifier = Modifier) {
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

  Canvas(modifier.size(28.dp)) {
    drawArc(
      color = color,
      startAngle = rotation,
      sweepAngle = 270f,
      useCenter = false,
      style = Stroke(width = 2.dp.toPx(), cap = StrokeCap.Round),
    )
  }
}
