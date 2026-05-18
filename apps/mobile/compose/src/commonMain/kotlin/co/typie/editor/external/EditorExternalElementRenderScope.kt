package co.typie.editor.external

import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

internal class EditorExternalElementRenderScope(val zoom: Float, val shape: Shape) {
  fun scaledDp(value: Float): Dp = (value * zoom).dp

  fun scaledSp(value: Float): TextUnit = (value * zoom).sp
}
