package co.typie.screen.editor.editor.placeholder

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.LineHeightStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import co.typie.editor.body.EditorBodyGeometry
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolveMeasuredPageLength
import co.typie.editor.body.resolvePageContentTop
import co.typie.editor.ffi.Alignment as FfiAlignment
import co.typie.editor.ffi.PlaceholderMetrics
import co.typie.editor.ffi.Size as PageSize
import co.typie.ext.clickable
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import kotlin.math.max

private const val PlaceholderIconOpticalOffsetEm = 0.1f
private const val PtToPx = 96f / 72f

@Composable
internal fun EditorDocumentPlaceholder(
  placeholder: PlaceholderMetrics?,
  geometry: EditorBodyGeometry,
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<PageSize>,
  displayZoom: Float,
  modifier: Modifier = Modifier,
  onLoadTemplate: suspend () -> Unit,
) {
  val density = LocalDensity.current
  val placement =
    resolveEditorDocumentPlaceholderPlacement(
      placeholder = placeholder,
      geometry = geometry,
      layoutSpec = layoutSpec,
      pageSizes = pageSizes,
      displayZoom = displayZoom,
      density = density.density,
    ) ?: return
  val textColor = AppTheme.colors.textHint
  val fontSize = with(density) { placement.fontSizePx.dp.toSp() }
  val lineHeight = with(density) { (placement.fontSizePx * placement.lineHeightRatio).dp.toSp() }
  val letterSpacing = placement.letterSpacingEm.em
  val textStyle =
    TextStyle(fontSize = fontSize, lineHeight = lineHeight, letterSpacing = letterSpacing)
  val firstLineTextStyle =
    textStyle.copy(
      lineHeightStyle =
        LineHeightStyle(
          alignment = LineHeightStyle.Alignment.Center,
          trim = LineHeightStyle.Trim.LastLineBottom,
        )
    )
  val secondLineTextStyle =
    textStyle.copy(
      lineHeightStyle =
        LineHeightStyle(
          alignment = LineHeightStyle.Alignment.Center,
          trim = LineHeightStyle.Trim.FirstLineTop,
        )
    )

  Box(
    modifier =
      modifier
        .graphicsLayer {
          translationX = placement.left.dp.toPx()
          translationY = placement.top.dp.toPx()
        }
        .width(placement.width.dp)
  ) {
    Column(
      modifier = Modifier.width(placement.width.dp),
      horizontalAlignment = placement.horizontalAlignment,
      verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
      Text(
        text = "내용을 입력하거나",
        modifier = Modifier.width(placement.width.dp),
        style = firstLineTextStyle,
        color = textColor,
        textAlign = placement.textAlign,
      )
      Row(
        modifier = Modifier.semantics { role = Role.Button }.clickable { onLoadTemplate() },
        verticalAlignment = Alignment.Top,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
      ) {
        Icon(
          icon = Lucide.LayoutTemplate,
          modifier =
            Modifier.size(placement.fontSizePx.dp).graphicsLayer {
              translationY = placement.fontSizePx.dp.toPx() * PlaceholderIconOpticalOffsetEm
            },
          tint = textColor,
        )
        Text(text = "템플릿 불러오기", style = secondLineTextStyle, color = textColor)
      }
    }
  }
}

internal data class EditorDocumentPlaceholderPlacement(
  val left: Float,
  val top: Float,
  val width: Float,
  val fontSizePx: Float,
  val lineHeightRatio: Float,
  val letterSpacingEm: Float,
  val textAlign: TextAlign,
  val horizontalAlignment: Alignment.Horizontal,
)

internal fun resolveEditorDocumentPlaceholderPlacement(
  placeholder: PlaceholderMetrics?,
  geometry: EditorBodyGeometry,
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<PageSize>,
  displayZoom: Float = 1f,
  density: Float = 0f,
): EditorDocumentPlaceholderPlacement? {
  val metrics = placeholder ?: return null
  val fontSize = metrics.fontSize ?: return null
  val lineHeight = metrics.lineHeight ?: return null
  val letterSpacing = metrics.letterSpacing ?: return null
  val align = metrics.align ?: return null
  val page = metrics.pageIdx
  if (page !in pageSizes.indices) {
    return null
  }

  val zoom = normalizeDisplayZoom(displayZoom)
  val pageTop =
    layoutSpec.resolvePageContentTop(
      page = page,
      pageSizes = pageSizes,
      displayZoom = zoom,
      density = density,
    ) ?: return null
  val columnWidth = geometry.pageColumnWidth.coerceAtLeast(0f)
  if (columnWidth <= 0f) {
    return null
  }

  val contentWidth = max(geometry.visibleBodySize.width, columnWidth).coerceAtLeast(0f)
  val columnLeft = ((contentWidth - columnWidth) / 2f).coerceAtLeast(0f)
  val pageDisplayWidth =
    resolveMeasuredPageLength(pageSizes[page].width, displayZoom = zoom, density = density)
  val pageLeft = columnLeft + ((columnWidth - pageDisplayWidth) / 2f).coerceAtLeast(0f)
  val rect = metrics.rect
  val width = rect.width * zoom
  if (width <= 0f) {
    return null
  }

  val fontSizePx = (fontSize.toFloat() / 100f) * PtToPx * zoom
  return EditorDocumentPlaceholderPlacement(
    left = pageLeft + rect.x * zoom,
    top = geometry.topSpacerHeight + pageTop + rect.y * zoom,
    width = width,
    fontSizePx = fontSizePx,
    lineHeightRatio = lineHeight.toFloat() / 100f,
    letterSpacingEm = letterSpacing.toFloat() / 100f,
    textAlign = align.toTextAlign(),
    horizontalAlignment = align.toHorizontalAlignment(),
  )
}

private fun FfiAlignment.toTextAlign(): TextAlign =
  when (this) {
    FfiAlignment.Left -> TextAlign.Start
    FfiAlignment.Center -> TextAlign.Center
    FfiAlignment.Right -> TextAlign.End
    FfiAlignment.Justify -> TextAlign.Justify
  }

private fun FfiAlignment.toHorizontalAlignment(): Alignment.Horizontal =
  when (this) {
    FfiAlignment.Center -> Alignment.CenterHorizontally
    FfiAlignment.Right -> Alignment.End
    FfiAlignment.Left,
    FfiAlignment.Justify -> Alignment.Start
  }

private fun normalizeDisplayZoom(displayZoom: Float): Float =
  if (displayZoom.isFinite() && displayZoom > 0f) {
    displayZoom
  } else {
    1f
  }
