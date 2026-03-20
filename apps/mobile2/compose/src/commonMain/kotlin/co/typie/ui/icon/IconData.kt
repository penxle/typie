package co.typie.ui.icon

import androidx.compose.ui.graphics.PathFillType
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin

enum class PathStyle { Fill, Stroke }

class IconPath(
    val data: String,
    val style: PathStyle,
    val fillType: PathFillType = PathFillType.NonZero,
    val strokeLineCap: StrokeCap = StrokeCap.Butt,
    val strokeLineJoin: StrokeJoin = StrokeJoin.Miter,
)

class IconData(
    val paths: List<IconPath>,
    val viewportWidth: Float = 24f,
    val viewportHeight: Float = 24f,
)
