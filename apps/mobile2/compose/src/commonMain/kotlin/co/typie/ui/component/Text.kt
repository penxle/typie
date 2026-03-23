package co.typie.ui.component

import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import co.typie.ui.skeleton.LocalSkeleton
import co.typie.ui.skeleton.SkeletonTextBone
import co.typie.ui.theme.AppTheme

@Composable
fun Text(
  text: String,
  modifier: Modifier = Modifier,
  style: TextStyle = AppTheme.typography.body,
  color: Color = AppTheme.colors.textDefault,
  overflow: TextOverflow = TextOverflow.Clip,
  maxLines: Int = Int.MAX_VALUE,
) {
  val skeleton = LocalSkeleton.current
  if (skeleton.enabled) {
    SkeletonTextBone(text = text, style = style, modifier = modifier, maxLines = maxLines)
    return
  }

  BasicText(
    text = text,
    modifier = modifier,
    style = style.copy(color = color),
    overflow = overflow,
    maxLines = maxLines,
  )
}
