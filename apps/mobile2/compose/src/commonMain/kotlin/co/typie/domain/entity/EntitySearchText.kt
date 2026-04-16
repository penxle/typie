package co.typie.domain.entity

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString

fun buildSearchHighlightedText(
  text: String,
  highlightColor: Color,
  baseColor: Color? = null,
): AnnotatedString {
  return buildAnnotatedString {
    var remaining = text
    while (remaining.isNotEmpty()) {
      val startIdx = remaining.indexOf("<em>")
      if (startIdx == -1) {
        appendWithColor(remaining, baseColor)
        break
      }

      appendWithColor(remaining.substring(0, startIdx), baseColor)

      val endIdx = remaining.indexOf("</em>", startIdx)
      if (endIdx == -1) {
        appendWithColor(remaining.substring(startIdx), baseColor)
        break
      }

      val highlighted = remaining.substring(startIdx + 4, endIdx)
      pushStyle(SpanStyle(color = highlightColor))
      append(highlighted)
      pop()

      remaining = remaining.substring(endIdx + 5)
    }
  }
}

private fun AnnotatedString.Builder.appendWithColor(text: String, color: Color?) {
  if (text.isEmpty()) return

  if (color == null) {
    append(text)
    return
  }

  pushStyle(SpanStyle(color = color))
  append(text)
  pop()
}
