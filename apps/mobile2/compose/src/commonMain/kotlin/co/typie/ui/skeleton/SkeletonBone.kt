package co.typie.ui.skeleton

import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.rememberTextMeasurer
import co.typie.ui.theme.AppShapes

@Composable
fun SkeletonBone(modifier: Modifier = Modifier, shape: Shape = AppShapes.rounded(AppShapes.sm)) {
  Box(modifier.skeletonBone(shape))
}

@Composable
fun SkeletonTextBone(
  text: String,
  style: TextStyle,
  modifier: Modifier = Modifier,
  maxLines: Int = Int.MAX_VALUE,
) {
  val annotated = remember(text) { AnnotatedString(text) }
  SkeletonTextBone(text = annotated, style = style, modifier = modifier, maxLines = maxLines)
}

@Composable
fun SkeletonTextBone(
  text: AnnotatedString,
  style: TextStyle,
  modifier: Modifier = Modifier,
  maxLines: Int = Int.MAX_VALUE,
) {
  val textMeasurer = rememberTextMeasurer()
  var layoutResult by remember { mutableStateOf<TextLayoutResult?>(null) }

  Layout(
    modifier = modifier.skeletonTextBone { layoutResult },
    measurePolicy = { _, constraints ->
      val result =
        textMeasurer.measure(
          text = text,
          style = style,
          maxLines = maxLines,
          constraints = constraints,
        )
      layoutResult = result
      layout(result.size.width, result.size.height) {}
    },
  )
}
