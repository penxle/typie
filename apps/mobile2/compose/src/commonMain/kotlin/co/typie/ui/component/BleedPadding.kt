package co.typie.ui.component

import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.layout
import androidx.compose.ui.unit.Constraints

fun Modifier.bleedPadding(insets: ScrollFogInsets): Modifier {
  if (insets == ScrollFogInsets()) {
    return this
  }

  return this.layout { measurable, constraints ->
    val leftPx = insets.left.roundToPx()
    val rightPx = insets.right.roundToPx()
    val topPx = insets.top.roundToPx()
    val bottomPx = insets.bottom.roundToPx()

    val horizontalBleed = leftPx + rightPx
    val verticalBleed = topPx + bottomPx
    val childConstraints =
      constraints.copy(
        minWidth = constraints.minWidth + horizontalBleed,
        maxWidth = constraints.maxWidth.plusOrKeepInfinity(horizontalBleed),
        minHeight = constraints.minHeight + verticalBleed,
        maxHeight = constraints.maxHeight.plusOrKeepInfinity(verticalBleed),
      )
    val placeable = measurable.measure(childConstraints)
    val width =
      (placeable.width - horizontalBleed).coerceIn(constraints.minWidth, constraints.maxWidth)
    val height =
      (placeable.height - verticalBleed).coerceIn(constraints.minHeight, constraints.maxHeight)

    layout(width, height) { placeable.place(x = -leftPx, y = -topPx) }
  }
}

@Composable
fun BleedPadding(
  insets: ScrollFogInsets,
  modifier: Modifier = Modifier,
  content: @Composable () -> Unit,
) {
  Box(modifier = modifier.bleedPadding(insets)) { content() }
}

private fun Int.plusOrKeepInfinity(delta: Int): Int {
  if (this == Constraints.Infinity) {
    return this
  }

  return this + delta
}
