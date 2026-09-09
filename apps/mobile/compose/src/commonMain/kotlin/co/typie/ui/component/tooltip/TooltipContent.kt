package co.typie.ui.component.tooltip

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.LineHeightStyle
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import co.typie.generated.resources.Res
import co.typie.generated.resources.tooltip_shortcuts
import co.typie.ui.theme.AppTheme
import co.typie.ui.utils.platformModUsesMeta
import org.jetbrains.compose.resources.Font

internal data class TooltipContent(
  val message: TextLayoutResult,
  val keys: List<TextLayoutResult>,
  val keyWidths: List<Int>,
  val keyGap: Int,
  val keyHeight: Int,
  val maxWidth: Int,
) {
  private val shortcutWidth = keyWidths.sum() + keyGap * (keys.size - 1).coerceAtLeast(0)
  val size =
    IntSize(
      maxOf(message.size.width, shortcutWidth).coerceAtMost(maxWidth),
      message.size.height + if (keys.isEmpty()) 0 else keyHeight,
    )
}

@Composable
internal fun rememberTooltipContent(
  text: String,
  shortcut: String?,
  maxWidth: Int,
): TooltipContent {
  val density = LocalDensity.current
  val measurer = rememberTextMeasurer()
  val style =
    AppTheme.typography.action.copy(
      fontWeight = FontWeight.SemiBold,
      fontSize = 12.sp,
      lineHeight = 16.8.sp,
      lineHeightStyle =
        LineHeightStyle(LineHeightStyle.Alignment.Center, LineHeightStyle.Trim.None),
      fontFeatureSettings = "\"ss05\" 1, \"cv12\" 1, \"ss18\" 1",
      letterSpacing = (-0.015).em,
    )
  val font = FontFamily(Font(Res.font.tooltip_shortcuts, FontWeight.Medium))
  val shortcutStyle =
    style.copy(fontFamily = font, fontWeight = FontWeight.Medium, lineHeight = 12.sp)
  val minKeyWidth = with(density) { 12.dp.roundToPx() }
  val keyHeight = with(density) { 12.sp.roundToPx() }
  val keyGap = with(density) { if (platformModUsesMeta) 0 else 2.dp.roundToPx() }
  return remember(
    text,
    shortcut,
    maxWidth,
    style,
    shortcutStyle,
    minKeyWidth,
    keyGap,
    keyHeight,
    measurer,
  ) {
    val keys = shortcut?.let { tooltipShortcutKeys(it, platformModUsesMeta) }.orEmpty()
    val layouts = keys.map {
      measurer.measure(it, shortcutStyle, constraints = Constraints(maxWidth = maxWidth))
    }
    TooltipContent(
      measurer.measure(text, style, constraints = Constraints(maxWidth = maxWidth)),
      layouts,
      layouts.mapIndexed { index, layout ->
        if (!platformModUsesMeta && index % 2 == 1) layout.size.width
        else maxOf(minKeyWidth, layout.size.width)
      },
      keyGap,
      keyHeight,
      maxWidth,
    )
  }
}

internal fun tooltipShortcutKeys(label: String, usesMeta: Boolean): List<String> = buildList {
  var remaining = label
  if (usesMeta) {
    while (remaining.firstOrNull() in listOf('⌃', '⌥', '⇧', '⌘')) {
      add(remaining.take(1))
      remaining = remaining.drop(1)
    }
  } else {
    while (remaining.indexOf('+') > 0) {
      val delimiter = remaining.indexOf('+')
      add(remaining.take(delimiter))
      add("+")
      remaining = remaining.drop(delimiter + 1)
    }
  }
  if (remaining.isNotEmpty()) add(remaining)
}

internal fun DrawScope.drawTooltipContent(content: TooltipContent, color: Color, alpha: Float) {
  // Both versions remain centered in the animated surface, just as the web's content transform.
  val origin =
    Offset((size.width - content.size.width) / 2f, (size.height - content.size.height) / 2f)
  drawText(content.message, color = color, topLeft = origin, alpha = alpha)
  var x = origin.x
  content.keys.forEachIndexed { index, key ->
    drawText(
      key,
      color = color,
      topLeft =
        Offset(
          x + (content.keyWidths[index] - key.size.width) / 2f,
          // A CSS line box can be shorter than the font metrics; center the glyphs in 1em.
          origin.y + content.message.size.height + (content.keyHeight - key.size.height) / 2f,
        ),
      alpha = alpha * 0.5f,
    )
    x += content.keyWidths[index] + content.keyGap
  }
}
