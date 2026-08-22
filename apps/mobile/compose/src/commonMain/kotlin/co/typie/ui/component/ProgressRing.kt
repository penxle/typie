package co.typie.ui.component

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Ease
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.domain.goal.GoalColorState
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.ResolvedThemeMode

@Composable
fun ProgressRing(
  progress: Float,
  modifier: Modifier = Modifier,
  state: GoalColorState = GoalColorState.Under,
  pie: Float? = null,
  pieWarning: Boolean = false,
  size: Dp = 16.dp,
) {
  val trackColor = progressTrackColor()
  val fillColor = progressFillColor(state)
  val fillFraction = progressFillFraction(progress, state)

  val pieNeutralColor =
    if (AppTheme.themeMode == ResolvedThemeMode.Dark) AppColor.dark.gray.s700
    else AppColor.light.gray.s300
  val pieColor by
    animateColorAsState(
      targetValue = if (pieWarning) AppTheme.colors.danger else pieNeutralColor,
      animationSpec = tween(PROGRESS_TRANSITION_MILLIS, easing = Ease),
    )
  val pieFraction =
    if (pie == null) 0f
    else
      animateFloatAsState(
          targetValue = pie.coerceIn(0f, 1f),
          animationSpec = tween(PROGRESS_TRANSITION_MILLIS, easing = Ease),
        )
        .value

  Canvas(modifier.size(size)) {
    val scale = this.size.minDimension / VIEW_BOX_SIZE
    val ringRect = Rect(center = center, radius = RING_RADIUS * scale)
    val ringStrokeWidth = RING_STROKE_WIDTH * scale

    drawArc(
      color = trackColor,
      startAngle = START_ANGLE,
      sweepAngle = FULL_SWEEP,
      useCenter = false,
      topLeft = ringRect.topLeft,
      size = ringRect.size,
      style = Stroke(width = ringStrokeWidth),
    )

    if (fillFraction > 0f) {
      drawArc(
        color = fillColor,
        startAngle = START_ANGLE,
        sweepAngle = fillFraction * FULL_SWEEP,
        useCenter = false,
        topLeft = ringRect.topLeft,
        size = ringRect.size,
        style = Stroke(width = ringStrokeWidth, cap = StrokeCap.Round),
      )
    }

    if (pie != null) {
      val pieRect = Rect(center = center, radius = PIE_RADIUS * scale)

      drawArc(
        color = pieColor,
        startAngle = START_ANGLE,
        sweepAngle = pieFraction * FULL_SWEEP,
        useCenter = false,
        topLeft = pieRect.topLeft,
        size = pieRect.size,
        style = Stroke(width = PIE_STROKE_WIDTH * scale),
      )
    }
  }
}

@Composable
internal fun progressTrackColor(): Color =
  if (AppTheme.themeMode == ResolvedThemeMode.Dark) AppColor.dark.gray.s800
  else AppColor.light.gray.s200

@Composable
internal fun progressFillColor(state: GoalColorState): Color {
  val targetColor =
    when (state) {
      GoalColorState.Under ->
        if (AppTheme.themeMode == ResolvedThemeMode.Dark) AppColor.dark.gray.s400
        else AppColor.light.gray.s500
      GoalColorState.Achieved -> AppTheme.colors.success
      GoalColorState.Over -> AppTheme.colors.warning
      GoalColorState.Excess -> AppTheme.colors.danger
    }

  val color by
    animateColorAsState(
      targetValue = targetColor,
      animationSpec = tween(PROGRESS_TRANSITION_MILLIS, easing = Ease),
    )

  return color
}

@Composable
internal fun progressFillFraction(progress: Float, state: GoalColorState): Float {
  val targetFraction = if (state == GoalColorState.Under) progress.coerceIn(0f, 1f) else 1f
  val fraction by
    animateFloatAsState(
      targetValue = targetFraction,
      animationSpec = tween(PROGRESS_TRANSITION_MILLIS, easing = Ease),
    )

  return fraction
}

private const val PROGRESS_TRANSITION_MILLIS = 300
private const val VIEW_BOX_SIZE = 32f
private const val RING_RADIUS = 13f
private const val RING_STROKE_WIDTH = 3.5f
private const val PIE_RADIUS = 4.5f
private const val PIE_STROKE_WIDTH = 9f
private const val START_ANGLE = -90f
private const val FULL_SWEEP = 360f
