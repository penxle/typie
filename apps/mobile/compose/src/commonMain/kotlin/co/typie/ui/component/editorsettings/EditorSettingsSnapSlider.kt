package co.typie.ui.component.editorsettings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.ext.ime
import co.typie.ext.rememberTextInputBinding
import co.typie.ext.textInputFocusable
import co.typie.icons.Lucide
import co.typie.ui.component.Slider
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.launch

@Composable
internal fun EditorSettingsSnapSlider(
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
  val haptic = LocalHapticFeedback.current
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

    Slider(
      value = displayValue.toFloat(),
      range = range.first.toFloat()..range.last.toFloat(),
      step = sliderStep.toFloat(),
      onDragStart = {
        isDragging = true
        dragValue = value
        focusManager.clearFocus()
      },
      onDrag = { next ->
        haptic.performHapticFeedback(HapticFeedbackType.SegmentTick)
        dragValue = next.roundToInt().coerceIn(range.first, range.last)
      },
      onDragEnd = { next ->
        val rounded = next.roundToInt().coerceIn(range.first, range.last)
        dragValue = rounded
        scope.launch {
          onValueChange(rounded)
          isDragging = false
        }
      },
      onDragCancel = {
        dragValue = value
        isDragging = false
      },
      modifier = Modifier.fillMaxWidth().height(32.dp),
      thumbContent = { inRange ->
        if (!inRange) {
          Icon(
            icon = if (displayValue < range.first) Lucide.ChevronsLeft else Lucide.ChevronsRight,
            modifier = Modifier.size(14.dp),
            tint = AppTheme.colors.textMuted,
          )
        }
      },
    )
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
  val focusManager = LocalFocusManager.current
  val scope = rememberCoroutineScope()
  val textInputBinding = rememberTextInputBinding(onDismiss = { focusManager.clearFocus() })

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
      scope.launch(start = CoroutineStart.UNDISPATCHED) {
        onValueChange(rounded.coerceIn(fullRange))
      }
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
        Modifier.width(IntrinsicSize.Min).textInputFocusable(textInputBinding) { state ->
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
            textInputBinding.requestFocus()
          },
      )
    }
  }
}
