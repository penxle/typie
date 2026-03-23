package co.typie.ui.skeleton

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp

@Composable
fun SkeletonBone(
  modifier: Modifier = Modifier,
  shape: Shape = RoundedCornerShape(4.dp),
) {
  val skeleton = LocalSkeleton.current
  val uniteGroup = LocalSkeletonUnite.current

  if (uniteGroup != null) {
    val boneColor by skeleton.color
    val cornerRadius = with(LocalDensity.current) { 4.dp.toPx() }
    var coords by remember { mutableStateOf<LayoutCoordinates?>(null) }

    Box(
      modifier
        .onGloballyPositioned { coords = it; uniteGroup.register(it) }
        .graphicsLayer { clip = false }
        .drawWithContent {
          val myCoords = coords ?: return@drawWithContent
          val union = uniteGroup.getUnionBounds(myCoords)
          drawRoundRect(
            color = boneColor,
            topLeft = Offset(union.left, union.top),
            size = Size(union.width, union.height),
            cornerRadius = CornerRadius(cornerRadius),
          )
        },
    )
  } else {
    Box(
      modifier
        .clip(shape)
        .background(skeleton.color.value),
    )
  }
}

@Composable
fun SkeletonTextBone(
  text: String,
  style: TextStyle,
  modifier: Modifier = Modifier,
  maxLines: Int = Int.MAX_VALUE,
) {
  val skeleton = LocalSkeleton.current
  val uniteGroup = LocalSkeletonUnite.current
  val textMeasurer = rememberTextMeasurer()

  if (uniteGroup != null) {
    val boneColor by skeleton.color
    val cornerRadius = with(LocalDensity.current) { 4.dp.toPx() }
    var coords by remember { mutableStateOf<LayoutCoordinates?>(null) }
    var layoutResult by remember { mutableStateOf<TextLayoutResult?>(null) }

    Layout(
      modifier = modifier
        .onGloballyPositioned { coords = it; uniteGroup.register(it) }
        .graphicsLayer { clip = false }
        .drawWithContent {
          val myCoords = coords ?: return@drawWithContent
          val union = uniteGroup.getUnionBounds(myCoords)
          drawRoundRect(
            color = boneColor,
            topLeft = Offset(union.left, union.top),
            size = Size(union.width, union.height),
            cornerRadius = CornerRadius(cornerRadius),
          )
        },
      measurePolicy = { _, constraints ->
        val result = textMeasurer.measure(
          text = text,
          style = style,
          maxLines = maxLines,
          constraints = constraints,
        )
        layoutResult = result
        layout(result.size.width, result.size.height) {}
      },
    )
  } else {
    val boneColor by skeleton.color
    val cornerRadius = with(LocalDensity.current) { 4.dp.toPx() }
    var layoutResult by remember { mutableStateOf<TextLayoutResult?>(null) }

    Layout(
      modifier = modifier.drawBehind {
        val result = layoutResult ?: return@drawBehind
        for (i in 0 until result.lineCount) {
          drawRoundRect(
            color = boneColor,
            topLeft = Offset(result.getLineLeft(i), result.getLineTop(i)),
            size = Size(
              result.getLineRight(i) - result.getLineLeft(i),
              result.getLineBottom(i) - result.getLineTop(i),
            ),
            cornerRadius = CornerRadius(cornerRadius),
          )
        }
      },
      measurePolicy = { _, constraints ->
        val result = textMeasurer.measure(
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
}
