package co.typie.screen.settings.presetsettings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.times
import co.typie.ext.clickable
import co.typie.ext.ime
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlinx.coroutines.launch

@Composable
internal fun PresetSnapSlider(
  label: String,
  value: Int,
  onValueChange: suspend (Int) -> Unit,
  range: IntRange,
  sliderStep: Int,
  inputStep: Int = sliderStep,
  formatValue: (Int) -> String,
  parseValue: (String) -> Int?,
  unitSuffix: String = "",
  fullRange: IntRange = range,
  modifier: Modifier = Modifier,
) {
  val scope = rememberCoroutineScope()
  val focusManager = LocalFocusManager.current
  val bringIntoViewRequester = remember { BringIntoViewRequester() }

  var isDragging by remember { mutableStateOf(false) }
  var dragValue by remember { mutableStateOf(value) }
  val displayValue = if (isDragging) dragValue else value

  Column(
    modifier = modifier.fillMaxWidth().bringIntoViewRequester(bringIntoViewRequester),
    verticalArrangement = Arrangement.spacedBy(2.dp),
  ) {
    Row(
      modifier = Modifier.fillMaxWidth(),
      horizontalArrangement = Arrangement.SpaceBetween,
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Text(text = label, style = AppTheme.typography.label, color = AppTheme.colors.textMuted)

      ValueBadge(
        value = displayValue,
        isDragging = isDragging,
        onValueChange = onValueChange,
        step = inputStep,
        fullRange = fullRange,
        formatValue = formatValue,
        parseValue = parseValue,
        unitSuffix = unitSuffix,
        bringIntoViewRequester = bringIntoViewRequester,
      )
    }

    SliderTrack(
      value = displayValue,
      onDrag = { dragValue = it },
      onDragStart = {
        isDragging = true
        dragValue = value
        focusManager.clearFocus()
      },
      onDragEnd = {
        scope.launch {
          onValueChange(dragValue)
          isDragging = false
        }
      },
      range = range,
      step = sliderStep,
    )
  }
}

@Composable
private fun SliderTrack(
  value: Int,
  onDrag: (Int) -> Unit,
  onDragStart: () -> Unit,
  onDragEnd: () -> Unit,
  range: IntRange,
  step: Int,
) {
  val colors = AppTheme.colors
  val density = LocalDensity.current
  val haptic = LocalHapticFeedback.current
  val thumbSize = 24.dp

  val inRange = value in range
  val rangeSpan = (range.last - range.first).toFloat()

  BoxWithConstraints(
    modifier = Modifier.fillMaxWidth().height(32.dp),
    contentAlignment = Alignment.CenterStart,
  ) {
    val travel = (maxWidth - thumbSize).coerceAtLeast(0.dp)
    val travelPx = with(density) { travel.toPx() }
    val thumbRadiusPx = with(density) { (thumbSize / 2).toPx() }

    fun fractionOf(v: Int): Float = ((v - range.first) / rangeSpan).coerceIn(0f, 1f)

    fun valueFromX(x: Float): Int {
      val fraction = ((x - thumbRadiusPx) / travelPx).coerceIn(0f, 1f)
      val raw = fraction * rangeSpan + range.first
      val candidate = ((raw - range.first) / step).roundToInt() * step + range.first
      return candidate.coerceIn(range)
    }

    val clampedFraction = fractionOf(value.coerceIn(range))
    val filledFraction = if (inRange) clampedFraction else if (value < range.first) 0f else 1f
    val thumbOffset = clampedFraction * travel

    Box(
      modifier =
        Modifier.fillMaxWidth()
          .height(8.dp)
          .background(colors.borderEmphasis.copy(alpha = 0.5f), AppShapes.circle)
    ) {
      Box(
        modifier =
          Modifier.fillMaxWidth(filledFraction)
            .height(8.dp)
            .background(colors.textDefault, AppShapes.circle)
      )
    }

    Box(
      modifier =
        Modifier.matchParentSize().pointerInput(maxWidth, range) {
          awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = true)
            val slop = viewConfiguration.touchSlop
            var total = Offset.Zero
            var dragging = false
            var current = value

            fun update(x: Float) {
              val next = valueFromX(x)
              if (next == current) return
              current = next
              haptic.performHapticFeedback(HapticFeedbackType.SegmentTick)
              onDrag(next)
            }

            while (true) {
              val event = awaitPointerEvent()
              val change = event.changes.firstOrNull { it.id == down.id } ?: break

              if (change.changedToUp()) {
                if (!dragging) {
                  onDragStart()
                  update(change.position.x)
                  onDragEnd()
                }
                break
              }
              if (change.isConsumed) break

              total += change.positionChange()
              if (!dragging) {
                if (abs(total.y) > slop) break
                if (abs(total.x) > slop) {
                  dragging = true
                  onDragStart()
                  change.consume()
                }
              }
              if (dragging) {
                update(change.position.x)
                change.consume()
              }
            }
            if (dragging) onDragEnd()
          }
        }
    )

    Box(
      modifier =
        Modifier.graphicsLayer { translationX = thumbOffset.toPx() }
          .size(thumbSize)
          .dropShadow(AppShapes.circle) {
            color = colors.shadowAmbient
            radius = 4f
          }
          .dropShadow(AppShapes.circle) {
            color = colors.shadowSpot
            radius = 8f
            offset = Offset(0f, 1f)
          }
          .border(1.dp, colors.borderDefault, AppShapes.circle)
          .background(colors.surfaceDefault, AppShapes.circle),
      contentAlignment = Alignment.Center,
    ) {
      if (!inRange) {
        Icon(
          icon = if (value < range.first) Lucide.ChevronsLeft else Lucide.ChevronsRight,
          modifier = Modifier.size(14.dp),
          tint = colors.textMuted,
        )
      }
    }
  }
}

