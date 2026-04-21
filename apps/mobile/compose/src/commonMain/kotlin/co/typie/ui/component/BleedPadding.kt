package co.typie.ui.component

import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.Measurable
import androidx.compose.ui.layout.MeasureResult
import androidx.compose.ui.layout.MeasureScope
import androidx.compose.ui.node.LayoutModifierNode
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.unit.Constraints

fun Modifier.bleedPadding(padding: PaddingValues): Modifier = this then BleedPaddingElement(padding)

private data class BleedPaddingElement(private val padding: PaddingValues) :
  ModifierNodeElement<BleedPaddingNode>() {
  override fun create(): BleedPaddingNode = BleedPaddingNode(padding)

  override fun update(node: BleedPaddingNode) {
    node.padding = padding
  }
}

private class BleedPaddingNode(var padding: PaddingValues) : Modifier.Node(), LayoutModifierNode {
  override fun MeasureScope.measure(
    measurable: Measurable,
    constraints: Constraints,
  ): MeasureResult {
    val leftPx = padding.calculateLeftPadding(layoutDirection).roundToPx()
    val rightPx = padding.calculateRightPadding(layoutDirection).roundToPx()
    val topPx = padding.calculateTopPadding().roundToPx()
    val bottomPx = padding.calculateBottomPadding().roundToPx()

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

    return layout(width, height) { placeable.place(x = -leftPx, y = -topPx) }
  }
}

private fun Int.plusOrKeepInfinity(delta: Int): Int {
  if (this == Constraints.Infinity) {
    return this
  }

  return this + delta
}
