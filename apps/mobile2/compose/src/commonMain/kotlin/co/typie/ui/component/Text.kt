package co.typie.ui.component

import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.SuitFontFamily

@Composable
fun Text(
  text: String,
  modifier: Modifier = Modifier,
  style: TextStyle = TextStyle.Default,
  overflow: TextOverflow = TextOverflow.Clip,
  maxLines: Int = Int.MAX_VALUE,
) {
  val defaultStyle = TextStyle(
    fontFamily = SuitFontFamily,
    color = AppTheme.colors.textDefault,
  )

  BasicText(
    text = text,
    modifier = modifier,
    style = defaultStyle.merge(style),
    overflow = overflow,
    maxLines = maxLines,
  )
}
