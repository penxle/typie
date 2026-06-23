package co.typie.ui.component

import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import co.typie.Konfig
import co.typie.ui.theme.AppTheme
import co.typie.ui.utils.toHexRgbString
import io.ktor.http.Url
import io.ktor.http.appendPathSegments
import io.ktor.http.buildUrl
import io.ktor.http.takeFrom

@Composable
fun FontSpecimen(
  fontId: String,
  text: String,
  fallbackTexts: List<String> = emptyList(),
  style: TextStyle,
) {
  val fallback: @Composable () -> Unit = {
    Text(
      text = text,
      style = style,
      modifier = Modifier.wrapContentWidth(),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }

  val height = with(LocalDensity.current) { style.fontSize.toDp() }
  val color = if (style.color == Color.Unspecified) AppTheme.colors.textDefault else style.color

  val url =
    remember(fontId, text, fallbackTexts) {
      buildUrl(
        fontId = fontId,
        text = text,
        fallbacks = fallbackTexts,
        color = Color.Black.toHexRgbString(),
      )
    }

  Img(
    url = url,
    color = color,
    placeholder = fallback,
    modifier = Modifier.height(height).wrapContentWidth(),
    contentScale = ContentScale.Fit,
  )
}

private fun buildUrl(
  fontId: String,
  text: String,
  fallbacks: List<String> = emptyList(),
  color: String,
): Url = buildUrl {
  takeFrom(Konfig.API_URL)
  appendPathSegments("font", fontId, "specimen")
  parameters.apply {
    append("text", text)
    append("color", color)
    appendAll("fallbacks", fallbacks)
  }
}