@Composable
private fun ValueBadge(
  value: Int,
  isDragging: Boolean,
  onValueChange: suspend (Int) -> Unit,
  step: Int,
  fullRange: IntRange,
  formatValue: (Int) -> String,
  parseValue: (String) -> Int?,
  unitSuffix: String,
  bringIntoViewRequester: BringIntoViewRequester,
) {
  var editText by remember { mutableStateOf(TextFieldValue("")) }
  var isFocused by remember { mutableStateOf(false) }
  val focusRequester = remember { FocusRequester() }
  val focusManager = LocalFocusManager.current
  val scope = rememberCoroutineScope()

  val density = LocalDensity.current
  val imeBottom = WindowInsets.ime.getBottom(density)

  LaunchedEffect(isFocused, imeBottom) {
    if (isFocused && imeBottom > 0) {
      bringIntoViewRequester.bringIntoView()
    }
  }

  val numberText = formatValue(value).removeSuffix(unitSuffix)
  val numberColor = if (isFocused) AppTheme.colors.textDefault else AppTheme.colors.textMuted
  val badgeShape = AppShapes.rounded(AppShapes.sm)

  fun commit() {
    if (isDragging) return
    val parsed = parseValue(editText.text)
    if (parsed != null) {
      val clamped = parsed.coerceIn(fullRange)
      val rounded =
        ((clamped.toFloat() - fullRange.first) / step).roundToInt() * step + fullRange.first
      scope.launch { onValueChange(rounded.coerceIn(fullRange)) }
    }
  }

  Row(
    modifier =
      Modifier.border(
          1.5.dp,
          if (isFocused) AppTheme.colors.textDefault else Color.Transparent,
          badgeShape,
        )
        .background(AppTheme.colors.surfaceInset, badgeShape)
        .padding(horizontal = 8.dp, vertical = 4.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    BasicTextField(
      value = if (isFocused) editText else TextFieldValue(numberText),
      onValueChange = { editText = it },
      modifier =
        Modifier.width(IntrinsicSize.Min).focusRequester(focusRequester).onFocusChanged { state ->
          val wasFocused = isFocused
          isFocused = state.isFocused
          if (state.isFocused && !wasFocused) {
            if (editText.text != numberText) {
              editText = TextFieldValue(numberText)
            }
          } else if (!state.isFocused && wasFocused) {
            commit()
          }
        },
      textStyle = AppTheme.typography.action.copy(color = numberColor),
      cursorBrush = SolidColor(AppTheme.colors.textDefault),
      keyboardOptions =
        KeyboardOptions(keyboardType = KeyboardType.Decimal, imeAction = ImeAction.Done),
      keyboardActions = KeyboardActions(onDone = { focusManager.clearFocus() }),
      singleLine = true,
    )

    if (unitSuffix.isNotEmpty()) {
      Text(
        text = unitSuffix,
        style = AppTheme.typography.action,
        color = AppTheme.colors.textMuted,
        modifier =
          Modifier.clickable {
            if (!isFocused) {
              editText = TextFieldValue(numberText, TextRange(numberText.length))
            }
            focusRequester.requestFocus()
          },
      )
    }
  }
}
