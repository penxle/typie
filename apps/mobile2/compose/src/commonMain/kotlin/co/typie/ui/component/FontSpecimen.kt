package co.typie.ui.component

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImagePainter
import coil3.compose.rememberAsyncImagePainter
import co.typie.Konfig
import co.typie.ui.theme.AppTheme
import co.typie.ui.utils.toHexRgbString
import io.ktor.http.URLBuilder
import io.ktor.http.appendPathSegments

@Composable
fun FontSpecimen(
  text: String,
  fontId: String?,
  weight: Int?,
  style: TextStyle,
  modifier: Modifier = Modifier,
  fallbackTexts: List<String> = emptyList(),
  contentAlignment: Alignment = Alignment.CenterStart,
) {
  val fallbackStyle = if (weight != null) {
    style.copy(fontWeight = FontWeight(weight.coerceIn(1, 1000)))
  } else {
    style
  }

  val fallback: @Composable () -> Unit = {
    Text(
      text = text,
      style = fallbackStyle,
      modifier = Modifier.wrapContentWidth(),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }

  if (fontId == null) {
    Box(
      modifier = modifier,
      contentAlignment = contentAlignment,
    ) {
      fallback()
    }
    return
  }

  val specimenHeight = with(LocalDensity.current) { style.fontSize.toDp() } + 4.dp
  val resolvedColor = if (style.color == Color.Unspecified) AppTheme.colors.textPrimary else style.color
  val specimenUrl = remember(fontId, text, fallbackTexts, resolvedColor) {
    fontSpecimenUrl(
      fontId = fontId,
      text = text,
      fallbackTexts = fallbackTexts,
      colorHex = resolvedColor.toHexRgbString(),
    )
  }
  val painter = rememberAsyncImagePainter(model = specimenUrl)
  val painterState by painter.state.collectAsState()

  Box(
    modifier = modifier.heightIn(min = specimenHeight),
    contentAlignment = contentAlignment,
  ) {
    if (painterState is AsyncImagePainter.State.Success) {
      Image(
        painter = painter,
        contentDescription = null,
        contentScale = ContentScale.Fit,
        modifier = Modifier
          .height(specimenHeight)
          .wrapContentWidth(Alignment.Start),
      )
    } else {
      Box(
        modifier = Modifier
          .height(specimenHeight)
          .wrapContentWidth(Alignment.Start),
        contentAlignment = Alignment.CenterStart,
      ) {
        fallback()
      }
    }
  }
}

internal fun familySpecimenFallbacks(
  displayName: String,
  familyName: String,
): List<String> {
  return chooseDistinctFallbackTexts(
    primaryText = displayName,
    candidates = listOf(familyName),
  )
}

internal fun weightSpecimenFallbacks(
  label: String,
  subfamilyDisplayName: String?,
  weight: Int,
): List<String> {
  return chooseDistinctFallbackTexts(
    primaryText = label,
    candidates = listOf(subfamilyDisplayName, weight.toString()),
  )
}

private fun chooseDistinctFallbackTexts(
  primaryText: String,
  candidates: List<String?>,
): List<String> {
  val normalizedPrimary = primaryText.trim()
  val seen = mutableSetOf(normalizedPrimary.lowercase())

  return candidates
    .mapNotNull { candidate -> candidate?.trim()?.takeIf { it.isNotEmpty() } }
    .filter { candidate ->
      val normalizedCandidate = candidate.lowercase()
      if (seen.contains(normalizedCandidate)) {
        return@filter false
      }

      seen += normalizedCandidate
      true
    }
}

internal fun fontSpecimenUrl(
  fontId: String,
  text: String,
  fallbackTexts: List<String> = emptyList(),
  colorHex: String? = null,
): String {
  return URLBuilder(Konfig.API_URL).apply {
    appendPathSegments("font", fontId, "specimen")
    parameters.append("text", text)
    chooseDistinctFallbackTexts(
      primaryText = text,
      candidates = fallbackTexts,
    ).forEach { fallbackText ->
      parameters.append("fallbacks", fallbackText)
    }
    colorHex?.takeIf { it.isNotBlank() }?.let {
      parameters.append("color", it)
    }
  }.buildString()
}
