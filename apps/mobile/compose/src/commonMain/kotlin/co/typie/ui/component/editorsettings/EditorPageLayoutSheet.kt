package co.typie.ui.component.editorsettings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.editor.DefaultRootPaginatedLayout
import co.typie.editor.ffi.LayoutMode
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.theme.AppTheme

@Composable
context(_: SheetScope<Unit>)
internal fun EditorPageLayoutSheet(layout: LayoutMode, onSave: (LayoutMode.Paginated) -> Unit) {
  val initial = layout as? LayoutMode.Paginated ?: DefaultRootPaginatedLayout

  var pageWidth by remember { mutableIntStateOf(initial.pageWidth) }
  var pageHeight by remember { mutableIntStateOf(initial.pageHeight) }
  var marginTop by remember { mutableIntStateOf(initial.pageMarginTop) }
  var marginBottom by remember { mutableIntStateOf(initial.pageMarginBottom) }
  var marginLeft by remember { mutableIntStateOf(initial.pageMarginLeft) }
  var marginRight by remember { mutableIntStateOf(initial.pageMarginRight) }

  fun save() {
    onSave(
      LayoutMode.Paginated(
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
    value.coerceIn(0, maxOf(0, pageHeight - marginBottom - MinContentSizePx))
  fun clampMarginBottom(value: Int): Int =
    value.coerceIn(0, maxOf(0, pageHeight - marginTop - MinContentSizePx))
  fun clampMarginLeft(value: Int): Int =
    value.coerceIn(0, maxOf(0, pageWidth - marginRight - MinContentSizePx))
  fun clampMarginRight(value: Int): Int =
    value.coerceIn(0, maxOf(0, pageWidth - marginLeft - MinContentSizePx))

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
              pageWidth = mmToPx(maxOf(MinPageSizeMm, pxToMm(it)))
              save()
            },
            modifier = Modifier.weight(1f),
          )
          MmInputField(
            label = "세로",
            valuePx = pageHeight,
            onCommit = {
              pageHeight = mmToPx(maxOf(MinPageSizeMm, pxToMm(it)))
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
