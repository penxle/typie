package co.typie.ui.component

import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.InlineTextContent
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import co.typie.ui.skeleton.skeletonTextBone
import co.typie.ui.theme.AppTheme

@Composable
fun Text(
  text: String,
  modifier: Modifier = Modifier,
  style: TextStyle = AppTheme.typography.body,
  color: Color = AppTheme.colors.textDefault,
  overflow: TextOverflow = TextOverflow.Clip,
  softWrap: Boolean = true,
  minLines: Int = 1,
  maxLines: Int = Int.MAX_VALUE,
  textAlign: TextAlign = TextAlign.Start,
) {
  var layoutResult by remember { mutableStateOf<TextLayoutResult?>(null) }
  BasicText(
    text = text,
    modifier = modifier.skeletonTextBone { layoutResult },
    style = style.copy(color = color, textAlign = textAlign),
    overflow = overflow,
    softWrap = softWrap,
    minLines = minLines,
    maxLines = maxLines,
    onTextLayout = { layoutResult = it },
  )
}

@Composable
fun Text(
  text: AnnotatedString,
  modifier: Modifier = Modifier,
  style: TextStyle = AppTheme.typography.body,
  color: Color = AppTheme.colors.textDefault,
  overflow: TextOverflow = TextOverflow.Clip,
  softWrap: Boolean = true,
  minLines: Int = 1,
  maxLines: Int = Int.MAX_VALUE,
  textAlign: TextAlign = TextAlign.Start,
  inlineContent: Map<String, InlineTextContent> = mapOf(),
) {
  var layoutResult by remember { mutableStateOf<TextLayoutResult?>(null) }
  BasicText(
    text = text,
    modifier = modifier.skeletonTextBone { layoutResult },
    style = style.copy(color = color, textAlign = textAlign),
    overflow = overflow,
    softWrap = softWrap,
    minLines = minLines,
    maxLines = maxLines,
    inlineContent = inlineContent,
    onTextLayout = { layoutResult = it },
  )
}
