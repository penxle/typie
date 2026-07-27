package co.typie.ui.component.editorsettings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import co.typie.editor.ffi.LayoutMode
import co.typie.ext.rememberTextInputBinding
import co.typie.ext.textInputFocusable
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

internal const val MinPageSizeMm = 100

internal fun mmToPx(mm: Int): Int = kotlin.math.round((mm * 96.0) / 25.4).toInt()

internal fun pxToMm(px: Int): Int = kotlin.math.round((px * 25.4) / 96.0).toInt()

internal val MinContentSizePx = mmToPx(50)

internal fun clampPageMargins(layout: LayoutMode.Paginated): LayoutMode.Paginated {
  val maxHorizontal = maxOf(0, layout.pageWidth - MinContentSizePx)
  val maxVertical = maxOf(0, layout.pageHeight - MinContentSizePx)
  val left = layout.pageMarginLeft.coerceIn(0, maxHorizontal)
  val top = layout.pageMarginTop.coerceIn(0, maxVertical)
  return layout.copy(
    pageMarginLeft = left,
    pageMarginRight = layout.pageMarginRight.coerceIn(0, maxOf(0, maxHorizontal - left)),
    pageMarginTop = top,
    pageMarginBottom = layout.pageMarginBottom.coerceIn(0, maxOf(0, maxVertical - top)),
  )
}

@Composable
internal fun MmInputField(
  label: String,
  valuePx: Int,
  onCommit: (Int) -> Unit,
  modifier: Modifier = Modifier,
) {
  val displayMm = pxToMm(valuePx)
  var textFieldValue by remember(displayMm) { mutableStateOf(TextFieldValue(displayMm.toString())) }
  var isFocused by remember { mutableStateOf(false) }
  val focusManager = LocalFocusManager.current
  val textInputBinding = rememberTextInputBinding(onDismiss = { focusManager.clearFocus() })

  fun commit() {
    val parsed = textFieldValue.text.trim().toIntOrNull()
    if (parsed != null) {
      onCommit(mmToPx(maxOf(0, parsed)))
    } else {
      textFieldValue = TextFieldValue(displayMm.toString())
    }
  }

  DisposableEffect(Unit) { onDispose { focusManager.clearFocus() } }

  Row(
    modifier =
      modifier
        .background(AppTheme.colors.surfaceInset, AppShapes.rounded(AppShapes.sm))
        .padding(horizontal = 12.dp, vertical = 10.dp),
    horizontalArrangement = Arrangement.spacedBy(4.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Text(text = label, style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

    BasicTextField(
      value = textFieldValue,
      onValueChange = { textFieldValue = it },
      modifier =
        Modifier.weight(1f).textInputFocusable(textInputBinding) { state ->
          val wasFocused = isFocused
          isFocused = state.isFocused
          if (wasFocused && !state.isFocused) commit()
        },
      textStyle = AppTheme.typography.caption.copy(color = AppTheme.colors.textDefault),
      cursorBrush = SolidColor(AppTheme.colors.textDefault),
      keyboardOptions =
        KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Done),
      keyboardActions = KeyboardActions(onDone = { focusManager.clearFocus() }),
      singleLine = true,
    )

    Text(text = "mm", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)
  }
}
