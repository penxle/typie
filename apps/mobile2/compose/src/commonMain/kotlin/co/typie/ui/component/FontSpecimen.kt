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
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImagePainter
import coil3.compose.rememberAsyncImagePainter
import co.typie.Konfig
import io.ktor.http.encodeURLQueryComponent

@Composable
fun FontSpecimen(
  text: String,
  fontId: String?,
  weight: Int?,
  style: TextStyle,
  modifier: Modifier = Modifier,
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
  val specimenUrl = remember(fontId, text) { fontSpecimenUrl(fontId = fontId, text = text) }
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

private fun fontSpecimenUrl(
  fontId: String,
  text: String,
): String {
  return "${Konfig.API_URL}/font/$fontId/specimen?text=${text.encodeURLQueryComponent()}"
}
