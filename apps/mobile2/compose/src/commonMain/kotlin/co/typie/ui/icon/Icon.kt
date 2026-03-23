package co.typie.ui.icon

import androidx.compose.foundation.Image
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.PathParser
import androidx.compose.ui.graphics.vector.rememberVectorPainter
import androidx.compose.ui.unit.dp
import co.typie.ui.skeleton.LocalSkeleton
import co.typie.ui.skeleton.SkeletonBone
import co.typie.ui.theme.AppTheme

@Composable
fun Icon(
  icon: IconData,
  contentDescription: String? = null,
  modifier: Modifier = Modifier,
  strokeWidth: Float = 2f,
  tint: Color = AppTheme.colors.textDefault,
) {
  val skeleton = LocalSkeleton.current
  if (skeleton.enabled) {
    SkeletonBone(modifier)
    return
  }

  val vector = remember(icon, strokeWidth) {
    icon.toImageVector(strokeWidth)
  }

  Image(
    painter = rememberVectorPainter(vector),
    contentDescription = contentDescription,
    modifier = modifier,
    colorFilter = ColorFilter.tint(tint),
  )
}

private fun IconData.toImageVector(strokeWidth: Float): ImageVector =
  ImageVector.Builder(
    defaultWidth = viewportWidth.dp,
    defaultHeight = viewportHeight.dp,
    viewportWidth = viewportWidth,
    viewportHeight = viewportHeight,
  ).apply {
    for (p in paths) {
      addPath(
        pathData = PathParser().parsePathString(p.data).toNodes(),
        fill = if (p.style == PathStyle.Fill) SolidColor(Color.Black) else null,
        stroke = if (p.style == PathStyle.Stroke) SolidColor(Color.Black) else null,
        strokeLineWidth = if (p.style == PathStyle.Stroke) strokeWidth else 0f,
        strokeLineCap = p.strokeLineCap,
        strokeLineJoin = p.strokeLineJoin,
        pathFillType = p.fillType,
      )
    }
  }.build()
