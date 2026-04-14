package co.typie.ui.component

import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.InlineTextContent
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import co.typie.ui.skeleton.LocalSkeleton
import co.typie.ui.skeleton.SkeletonTextBone
import co.typie.ui.theme.AppTheme

@Composable
fun Text(
  text: String,
  modifier: Modifier = Modifier,
  style: TextStyle = AppTheme.typography.body,
  color: Color = AppTheme.colors.textPrimary,
  overflow: TextOverflow = TextOverflow.Clip,
  softWrap: Boolean = true,
  maxLines: Int = Int.MAX_VALUE,
  textAlign: TextAlign = TextAlign.Start,
) {
  val skeleton = LocalSkeleton.current
  if (skeleton.enabled) {
    SkeletonTextBone(text = text, style = style, modifier = modifier, maxLines = maxLines)
    return
  }

  BasicText(
    text = text,
    modifier = modifier,
    style = style.copy(color = color, textAlign = textAlign),
    overflow = overflow,
    softWrap = softWrap,
    maxLines = maxLines,
  )
}

@Composable
fun Text(
  text: AnnotatedString,
  modifier: Modifier = Modifier,
  style: TextStyle = AppTheme.typography.body,
  color: Color = AppTheme.colors.textPrimary,
  overflow: TextOverflow = TextOverflow.Clip,
  softWrap: Boolean = true,
  maxLines: Int = Int.MAX_VALUE,
  textAlign: TextAlign = TextAlign.Start,
  inlineContent: Map<String, InlineTextContent> = mapOf(),
) {
  val skeleton = LocalSkeleton.current
  if (skeleton.enabled) {
    SkeletonTextBone(text = text, style = style, modifier = modifier, maxLines = maxLines)
    return
  }

  BasicText(
    text = text,
    modifier = modifier,
    style = style.copy(color = color, textAlign = textAlign),
    overflow = overflow,
    softWrap = softWrap,
    maxLines = maxLines,
    inlineContent = inlineContent,
  )
}
