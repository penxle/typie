package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.times
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow
import kotlin.math.abs
import kotlin.math.roundToInt

private const val MAX_REGULAR_HAPTIC_INTERVALS = 8

internal fun sliderHapticFeedbackType(
  range: ClosedFloatingPointRange<Float>,
  step: Float?,
  value: Float,
): HapticFeedbackType? {
  val normalizedStep = step?.takeIf { it > 0f }
  if (normalizedStep == null) {
    return if (value == range.start || value == range.endInclusive) HapticFeedbackType.GestureEnd
    else null
  }

  val intervals =
    ((range.endInclusive - range.start).coerceAtLeast(0f) / normalizedStep).roundToInt()
  return if (intervals <= MAX_REGULAR_HAPTIC_INTERVALS) HapticFeedbackType.SegmentTick
  else HapticFeedbackType.SegmentFrequentTick
}

internal class SliderGestureSession(
  initialValue: Float,
  private val valueFromX: (Float) -> Float,
  private val onDragStart: () -> Unit,
  private val onDrag: (Float) -> Unit,
) {
  private var current = initialValue
  private var started = false

  fun start() {
    if (started) {
      return
    }
    started = true
    onDragStart()
  }

  fun updateAt(x: Float) {
    val next = valueFromX(x)
    if (next == current) {
      return
    }
    current = next
    onDrag(next)
  }

  fun release(): Float? {
    if (!started) {
      return null
    }
    started = false
    return current
  }

  fun cancel() {
    started = false
  }
}

@Composable
internal fun Slider(
  value: Float,
  range: ClosedFloatingPointRange<Float>,
  onDragStart: () -> Unit,
  onDrag: (Float) -> Unit,
  onDragEnd: (Float) -> Unit,
  modifier: Modifier = Modifier,
  step: Float? = null,
  onDragCancel: () -> Unit = {},
  thumbSize: Dp = 24.dp,
  trackHeight: Dp = 8.dp,
  trackColor: Color = AppTheme.colors.borderEmphasis.copy(alpha = 0.5f),
  fillColor: Color = AppTheme.colors.textDefault,
  thumbContent: @Composable BoxScope.(Boolean) -> Unit = {},
) {
  val density = LocalDensity.current
  val hapticFeedback = LocalHapticFeedback.current
  val currentValue by rememberUpdatedState(value)
  val currentHapticFeedback by rememberUpdatedState(hapticFeedback)
  val currentOnDragStart by rememberUpdatedState(onDragStart)
  val currentOnDrag by rememberUpdatedState(onDrag)
  val currentOnDragEnd by rememberUpdatedState(onDragEnd)
  val currentOnDragCancel by rememberUpdatedState(onDragCancel)
  val normalizedStep = step?.takeIf { it > 0f }
  val inRange = value in range
  val rangeSpan = (range.endInclusive - range.start).coerceAtLeast(0.0001f)

  BoxWithConstraints(modifier = modifier, contentAlignment = Alignment.CenterStart) {
    val travel = (maxWidth - thumbSize).coerceAtLeast(0.dp)
    val travelPx = with(density) { travel.toPx() }
    val thumbRadiusPx = with(density) { (thumbSize / 2).toPx() }

    fun fractionOf(v: Float): Float = ((v - range.start) / rangeSpan).coerceIn(0f, 1f)

    fun valueFromX(x: Float): Float {
      if (travelPx <= 0f) {
        return currentValue
      }
      val fraction = ((x - thumbRadiusPx) / travelPx).coerceIn(0f, 1f)
      val raw = fraction * rangeSpan + range.start
      val candidate =
        normalizedStep?.let { ((raw - range.start) / it).roundToInt() * it + range.start } ?: raw
      return candidate.coerceIn(range.start, range.endInclusive)
    }

    val clampedFraction = fractionOf(value.coerceIn(range.start, range.endInclusive))
    val filledFraction = if (inRange) clampedFraction else if (value < range.start) 0f else 1f
    val thumbOffset = clampedFraction * travel

    Box(
      modifier =
        Modifier.fillMaxWidth().height(trackHeight).background(trackColor, AppShapes.circle)
    ) {
      Box(
        modifier =
          Modifier.fillMaxWidth(filledFraction)
            .height(trackHeight)
            .background(fillColor, AppShapes.circle)
      )
    }

    Box(
      modifier =
        Modifier.fillMaxSize().pointerInput(travelPx, range, normalizedStep) {
          awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = true)
            val slop = viewConfiguration.touchSlop
            var total = Offset.Zero
            var dragging = false
            var released = false
            val gesture =
              SliderGestureSession(
                initialValue = currentValue,
                valueFromX = ::valueFromX,
                onDragStart = { currentOnDragStart() },
                onDrag = { next ->
                  sliderHapticFeedbackType(range, normalizedStep, next)
                    ?.let(currentHapticFeedback::performHapticFeedback)
                  currentOnDrag(next)
                },
              )

            while (true) {
              val event = awaitPointerEvent()
              val change = event.changes.firstOrNull { it.id == down.id } ?: break

              if (change.changedToUp()) {
                if (!dragging) {
                  gesture.start()
                }
                gesture.updateAt(change.position.x)
                gesture.release()?.let { next -> currentOnDragEnd(next) }
                released = true
                break
              }
              if (!dragging && change.isConsumed) {
                gesture.cancel()
                break
              }

              total += change.positionChange()
              if (!dragging) {
                if (abs(total.y) > slop) {
                  gesture.cancel()
                  break
                }
                if (abs(total.x) > slop) {
                  dragging = true
                  gesture.start()
                  change.consume()
                }
              }
              if (dragging) {
                gesture.updateAt(change.position.x)
                change.consume()
              }
            }
            if (!released) {
              gesture.cancel()
              currentOnDragCancel()
            }
          }
        }
    )

    Box(
      modifier =
        Modifier.graphicsLayer { translationX = thumbOffset.toPx() }
          .size(thumbSize)
          .shadow(AppTheme.shadows.sm, AppShapes.circle)
          .border(1.dp, AppTheme.colors.borderDefault, AppShapes.circle)
          .background(AppTheme.colors.surfaceDefault, AppShapes.circle),
      contentAlignment = Alignment.Center,
    ) {
      thumbContent(inRange)
    }
  }
}
