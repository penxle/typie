package co.typie.screen.settings.presetsettings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

private const val MIN_PAGE_SIZE_MM = 100
private val MIN_CONTENT_SIZE_PX = mmToPx(50)

@Composable
context(_: SheetScope<Unit>)
internal fun PresetPageLayoutSheet(preset: Preset, onSave: (PresetPageLayout.Paginated) -> Unit) {
  val initial = preset.layout as? PresetPageLayout.Paginated ?: PresetPageLayout.Paginated()

  var pageWidth by remember { mutableIntStateOf(initial.pageWidth) }
  var pageHeight by remember { mutableIntStateOf(initial.pageHeight) }
  var marginTop by remember { mutableIntStateOf(initial.pageMarginTop) }
  var marginBottom by remember { mutableIntStateOf(initial.pageMarginBottom) }
  var marginLeft by remember { mutableIntStateOf(initial.pageMarginLeft) }
  var marginRight by remember { mutableIntStateOf(initial.pageMarginRight) }

  fun save() {
    onSave(
      PresetPageLayout.Paginated(
        pageWidth = pageWidth,
        pageHeight = pageHeight,
        pageMarginTop = marginTop,
        pageMarginBottom = marginBottom,
        pageMarginLeft = marginLeft,
        pageMarginRight = marginRight,
      )
    )
  }

  fun clampMarginTop(value: Int): Int =
    value.coerceIn(0, maxOf(0, pageHeight - marginBottom - MIN_CONTENT_SIZE_PX))
  fun clampMarginBottom(value: Int): Int =
    value.coerceIn(0, maxOf(0, pageHeight - marginTop - MIN_CONTENT_SIZE_PX))
  fun clampMarginLeft(value: Int): Int =
    value.coerceIn(0, maxOf(0, pageWidth - marginRight - MIN_CONTENT_SIZE_PX))
  fun clampMarginRight(value: Int): Int =
    value.coerceIn(0, maxOf(0, pageWidth - marginLeft - MIN_CONTENT_SIZE_PX))

  SheetLayout(
    header = {
      SheetBar(
        center = {
          Text(
            text = "페이지 설정",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    }
  ) {
    Column(verticalArrangement = Arrangement.spacedBy(20.dp)) {
      Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("페이지 크기", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

        Row(
          modifier = Modifier.fillMaxWidth(),
          horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
          MmInputField(
            label = "가로",
            valuePx = pageWidth,
            onCommit = {
              pageWidth = mmToPx(maxOf(MIN_PAGE_SIZE_MM, pxToMm(it)))
              save()
            },
            modifier = Modifier.weight(1f),
          )
          MmInputField(
            label = "세로",
            valuePx = pageHeight,
            onCommit = {
              pageHeight = mmToPx(maxOf(MIN_PAGE_SIZE_MM, pxToMm(it)))
              save()
            },
            modifier = Modifier.weight(1f),
          )
        }
      }

      Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("여백", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

        Row(
          modifier = Modifier.fillMaxWidth(),
          horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
          MmInputField(
            label = "상",
            valuePx = marginTop,
            onCommit = {
              marginTop = clampMarginTop(it)
              save()
            },
            modifier = Modifier.weight(1f),
          )
          MmInputField(
            label = "하",
            valuePx = marginBottom,
            onCommit = {
              marginBottom = clampMarginBottom(it)
              save()
            },
            modifier = Modifier.weight(1f),
          )
        }

        Row(
          modifier = Modifier.fillMaxWidth(),
          horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
          MmInputField(
            label = "좌",
            valuePx = marginLeft,
            onCommit = {
              marginLeft = clampMarginLeft(it)
              save()
            },
            modifier = Modifier.weight(1f),
          )
          MmInputField(
            label = "우",
            valuePx = marginRight,
            onCommit = {
              marginRight = clampMarginRight(it)
              save()
            },
            modifier = Modifier.weight(1f),
          )
        }
      }
    }
  }
}

@Composable
private fun MmInputField(
  label: String,
  valuePx: Int,
  onCommit: (Int) -> Unit,
  modifier: Modifier = Modifier,
) {
  val displayMm = pxToMm(valuePx)
  var textFieldValue by remember(displayMm) { mutableStateOf(TextFieldValue(displayMm.toString())) }
  var isFocused by remember { mutableStateOf(false) }
  val focusManager = LocalFocusManager.current

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
        Modifier.weight(1f).onFocusChanged { state ->
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

private fun mmToPx(mm: Int): Int = kotlin.math.round((mm * 96.0) / 25.4).toInt()

private fun pxToMm(px: Int): Int = kotlin.math.round((px * 25.4) / 96.0).toInt()
